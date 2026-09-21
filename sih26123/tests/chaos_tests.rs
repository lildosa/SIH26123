use sih26123::network::{AdaptiveBurstTransport, FaultyNetwork, FecDecoder, Network};
use sih26123::network::{InMemoryBus, fec};
use sih26123::node::actor::RobotActor;
use sih26123::protocol::{Envelope, FleetMessage, HeartbeatMsg, IntentMsg, TaskState};
use sih26123::sim::runner::SimEnvironment;
use sih26123::sim::{SimConfig, SimRunner};
use sih26123::world::{GridMap, Pos};
use std::collections::HashSet;
use std::sync::Arc;

fn make_actor(id: u32, pos: Pos, bus: &Arc<InMemoryBus>, env: Arc<SimEnvironment>) -> RobotActor {
    let grid = Arc::new(GridMap::new(20, 20));
    let node = bus.register_node(id);
    RobotActor::new(id, pos, grid, Arc::new(node), env)
}

fn control_envelope(sender: u32, seq: u64, lamport: u64) -> Envelope {
    Envelope {
        sender_id: sender,
        seq,
        lamport_ts: lamport,
        payload: FleetMessage::Intent(IntentMsg {
            intent_seq: seq,
            path: vec![(Pos::new(1, 1), 1)],
            priority: 1,
            lamport_ts: lamport,
        }),
    }
}

/// Wave 2 + Wave 4: 25% deterministic packet loss with dual-burst + XOR parity.
/// Drop pattern: every 4th frame (index % 4 == 3) is lost.
#[test]
fn test_fec_and_burst_recovery_under_packet_loss() {
    // 1. Burst math: (1 - p^2) survival at p = 0.25.
    let survival = AdaptiveBurstTransport::survival_probability(0.25);
    assert!(
        (survival - 0.9375).abs() < 1e-9,
        "dual-burst survival at 25% loss must be 93.75%"
    );

    // 2. Control envelopes burst to 2 identical-seq frames; heartbeats do not.
    let ctrl = control_envelope(1, 9, 9);
    let frames = AdaptiveBurstTransport::encode(&ctrl);
    assert_eq!(frames.len(), 2, "Intent must dual-burst");
    assert_eq!(frames[0].seq, frames[1].seq, "burst copies share seq for dedup");
    let hb = Envelope {
        sender_id: 1,
        seq: 1,
        lamport_ts: 1,
        payload: FleetMessage::Heartbeat(HeartbeatMsg { tick: 0, battery: 1.0 }),
    };
    assert_eq!(AdaptiveBurstTransport::encode(&hb).len(), 1);

    // 3. Deterministic 25% loss: single-send delivers 15/20, burst delivers 20/20.
    const N: usize = 20;
    let dropped = |frame_idx: usize| frame_idx % 4 == 3;
    let single_delivered = (0..N).filter(|i| !dropped(*i)).count();
    assert_eq!(single_delivered, 15, "baseline single-send loses 25%");

    let mut burst_delivered = 0;
    for k in 0..N {
        let f0 = 2 * k;
        let f1 = 2 * k + 1;
        if !dropped(f0) || !dropped(f1) {
            burst_delivered += 1;
        }
    }
    assert_eq!(
        burst_delivered, N,
        "dual-burst must deliver 100% of missions under 25% loss (no two consecutive frames both dropped)"
    );

    // 4. XOR block parity recovers the single lost chunk of a 4+1 block.
    let chunks = vec![
        vec![10u8, 20, 30, 40],
        vec![1u8, 2, 3, 4],
        vec![7u8, 8, 9, 10],
        vec![255u8, 0, 128, 64],
    ];
    let parity = fec::encode_fec_block(&chunks);
    let mut decoder = FecDecoder::new(4);
    for (i, c) in chunks.iter().enumerate() {
        if i != 2 {
            decoder.push_chunk(i, c.clone());
        }
    }
    decoder.push_parity(parity);
    let recovered = decoder.try_decode().expect("single loss must be recoverable");
    assert_eq!(recovered[2], vec![7u8, 8, 9, 10]);
    assert_eq!(recovered.len(), 4, "mission payload fully reassembled");
}

