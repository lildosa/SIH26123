use sih26123::negotiator::should_yield_lamport;
use sih26123::node::actor::RobotActor;
use sih26123::network::InMemoryBus;
use sih26123::protocol::{ConflictMsg, Envelope, FleetMessage, HeartbeatMsg, IntentMsg};
use sih26123::sim::runner::SimEnvironment;
use sih26123::world::{GridMap, Pos};
use std::sync::Arc;

fn make_actor(id: u32, pos: Pos, bus: &Arc<InMemoryBus>, env: Arc<SimEnvironment>) -> RobotActor {
    let grid = Arc::new(GridMap::new(20, 20));
    let node = bus.register_node(id);
    RobotActor::new(id, pos, grid, Arc::new(node), env)
}

#[test]
fn test_lamport_clock_increments_on_send() {
    let bus = InMemoryBus::new();
    let env = Arc::new(SimEnvironment::new(GridMap::new(20, 20)));
    let mut robot = make_actor(1, Pos::new(0, 0), &bus, env);

    assert_eq!(robot.lamport_clock, 0);
    robot.send(FleetMessage::Heartbeat(HeartbeatMsg { tick: 0, battery: 1.0 }));
    assert_eq!(robot.lamport_clock, 1, "Send must increment local Lamport clock");

    robot.send(FleetMessage::Heartbeat(HeartbeatMsg { tick: 0, battery: 1.0 }));
    robot.send(FleetMessage::Heartbeat(HeartbeatMsg { tick: 0, battery: 1.0 }));
    assert_eq!(robot.lamport_clock, 3, "Each send increments by exactly 1");

    let outbox = &robot.outbox;
    assert_eq!(outbox.len(), 3);
    assert_eq!(outbox[0].lamport_ts, 1);
    assert_eq!(outbox[1].lamport_ts, 2);
    assert_eq!(outbox[2].lamport_ts, 3);
    assert_eq!(outbox[0].lamport_ts, robot.lamport_clock - 2, "Envelope stamps must be monotonically increasing");
}

#[test]
fn test_lamport_clock_merges_on_receive() {
    let bus = InMemoryBus::new();
    let env = Arc::new(SimEnvironment::new(GridMap::new(20, 20)));
    let mut robot = make_actor(1, Pos::new(0, 0), &bus, env);

    let incoming = Envelope {
        sender_id: 2,
        seq: 1,
        lamport_ts: 7,
        payload: FleetMessage::Heartbeat(HeartbeatMsg { tick: 0, battery: 1.0 }),
    };

    robot.decide_phase(vec![incoming]);

    assert_eq!(
        robot.lamport_clock, 10,
        "Receive merges to 8, then heartbeat and pose each increment the clock"
    );

    let reply = &robot.outbox[0];
    assert_eq!(
        reply.lamport_ts, 9,
        "First reply increments the merged clock"
    );
}

#[test]
fn test_lamport_clock_local_events_overtake_incoming() {
    let bus = InMemoryBus::new();
    let env = Arc::new(SimEnvironment::new(GridMap::new(20, 20)));
    let mut robot = make_actor(1, Pos::new(0, 0), &bus, env);

    robot.decide_phase(vec![Envelope {
        sender_id: 2,
        seq: 1,
        lamport_ts: 2,
        payload: FleetMessage::Heartbeat(HeartbeatMsg { tick: 0, battery: 1.0 }),
    }]);
    assert_eq!(robot.lamport_clock, 5);

    let mut fast_peer_mailbox = Vec::new();
    for seq in 1..=4 {
        fast_peer_mailbox.push(Envelope {
            sender_id: 3,
            seq,
            lamport_ts: 2,
            payload: FleetMessage::Heartbeat(HeartbeatMsg { tick: 0, battery: 1.0 }),
        });
    }
    robot.decide_phase(fast_peer_mailbox);
    assert_eq!(
        robot.lamport_clock, 11,
        "Four receives and two sends advance the existing clock"
    );
}

