use sih26123::negotiator::{resolve_deadlocks, should_yield, should_yield_under_uncertainty, WaitForGraph};

#[test]
fn test_no_deadlock() {
    let mut wfg = WaitForGraph::new();
    wfg.add_wait(1, 2); // 1 waits for 2, 2 is moving freely
    let cycles = wfg.detect_cycles();
    assert!(cycles.is_empty(), "No cycle should be detected in linear wait");
    assert!(resolve_deadlocks(&wfg).is_empty());
}

#[test]
fn test_simple_cycle() {
    let mut wfg = WaitForGraph::new();
    wfg.add_wait(1, 2);
    wfg.add_wait(2, 1);
    let cycles = wfg.detect_cycles();
    assert_eq!(cycles.len(), 1, "Exactly one cycle should be detected");
    let yielders = resolve_deadlocks(&wfg);
    assert_eq!(yielders, vec![2], "Robot 2 (highest ID) should yield");
}

#[test]
fn test_three_robot_cycle() {
    let mut wfg = WaitForGraph::new();
    wfg.add_wait(1, 2);
    wfg.add_wait(2, 3);
    wfg.add_wait(3, 1);
    let cycles = wfg.detect_cycles();
    assert_eq!(cycles.len(), 1);
    let yielders = resolve_deadlocks(&wfg);
    assert_eq!(yielders, vec![3], "Robot 3 should yield in 1->2->3->1 cycle");
}

#[test]
fn test_two_independent_cycles() {
    let mut wfg = WaitForGraph::new();
    // Cycle 1: 1 <-> 2
    wfg.add_wait(1, 2);
    wfg.add_wait(2, 1);
    // Cycle 2: 3 <-> 4
    wfg.add_wait(3, 4);
    wfg.add_wait(4, 3);

    let cycles = wfg.detect_cycles();
    assert_eq!(cycles.len(), 2, "Two distinct cycles should be found");
    let mut yielders = resolve_deadlocks(&wfg);
    yielders.sort();
    assert_eq!(yielders, vec![2, 4], "Robots 2 and 4 should yield");
}

#[test]
fn test_pairwise_lower_priority_yields() {
    // Robot 1 has priority 2 (higher priority)
    // Robot 5 has priority 7 (lower priority)
    assert!(
        should_yield(5, 7, 1, 2),
        "Robot 5 (priority 7) should yield to Robot 1 (priority 2)"
    );
    assert!(
        !should_yield(1, 2, 5, 7),
        "Robot 1 (priority 2) should NOT yield to Robot 5 (priority 7)"
    );
}

#[test]
fn test_pairwise_tiebreak_by_id() {
    // Robot 3 and Robot 7 both have priority 2
    assert!(
        should_yield(7, 2, 3, 2),
        "Robot 7 (higher ID) should yield to Robot 3 on equal priority"
    );
    assert!(
        !should_yield(3, 2, 7, 2),
        "Robot 3 (lower ID) should NOT yield to Robot 7 on equal priority"
    );
}

#[test]
fn test_stale_intent_yields_safely() {
    assert!(
        should_yield_under_uncertainty(),
        "Under missing or stale peer intent, should default to safe yielding/waiting"
    );
}

#[test]
fn test_deterministic_under_stale_views() {
    // Robot A (id=1, prio=1) evaluates against Robot B (id=2, prio=2)
    let a_view_a_yields = should_yield(1, 1, 2, 2);
    // Robot B (id=2, prio=2) evaluates against Robot A (id=1, prio=1)
    let b_view_b_yields = should_yield(2, 2, 1, 1);

    // Both sides must independently agree: A does not yield, B yields
    assert!(!a_view_a_yields, "Robot A correctly knows it does not yield");
    assert!(b_view_b_yields, "Robot B correctly knows it must yield");
}

#[test]
fn test_simultaneous_conflict_deterministic() {
    // If two robots detect a conflict at the exact same tick:
    let r1_yields = should_yield(1, 1, 2, 2);
    let r2_yields = should_yield(2, 2, 1, 1);

    assert_eq!(
        (r1_yields, r2_yields),
        (false, true),
        "Simultaneous conflict evaluation must produce exactly one winner and one yielder"
    );
}