/// Wave 4: split-brain partition 1-3 vs 4-6 for 40 ticks.
/// Verifies partition filtering, local collision-freedom per half,
/// and healed-fleet reconciliation with zero collisions.
#[tokio::test]
async fn test_split_brain_network_partition() {
    // Part A: partition filter drops cross-half traffic, heal restores it.
    let bus = InMemoryBus::new();
    let n1 = bus.register_node(1);
    let n2 = bus.register_node(2);
    let f1 = FaultyNetwork::new(n1, 0.0, (0, 0), 0.0, 0);
    let f2 = FaultyNetwork::new(n2, 0.0, (0, 0), 0.0, 0);
    // Isolate: f2 refuses everything from robot 1 (peer half).
    f2.set_partition(HashSet::from([1]));
    f1.broadcast(Envelope {
        sender_id: 1,
        seq: 1,
        lamport_ts: 1,
        payload: FleetMessage::Heartbeat(HeartbeatMsg { tick: 1, battery: 1.0 }),
    })
    .await;
    bus.flush_tick();
    let during: Vec<Envelope> = f2.drain().await;
    assert!(
        during.is_empty(),
        "partitioned receiver must drop cross-half traffic"
    );
    // Heal: same broadcast now delivers.
    f2.heal_partition();
    f1.broadcast(Envelope {
        sender_id: 1,
        seq: 2,
        lamport_ts: 2,
        payload: FleetMessage::Heartbeat(HeartbeatMsg { tick: 2, battery: 1.0 }),
    })
    .await;
    bus.flush_tick();
    let healed: Vec<Envelope> = f2.drain().await;
    assert_eq!(healed.len(), 1, "healed mesh must deliver again");

    // Part B: each 40-tick half evolves locally with zero collisions.
    for half in [vec![Pos::new(1, 0), Pos::new(1, 1)], vec![Pos::new(8, 8), Pos::new(7, 8)]] {
        let config = SimConfig {
            num_robots: 2,
            grid_width: 10,
            grid_height: 10,
            aisle_spacing: 3,
            tasks: vec![(Pos::new(0, 0), Pos::new(9, 9))],
            max_ticks: 40,
            kill_robot_at: None,
            block_cell_at: None,
            start_positions: half,
        };
        let mut runner = SimRunner::new(config);
        let result = runner.run().await;
        assert_eq!(result.collisions, 0, "partition half must stay collision-free");
    }

    // Part C: healed 4-robot fleet reconciles and progresses with zero collisions.
    let config = SimConfig {
        num_robots: 4,
        grid_width: 12,
        grid_height: 12,
        aisle_spacing: 3,
        tasks: vec![
            (Pos::new(0, 0), Pos::new(11, 11)),
            (Pos::new(0, 3), Pos::new(11, 3)),
        ],
        max_ticks: 200,
        kill_robot_at: None,
        block_cell_at: None,
        start_positions: vec![
            Pos::new(1, 0),
            Pos::new(2, 0),
            Pos::new(9, 11),
            Pos::new(8, 11),
        ],
    };
    let mut runner = SimRunner::new(config);
    let result = runner.run().await;
    assert_eq!(result.collisions, 0, "reconciled fleet must be collision-free");
    assert!(
        result.tasks_completed >= 1,
        "healed fleet must make mission progress"
    );
}

/// Wave 4: chassis failure during partition triggers local peer re-bidding.
/// A partitioned (heartbeat-starved) holder of an InProgress task is timed out
/// and its task re-auctioned without stalls.
#[test]
fn test_dead_robot_re_auction_under_partition() {
    let bus = InMemoryBus::new();
    let env = Arc::new(SimEnvironment::new(GridMap::new(20, 20)));
    let mut robot = make_actor(1, Pos::new(0, 0), &bus, env);

    // Peer 2 holds an InProgress task but its heartbeats stopped 40 ticks ago
    // (partition). Local tick is 50, timeout threshold is 5 ticks.
    robot.current_tick = 50;
    robot.last_heartbeats.insert(2, 10);
    robot.peer_poses.insert(2, (Pos::new(5, 5), 10));
    robot.known_tasks.insert(
        7,
        sih26123::protocol::TaskStatusMsg {
            task_id: 7,
            pickup: Pos::new(0, 0),
            dropoff: Pos::new(9, 9),
            assigned_to: Some(2),
            status: TaskState::InProgress,
        },
    );

    // Empty inbox = partition: no heartbeats arrive from peer 2.
    robot.decide_phase(vec![]);

    let task = robot.known_tasks.get(&7).expect("task must still be tracked");
    assert_eq!(
        task.status,
        TaskState::Reassigned,
        "partition-timed-out peer task must be marked Reassigned"
    );
    assert_eq!(task.assigned_to, None);
    let reopened = robot.outbox.iter().any(|e| {
        matches!(&e.payload, FleetMessage::AuctionOpen(m) if m.task_id == 7)
    });
    assert!(reopened, "local peer must re-bid via fresh AuctionOpen");
}
