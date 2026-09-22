use crate::protocol::RobotId;
use std::collections::{HashMap, HashSet};

/// Per-robot local wait-for graph.
/// Used ONLY for multi-robot deadlock-CYCLE detection (3+ robots in circular wait).
/// Normal pairwise conflicts are resolved directly by `conflict::should_yield`.
#[derive(Debug, Clone, Default)]
pub struct WaitForGraph {
    pub edges: HashMap<RobotId, HashSet<RobotId>>,
}

impl WaitForGraph {
    pub fn new() -> Self {
        Self {
            edges: HashMap::new(),
        }
    }

    /// Adds a directed edge: `waiter` is waiting for `blocking` to move.
    pub fn add_wait(&mut self, waiter: RobotId, blocking: RobotId) {
        if waiter != blocking {
            self.edges.entry(waiter).or_default().insert(blocking);
        }
    }

    /// Removes a single directed wait edge.
    pub fn remove_edge(&mut self, waiter: RobotId, blocking: RobotId) {
        if let Some(targets) = self.edges.get_mut(&waiter) {
            targets.remove(&blocking);
        }
    }

    /// Removes all incoming and outgoing edges for `robot_id`.
    pub fn remove_robot(&mut self, robot_id: RobotId) {
        self.edges.remove(&robot_id);
        for targets in self.edges.values_mut() {
            targets.remove(&robot_id);
        }
    }

    pub fn clear(&mut self) {
        self.edges.clear();
    }

    /// Detects all elementary directed cycles in the graph using DFS.
    pub fn detect_cycles(&self) -> Vec<Vec<RobotId>> {
        let mut cycles = Vec::new();
        let mut visited = HashSet::new();
        let mut on_stack = HashSet::new();
        let mut path = Vec::new();

        let all_nodes: Vec<RobotId> = self.edges.keys().copied().collect();

        for node in all_nodes {
            if !visited.contains(&node) {
                self.dfs_find_cycles(node, &mut visited, &mut on_stack, &mut path, &mut cycles);
            }
        }

        // Deduplicate cycles by rotating to start with min element and sorting
        let mut unique_cycles = Vec::new();
        let mut seen_canonical = HashSet::new();

        for cycle in cycles {
            if cycle.is_empty() {
                continue;
            }
            let min_pos = cycle
                .iter()
                .enumerate()
                .min_by_key(|entry| *entry.1)
                .map(|entry| entry.0)
                .unwrap_or(0);

            let mut canonical = Vec::with_capacity(cycle.len());
            canonical.extend_from_slice(&cycle[min_pos..]);
            canonical.extend_from_slice(&cycle[..min_pos]);

            if seen_canonical.insert(canonical.clone()) {
                unique_cycles.push(canonical);
            }
        }

        unique_cycles
    }

    fn dfs_find_cycles(
        &self,
        curr: RobotId,
        visited: &mut HashSet<RobotId>,
        on_stack: &mut HashSet<RobotId>,
        path: &mut Vec<RobotId>,
        cycles: &mut Vec<Vec<RobotId>>,
    ) {
        visited.insert(curr);
        on_stack.insert(curr);
        path.push(curr);

        if let Some(neighbors) = self.edges.get(&curr) {
            for &next in neighbors {
                if on_stack.contains(&next) {
                    // Cycle detected from `next` to `curr` in `path`
                    if let Some(pos) = path.iter().position(|&x| x == next) {
                        let cycle = path[pos..].to_vec();
                        cycles.push(cycle);
                    }
                } else if !visited.contains(&next) {
                    self.dfs_find_cycles(next, visited, on_stack, path, cycles);
                }
            }
        }

        path.pop();
        on_stack.remove(&curr);
    }

    /// In a deadlock cycle, the robot with the HIGHEST RobotId yields deterministically.
    pub fn pick_yielder(cycle: &[RobotId]) -> Option<RobotId> {
        cycle.iter().copied().max()
    }
}