#[test]
fn test_lamport_intent_stamp_recorded_on_send() {
    let bus = InMemoryBus::new();
    let env = Arc::new(SimEnvironment::new(GridMap::new(20, 20)));
    let mut robot = make_actor(1, Pos::new(0, 0), &bus, env);
    let pickup = robot.pos;
    let dropoff = Pos::new(3, 0);
    robot.assigned_task = Some((1, pickup, dropoff));
    robot.state = sih26123::node::state::RobotState::Planning {
        task_id: 1,
        pickup,
        dropoff,
    };

    robot.decide_phase(vec![]);

    let intent_env = robot
        .outbox
        .iter()
        .find(|e| matches!(e.payload, FleetMessage::Intent(_)))
        .expect("Idle actor with open task must emit Intent");
    match &intent_env.payload {
        FleetMessage::Intent(m) => {
            assert_eq!(m.lamport_ts, robot.current_intent_lamport);
            assert!(m.lamport_ts > 0, "Intent must carry a positive Lamport stamp");
        }
        _ => unreachable!(),
    }
}

#[test]
fn test_intent_lamport_propagates_to_reservation_record() {
    let mut table = sih26123::planner::ReservationTable::new(1);
    let applied = table.apply_peer_intent(
        2,
        &IntentMsg {
            intent_seq: 1,
            path: vec![(Pos::new(1, 1), 1)],
            priority: 2,
            lamport_ts: 42,
        },
    );
    assert!(applied);
    assert_eq!(table.get_peer_intent(2).unwrap().lamport_ts, 42);
}

#[test]
fn test_lamport_higher_priority_wins() {
    assert!(!should_yield_lamport(5, 1, 100, 9, 2, 999), "Priority 1 must beat priority 2 regardless of lamport");
    assert!(should_yield_lamport(5, 2, 999, 9, 1, 1), "Lower priority must yield to higher priority regardless of lamport");
}

#[test]
fn test_lamport_equal_priority_older_timestamp_wins() {
    assert!(!should_yield_lamport(5, 1, 3, 9, 1, 7), "Older (smaller) lamport wins on equal priority");
    assert!(should_yield_lamport(5, 1, 7, 9, 1, 3), "Newer (larger) lamport yields on equal priority");
}

#[test]
fn test_lamport_equal_priority_and_timestamp_lower_id_wins() {
    assert!(!should_yield_lamport(5, 1, 4, 9, 1, 4), "Lower id wins on equal priority and lamport");
    assert!(should_yield_lamport(9, 1, 4, 5, 1, 4), "Higher id yields on equal priority and lamport");
}

#[test]
fn test_lamport_arbitration_full_ordering() {
    assert!(!should_yield_lamport(1, 1, 5, 2, 1, 5), "Self case: identical everything resolves by id");
    assert!(should_yield_lamport(2, 1, 5, 1, 1, 5), "Id 2 yields to id 1 when tied");
    assert!(!should_yield_lamport(2, 1, 1, 1, 1, 9), "Older lamport beats id ordering");
    assert!(should_yield_lamport(1, 2, 0, 2, 1, 999), "Priority dominates lamport and id");
}

#[test]
fn test_legacy_should_yield_semantics_unchanged() {
    assert!(!sih26123::negotiator::should_yield(1, 1, 2, 2));
    assert!(sih26123::negotiator::should_yield(2, 2, 1, 1));
    assert!(sih26123::negotiator::should_yield(2, 1, 1, 1));
    assert!(sih26123::negotiator::should_yield(1, 2, 2, 1));
}

