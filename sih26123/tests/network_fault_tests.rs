use sih26123::network::faulty::FaultyNetwork;
use sih26123::network::in_memory::InMemoryBus;
use sih26123::network::Network;
use sih26123::protocol::{Envelope, FleetMessage, HeartbeatMsg};

#[tokio::test]
async fn test_faulty_transport_packet_drop_simulation() {
    let bus = InMemoryBus::new();
    let node1 = bus.register_node(1);
    let node2 = bus.register_node(2);

    let faulty1 = FaultyNetwork::new(node1, 1.0, (0, 0), 0.0, 0);

    faulty1
        .broadcast(Envelope {
            sender_id: 1,
            seq: 1,
            payload: FleetMessage::Heartbeat(HeartbeatMsg { tick: 1, battery: 1.0 }),
        })
        .await;

    bus.flush_tick();
    let inbox2 = node2.drain().await;
    assert_eq!(inbox2.len(), 0, "100% drop must drop all packets");
}

#[tokio::test]
async fn test_faulty_transport_packet_duplication() {
    let bus = InMemoryBus::new();
    let node1 = bus.register_node(1);
    let node2 = bus.register_node(2);

    let faulty1 = FaultyNetwork::new(node1, 0.0, (0, 0), 1.0, 0);

    faulty1
        .broadcast(Envelope {
            sender_id: 1,
            seq: 1,
            payload: FleetMessage::Heartbeat(HeartbeatMsg { tick: 1, battery: 1.0 }),
        })
        .await;

    bus.flush_tick();
    let inbox2 = node2.drain().await;
    assert_eq!(inbox2.len(), 2, "100% duplication must result in 2 messages in inbox");
}

#[tokio::test]
async fn test_faulty_transport_latency_staged_delivery() {
    let bus = InMemoryBus::new();
    let node1 = bus.register_node(1);
    let node2 = bus.register_node(2);

    let faulty1 = FaultyNetwork::new(node1, 0.0, (1, 5), 0.0, 0);

    faulty1
        .broadcast(Envelope {
            sender_id: 1,
            seq: 1,
            payload: FleetMessage::Heartbeat(HeartbeatMsg { tick: 1, battery: 1.0 }),
        })
        .await;

    // Before flush: empty
    assert_eq!(node2.drain().await.len(), 0);
    // After flush: delivered
    bus.flush_tick();
    assert_eq!(node2.drain().await.len(), 1);
}

#[tokio::test]
async fn test_zero_loss_clean_channel() {
    let bus = InMemoryBus::new();
    let node1 = bus.register_node(1);
    let node2 = bus.register_node(2);

    let faulty1 = FaultyNetwork::new(node1, 0.0, (0, 0), 0.0, 0);

    for seq in 1..=5 {
        faulty1
            .broadcast(Envelope {
                sender_id: 1,
                seq,
                payload: FleetMessage::Heartbeat(HeartbeatMsg { tick: seq, battery: 1.0 }),
            })
            .await;
    }

    bus.flush_tick();
    let inbox2 = node2.drain().await;
    assert_eq!(inbox2.len(), 5, "Zero loss delivers exactly 5 envelopes");
}
