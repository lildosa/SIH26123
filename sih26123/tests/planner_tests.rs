use sih26123::planner::{plan, ConflictType, ReservationTable, SpaceTimeConstraints};
use sih26123::protocol::IntentMsg;
use sih26123::world::{Cell, GridMap, Pos};

#[test]
fn test_single_robot_finds_path() {
    let grid = GridMap::new(10, 10);
    let constraints = SpaceTimeConstraints::default();
    let path = plan(&grid, 1, Pos::new(0, 0), 0, Pos::new(9, 9), &constraints, 200);

    assert!(path.is_some(), "Path should be found on empty grid");
    let path = path.unwrap();
    assert_eq!(path.len(), 19, "Path length should be Manhattan distance 18 + 1 = 19");
    assert_eq!(path.first().unwrap(), &(Pos::new(0, 0), 0));
    assert_eq!(path.last().unwrap().0, Pos::new(9, 9));
}

#[test]
fn test_two_robots_no_collision() {
    let grid = GridMap::new(10, 10);

    // Robot 1 plans (0,0) -> (9,0)
    let constraints1 = SpaceTimeConstraints::default();
    let path1 = plan(&grid, 1, Pos::new(0, 0), 0, Pos::new(9, 0), &constraints1, 200).unwrap();

    // Robot 2 gets Robot 1's path as peer intent
    let mut table2 = ReservationTable::new(2);
    table2.apply_peer_intent(
        1,
        &IntentMsg {
            intent_seq: 1,
            path: path1.clone(),
            priority: 1,
            lamport_ts: 0,
        },
    );
    let constraints2 = table2.build_constraints();

    // Robot 2 plans (5,0) -> (0,0) starting at tick 0 - wait, R1 is at (0,0) at t=0, R2 at (5,0) at t=0
    let path2 = plan(&grid, 2, Pos::new(5, 5), 0, Pos::new(0, 0), &constraints2, 200).unwrap();

    // Verify no (Pos, Tick) collision
    for (p1, t1) in &path1 {
        for (p2, t2) in &path2 {
            assert!(
                !(p1 == p2 && t1 == t2),
                "Collision detected at {:?} tick {}",
                p1,
                t1
            );
        }
    }
}

#[test]
fn test_robot_waits_at_bottleneck() {
    let mut grid = GridMap::new(5, 3);
    // Row 0: free
    // Row 1: walls except (2,1) is the bottleneck
    // Row 2: free
    for x in 0..5 {
        if x != 2 {
            grid.set_cell(Pos::new(x, 1), Cell::Wall);
        }
    }

    // Robot 1: starts at (0,0) going to (4,2). Must pass through (2,1).
    let constraints1 = SpaceTimeConstraints::default();
    let path1 = plan(&grid, 1, Pos::new(0, 0), 0, Pos::new(4, 2), &constraints1, 200).unwrap();

    // Robot 2: starts at (0,2) going to (4,0). Must also pass through (2,1).
    let mut table2 = ReservationTable::new(2);
    table2.apply_peer_intent(
        1,
        &IntentMsg {
            intent_seq: 1,
            path: path1.clone(),
            priority: 1,
            lamport_ts: 0,
        },
    );
    let constraints2 = table2.build_constraints();

    let path2 = plan(&grid, 2, Pos::new(0, 2), 0, Pos::new(4, 0), &constraints2, 200).unwrap();

    // Robot 2 must wait at bottleneck or take an alternative step
    assert!(
        path2.len() >= path1.len(),
        "Robot 2 path length ({}) should be >= Robot 1 ({})",
        path2.len(),
        path1.len()
    );

    // Verify zero collisions
    for (p1, t1) in &path1 {
        for (p2, t2) in &path2 {
            assert!(
                !(p1 == p2 && t1 == t2),
                "Collision at bottleneck {:?} tick {}",
                p1,
                t1
            );
        }
    }
}

#[test]
fn test_no_head_on_swap() {
    let mut grid = GridMap::new(3, 3);
    // 3x1 corridor at y=1
    for x in 0..3 {
        grid.set_cell(Pos::new(x, 0), Cell::Wall);
        grid.set_cell(Pos::new(x, 2), Cell::Wall);
    }

    // Robot 1: (0,1) -> (2,1)
    let path1 = plan(
        &grid,
        1,
        Pos::new(0, 1),
        0,
        Pos::new(2, 1),
        &SpaceTimeConstraints::default(),
        50,
    )
    .unwrap();

    // Robot 2: (2,1) -> (0,1) with Robot 1's path
    let mut table2 = ReservationTable::new(2);
    table2.apply_peer_intent(
        1,
        &IntentMsg {
            intent_seq: 1,
            path: path1.clone(),
            priority: 1,
            lamport_ts: 0,
        },
    );
    let constraints2 = table2.build_constraints();

    let path2 = plan(&grid, 2, Pos::new(2, 1), 0, Pos::new(0, 1), &constraints2, 20);

    if let Some(p2) = path2 {
        // Verify no head-on swap
        for w1 in path1.windows(2) {
            for w2 in p2.windows(2) {
                let (from1, t1) = w1[0];
                let (to1, _) = w1[1];
                let (from2, t2) = w2[0];
                let (to2, _) = w2[1];
                if t1 == t2 {
                    assert!(
                        !(from1 == to2 && to1 == from2),
                        "Head-on swap detected at tick {} between {:?} and {:?}",
                        t1,
                        from1,
                        to1
                    );
                }
            }
        }
    }
}