#[test]
fn test_actor_equal_priority_older_lamport_wins_despite_larger_id() {
    let bus = InMemoryBus::new();
    let env = Arc::new(SimEnvironment::new(GridMap::new(20, 20)));
    // Actor 9 (larger id) holds the OLDER intent (lamport 3).
    let mut robot = make_actor(9, Pos::new(0, 0), &bus, env);
    robot.current_intent_seq = 5;
    robot.current_intent_priority = 1;
    robot.current_intent_lamport = 3;
    robot.assigned_task = Some((1, Pos::new(0, 0), Pos::new(3, 0)));
    robot.state = sih26123::node::state::RobotState::Moving {
        path: vec![(Pos::new(0, 0), 1), (Pos::new(1, 0), 2)],
        step_index: 0,
    };
    // Peer 1 (smaller id) holds a NEWER intent (lamport 8).
    robot.local_reservations.apply_peer_intent(
        1,
        &IntentMsg {
            intent_seq: 7,
            path: vec![(Pos::new(5, 5), 9)],
            priority: 1,
            lamport_ts: 8,
        },
    );
    let outbox_before = robot.outbox.len();
    robot.decide_phase(vec![Envelope {
        sender_id: 1,
        seq: 1,
        lamport_ts: 8,
        payload: FleetMessage::Conflict(ConflictMsg {
            challenger_intent_seq: 7,
            challenger_priority: 1,
            challenger_lamport: 8,
            challenged_id: 9,
            challenged_intent_seq: 5,
            conflicting_cell: Pos::new(1, 0),
            conflicting_tick: 2,
        }),
    }]);
    // Older lamport must win despite larger robot id: no Yield to the challenger.
    let yielded = robot.outbox[outbox_before..].iter().any(|e| {
        matches!(&e.payload, FleetMessage::Yield(m) if m.to_robot == 1 && m.yielded_intent_seq == 5)
    });
    assert!(!yielded, "Older Lamport (3) must beat newer (8) despite larger id 9 > 1");

    // Reverse: actor 9 now holds NEWER intent, peer holds OLDER -> must yield.
    let bus2 = InMemoryBus::new();
    let env2 = Arc::new(SimEnvironment::new(GridMap::new(20, 20)));
    let mut robot2 = make_actor(9, Pos::new(0, 0), &bus2, env2);
    robot2.current_intent_seq = 5;
    robot2.current_intent_priority = 1;
    robot2.current_intent_lamport = 8;
    robot2.assigned_task = Some((1, Pos::new(0, 0), Pos::new(3, 0)));
    robot2.state = sih26123::node::state::RobotState::Moving {
        path: vec![(Pos::new(0, 0), 1), (Pos::new(1, 0), 2)],
        step_index: 0,
    };
    robot2.local_reservations.apply_peer_intent(
        1,
        &IntentMsg {
            intent_seq: 7,
            path: vec![(Pos::new(5, 5), 9)],
            priority: 1,
            lamport_ts: 3,
        },
    );
    let before2 = robot2.outbox.len();
    robot2.decide_phase(vec![Envelope {
        sender_id: 1,
        seq: 1,
        lamport_ts: 3,
        payload: FleetMessage::Conflict(ConflictMsg {
            challenger_intent_seq: 7,
            challenger_priority: 1,
            challenger_lamport: 3,
            challenged_id: 9,
            challenged_intent_seq: 5,
            conflicting_cell: Pos::new(1, 0),
            conflicting_tick: 2,
        }),
    }]);
    let yielded2 = robot2.outbox[before2..].iter().any(|e| {
        matches!(&e.payload, FleetMessage::Yield(m) if m.to_robot == 1 && m.yielded_intent_seq == 5)
    });
    assert!(yielded2, "Newer Lamport (8) must yield to older (3) even with equal priority");
}

