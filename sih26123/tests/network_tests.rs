use sih26123::network::{FaultyNetwork, InMemoryBus, Network};
use sih26123::protocol::{Envelope, FleetMessage, HeartbeatMsg};

#[tokio::test]
async fn test_in_memory_tick_scoped_delivery() {
    let bus = InMemoryBus::new();
    let node1 = bus.register_node(1);
    let node2 = bus.register_node(2);

    let msg = Envelope {
        sender_id: 1,
        seq: 1,
        payload: FleetMessage::Heartbeat(HeartbeatMsg {
            tick: 0,
            battery: 1.0,
        }),
    };

    // Node 1 broadcasts
    node1.broadcast(msg.clone()).await;

    // Before flush, Node 2's inbox must be empty (tick-scoped staging)
    assert!(node2.drain().await.is_empty(), "Message should be staged, not delivered before flush");

    // Flush tick
    bus.flush_tick();

    // After flush, Node 2 should have the message
    let received = node2.drain().await;
    assert_eq!(received.len(), 1);
    assert_eq!(received[0].sender_id, 1);
    assert_eq!(received[0].seq, 1);
}

#[tokio::test]
async fn test_in_memory_no_self_echo() {
    let bus = InMemoryBus::new();
    let node1 = bus.register_node(1);

    node1.broadcast(Envelope {
        sender_id: 1,
        seq: 1,
        payload: FleetMessage::Heartbeat(HeartbeatMsg { tick: 0, battery: 1.0 }),
    }).await;

    bus.flush_tick();

    // Node 1 must not receive its own broadcast
    assert!(node1.drain().await.is_empty(), "Sender should not receive its own echo");
}

#[tokio::test]
async fn test_faulty_network_drop() {
    let bus = InMemoryBus::new();
    let node1 = bus.register_node(1);
    let node2 = bus.register_node(2);

    let faulty_node1 = FaultyNetwork::new(node1, 1.0, (0, 0), 0.0, 0); // 100% drop

    faulty_node1.broadcast(Envelope {
        sender_id: 1,
        seq: 1,
        payload: FleetMessage::Heartbeat(HeartbeatMsg { tick: 0, battery: 1.0 }),
    }).await;

    bus.flush_tick();

    assert!(node2.drain().await.is_empty(), "100% drop rate should deliver 0 messages");
}

#[tokio::test]
async fn test_faulty_network_duplicate() {
    let bus = InMemoryBus::new();
    let node1 = bus.register_node(1);
    let node2 = bus.register_node(2);

    let faulty_node1 = FaultyNetwork::new(node1, 0.0, (0, 0), 1.0, 0); // 100% duplicate

    faulty_node1.broadcast(Envelope {
        sender_id: 1,
        seq: 1,
        payload: FleetMessage::Heartbeat(HeartbeatMsg { tick: 0, battery: 1.0 }),
    }).await;

    bus.flush_tick();

    let received = node2.drain().await;
    assert_eq!(received.len(), 2, "100% duplicate rate should deliver 2 identical envelopes");
}