#[test]
fn test_replan_after_obstacle() {
    let mut grid = GridMap::new(10, 10);
    let constraints = SpaceTimeConstraints::default();

    // Initial plan (0,0) -> (9,0)
    let path1 = plan(&grid, 1, Pos::new(0, 0), 0, Pos::new(9, 0), &constraints, 200).unwrap();
    assert!(path1.iter().any(|(p, _)| *p == Pos::new(5, 0)));

    // Now (5,0) becomes a Wall
    grid.set_cell(Pos::new(5, 0), Cell::Wall);

    // Replan from current pos (e.g. at (4,0), tick 4)
    let path2 = plan(&grid, 1, Pos::new(4, 0), 4, Pos::new(9, 0), &constraints, 200).unwrap();

    assert!(
        !path2.iter().any(|(p, _)| *p == Pos::new(5, 0)),
        "Replanned path must avoid the dynamic obstacle at (5,0)"
    );
    assert_eq!(path2.last().unwrap().0, Pos::new(9, 0));
}

#[test]
fn test_stale_peer_intent_rejected() {
    let mut table = ReservationTable::new(1);
    let intent_v5 = IntentMsg {
        intent_seq: 5,
        path: vec![(Pos::new(1, 1), 1)],
        priority: 2,
        lamport_ts: 0,
    };
    let intent_v3 = IntentMsg {
        intent_seq: 3,
        path: vec![(Pos::new(2, 2), 1)],
        priority: 2,
        lamport_ts: 0,
    };

    assert!(table.apply_peer_intent(2, &intent_v5));
    assert_eq!(table.get_peer_intent(2).unwrap().intent_seq, 5);

    // Applying older seq 3 must be rejected
    assert!(!table.apply_peer_intent(2, &intent_v3));
    assert_eq!(table.get_peer_intent(2).unwrap().intent_seq, 5);
}

#[test]
fn test_conflicting_peer_intents_preserved() {
    let mut table = ReservationTable::new(1);

    // Robot 2 claims (5,0) at tick 5
    table.apply_peer_intent(
        2,
        &IntentMsg {
            intent_seq: 1,
            path: vec![(Pos::new(5, 0), 5)],
            priority: 2,
            lamport_ts: 0,
        },
    );

    // Robot 3 also claims (5,0) at tick 5
    table.apply_peer_intent(
        3,
        &IntentMsg {
            intent_seq: 1,
            path: vec![(Pos::new(5, 0), 5)],
            priority: 3,
            lamport_ts: 0,
        },
    );

    let candidate_path = vec![(Pos::new(5, 0), 5)];
    let conflicts = table.conflicts_with_peers(&candidate_path);

    // Both conflicts must be returned (multi-owner preservation)
    assert_eq!(conflicts.len(), 2, "Both peer conflicts should be preserved");
    let peer_ids: Vec<u32> = conflicts.iter().map(|c| c.peer_id).collect();
    assert!(peer_ids.contains(&2));
    assert!(peer_ids.contains(&3));
}

#[test]
fn test_edge_swap_conflict_detected() {
    let mut table = ReservationTable::new(1);

    // Peer 2 moves (1,0) -> (0,0) at tick 0 -> 1
    table.apply_peer_intent(
        2,
        &IntentMsg {
            intent_seq: 1,
            path: vec![(Pos::new(1, 0), 0), (Pos::new(0, 0), 1)],
            priority: 2,
            lamport_ts: 0,
        },
    );

    // Candidate moves (0,0) -> (1,0) at tick 0 -> 1 (head-on swap)
    let candidate = vec![(Pos::new(0, 0), 0), (Pos::new(1, 0), 1)];
    let conflicts = table.conflicts_with_peers(&candidate);

    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0].conflict_type, ConflictType::EdgeSwap);
    assert_eq!(conflicts[0].peer_id, 2);
}

#[test]
fn test_constraints_preserve_multi_owner() {
    let mut table = ReservationTable::new(1);
    table.apply_peer_intent(
        2,
        &IntentMsg {
            intent_seq: 1,
            path: vec![(Pos::new(2, 2), 2)],
            priority: 2,
            lamport_ts: 0,
        },
    );
    table.apply_peer_intent(
        3,
        &IntentMsg {
            intent_seq: 1,
            path: vec![(Pos::new(3, 3), 3)],
            priority: 3,
            lamport_ts: 0,
        },
    );

    let constraints = table.build_constraints();
    assert!(constraints.forbidden_cells.contains(&(Pos::new(2, 2), 2)));
    assert!(constraints.forbidden_cells.contains(&(Pos::new(3, 3), 3)));
}
