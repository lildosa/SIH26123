use crate::protocol::RobotId;

/// Computes pairwise conflict resolution between two intent versions.
/// Returns true if `my` side should yield to `their` side.
///
/// Priority rule:
/// - Lower priority number wins (e.g. priority 1 beats priority 2).
/// - Ties are broken deterministically by lower RobotId (e.g. Robot 1 beats Robot 2).
pub fn should_yield(
    my_id: RobotId,
    my_priority: u64,
    their_id: RobotId,
    their_priority: u64,
) -> bool {
    if my_priority != their_priority {
        my_priority > their_priority
    } else {
        my_id > their_id
    }
}

/// Deterministic conflict arbitration with causal tie-breaking.
/// Order: lower priority number wins, then older (smaller) Lamport
/// timestamp wins, then lower RobotId wins.
pub fn should_yield_lamport(
    my_id: RobotId,
    my_priority: u64,
    my_lamport: u64,
    their_id: RobotId,
    their_priority: u64,
    their_lamport: u64,
) -> bool {
    if my_priority != their_priority {
        my_priority > their_priority
    } else if my_lamport != their_lamport {
        my_lamport > their_lamport
    } else {
        my_id > their_id
    }
}

/// Under missing or stale peer intent data, default to safe yielding/waiting.
pub fn should_yield_under_uncertainty() -> bool {
    true
}
