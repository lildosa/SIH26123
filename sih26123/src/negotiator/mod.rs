pub mod conflict;
pub mod wait_for_graph;

pub use conflict::{should_yield, should_yield_under_uncertainty};
pub use wait_for_graph::WaitForGraph;

use crate::protocol::RobotId;

/// Given a local wait-for graph, detects all deadlock cycles and returns
/// the list of robots that must yield (one per cycle).
pub fn resolve_deadlocks(wfg: &WaitForGraph) -> Vec<RobotId> {
    let cycles = wfg.detect_cycles();
    let mut yielders = Vec::new();
    for cycle in cycles {
        if let Some(yielder) = WaitForGraph::pick_yielder(&cycle) {
            if !yielders.contains(&yielder) {
                yielders.push(yielder);
            }
        }
    }
    yielders
}
