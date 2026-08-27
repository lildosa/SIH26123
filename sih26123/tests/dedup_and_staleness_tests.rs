use sih26123::negotiator::should_yield;
use sih26123::planner::ReservationTable;
use sih26123::protocol::IntentMsg;
use sih26123::world::Pos;

#[test]
fn test_stale_sequence_number_rejection() {
    let mut table = ReservationTable::new(1);
    let path = vec![(Pos::new(0, 0), 1), (Pos::new(1, 0), 2)];

    let intent1 = IntentMsg {
        intent_seq: 10,
        path: path.clone(),
        priority: 1,
    };
    assert!(table.apply_peer_intent(2, &intent1));

    // Stale sequence number (seq 5 <= 10) must be rejected
    let intent_stale = IntentMsg {
        intent_seq: 5,
        path: vec![(Pos::new(2, 0), 3)],
        priority: 1,
    };
    assert!(!table.apply_peer_intent(2, &intent_stale));
    assert_eq!(table.get_peer_intent(2).unwrap().intent_seq, 10);
}

#[test]
fn test_duplicate_sequence_number_rejection() {
    let mut table = ReservationTable::new(1);
    let intent = IntentMsg {
        intent_seq: 42,
        path: vec![(Pos::new(0, 0), 1)],
        priority: 1,
    };
    assert!(table.apply_peer_intent(3, &intent));
    // Exact duplicate seq must return false
    assert!(!table.apply_peer_intent(3, &intent));
}

#[test]
fn test_monotonic_sequence_update() {
    let mut table = ReservationTable::new(1);
    for seq in 1..=5 {
        let intent = IntentMsg {
            intent_seq: seq,
            path: vec![(Pos::new(seq as usize, 0), seq)],
            priority: 1,
        };
        assert!(table.apply_peer_intent(2, &intent));
    }
    assert_eq!(table.get_peer_intent(2).unwrap().intent_seq, 5);
}

#[test]
fn test_out_of_order_intent_interleaving() {
    let mut table = ReservationTable::new(1);
    let msg1 = IntentMsg { intent_seq: 20, path: vec![(Pos::new(1, 1), 1)], priority: 2 };
    let msg2 = IntentMsg { intent_seq: 15, path: vec![(Pos::new(2, 2), 1)], priority: 2 };
    let msg3 = IntentMsg { intent_seq: 25, path: vec![(Pos::new(3, 3), 1)], priority: 2 };

    assert!(table.apply_peer_intent(2, &msg1));
    assert!(!table.apply_peer_intent(2, &msg2), "Stale seq 15 rejected");
    assert!(table.apply_peer_intent(2, &msg3), "Newer seq 25 accepted");
    assert_eq!(table.get_peer_intent(2).unwrap().intent_seq, 25);
}

#[test]
fn test_per_peer_independent_sequence_tracking() {
    let mut table = ReservationTable::new(1);
    let intent_peer2 = IntentMsg { intent_seq: 10, path: vec![(Pos::new(0, 0), 1)], priority: 2 };
    let intent_peer3 = IntentMsg { intent_seq: 5, path: vec![(Pos::new(1, 1), 1)], priority: 3 };

    assert!(table.apply_peer_intent(2, &intent_peer2));
    assert!(table.apply_peer_intent(3, &intent_peer3));

    assert_eq!(table.get_peer_intent(2).unwrap().intent_seq, 10);
    assert_eq!(table.get_peer_intent(3).unwrap().intent_seq, 5);
}

#[test]
fn test_conflict_arbitration_consistent_under_staleness() {
    // Robot 1 (prio 1) vs Robot 2 (prio 2)
    assert!(!should_yield(1, 1, 2, 2));
    assert!(should_yield(2, 2, 1, 1));
}