#[test]
fn test_conflict_unknown_version_yields_conservatively() {
    let bus = InMemoryBus::new();
    let env = Arc::new(SimEnvironment::new(GridMap::new(20, 20)));
    let mut robot = make_actor(2, Pos::new(0, 0), &bus, env);
    robot.current_intent_seq = 4;
    robot.current_intent_priority = 1;
    robot.current_intent_lamport = 10;
    robot.assigned_task = Some((1, Pos::new(0, 0), Pos::new(2, 0)));
    robot.state = sih26123::node::state::RobotState::Moving {
        path: vec![(Pos::new(0, 0), 1)],
        step_index: 0,
    };
    // No known peer intent + legacy zero stamp -> must yield safely.
    let before = robot.outbox.len();
    robot.decide_phase(vec![Envelope {
        sender_id: 3,
        seq: 1,
        lamport_ts: 0,
        payload: FleetMessage::Conflict(ConflictMsg {
            challenger_intent_seq: 99,
            challenger_priority: 1,
            challenger_lamport: 0,
            challenged_id: 2,
            challenged_intent_seq: 4,
            conflicting_cell: Pos::new(0, 0),
            conflicting_tick: 1,
        }),
    }]);
    let yielded = robot.outbox[before..]
        .iter()
        .any(|e| matches!(&e.payload, FleetMessage::Yield(_)));
    assert!(yielded, "Unknown challenger version with legacy stamp must yield conservatively");
}

#[test]
fn test_legacy_json_defaults_for_lamport_fields() {
    // Envelopes / intents / conflicts serialized before Wave 1 must still parse.
    let env_json = r#"{"sender_id":1,"seq":2,"payload":{"Heartbeat":{"tick":0,"battery":1.0}}}"#;
    let env: Envelope = serde_json::from_str(env_json).expect("legacy Envelope must parse");
    assert_eq!(env.lamport_ts, 0);

    let intent_json = r#"{"intent_seq":1,"path":[],"priority":1}"#;
    let intent: IntentMsg = serde_json::from_str(intent_json).expect("legacy IntentMsg must parse");
    assert_eq!(intent.lamport_ts, 0);

    let conflict_json = r#"{"challenger_intent_seq":1,"challenger_priority":1,"challenged_id":2,"challenged_intent_seq":3,"conflicting_cell":{"x":0,"y":0},"conflicting_tick":1}"#;
    let conflict: ConflictMsg =
        serde_json::from_str(conflict_json).expect("legacy ConflictMsg must parse");
    assert_eq!(conflict.challenger_lamport, 0);

    let pose_json = r#"{"pos":{"x":0,"y":0},"tick":0,"battery":1.0,"status":"Idle"}"#;
    let pose: sih26123::protocol::PoseMsg =
        serde_json::from_str(pose_json).expect("legacy PoseMsg must parse");
    assert_eq!(pose.orientation, None);
}

#[test]
fn test_duplicate_lamport_clocks_tiebreak_by_id_at_actor_level() {
    // Same priority AND same lamport -> lower id wins, exercised through Conflict path.
    let bus = InMemoryBus::new();
    let env = Arc::new(SimEnvironment::new(GridMap::new(20, 20)));
    let mut robot = make_actor(7, Pos::new(0, 0), &bus, env);
    robot.current_intent_seq = 4;
    robot.current_intent_priority = 1;
    robot.current_intent_lamport = 5;
    robot.assigned_task = Some((1, Pos::new(0, 0), Pos::new(2, 0)));
    robot.state = sih26123::node::state::RobotState::Moving {
        path: vec![(Pos::new(0, 0), 1)],
        step_index: 0,
    };
    robot.local_reservations.apply_peer_intent(
        3,
        &IntentMsg {
            intent_seq: 6,
            path: vec![(Pos::new(4, 4), 8)],
            priority: 1,
            lamport_ts: 5,
        },
    );
    let before = robot.outbox.len();
    robot.decide_phase(vec![Envelope {
        sender_id: 3,
        seq: 1,
        lamport_ts: 5,
        payload: FleetMessage::Conflict(ConflictMsg {
            challenger_intent_seq: 6,
            challenger_priority: 1,
            challenger_lamport: 5,
            challenged_id: 7,
            challenged_intent_seq: 4,
            conflicting_cell: Pos::new(0, 0),
            conflicting_tick: 1,
        }),
    }]);
    let yielded = robot.outbox[before..]
        .iter()
        .any(|e| matches!(&e.payload, FleetMessage::Yield(_)));
    assert!(yielded, "Equal lamport must fall back to id: 7 yields to 3");
}
