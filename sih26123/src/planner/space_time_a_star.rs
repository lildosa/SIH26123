use crate::planner::reservations::SpaceTimeConstraints;
use crate::protocol::{RobotId, Tick};
use crate::world::{GridMap, Pos};
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashSet};

#[derive(Clone, Eq, PartialEq)]
struct AStarNode {
    pos: Pos,
    tick: Tick,
    g_cost: usize,
    f_cost: usize,
    index: usize,
    parent_idx: Option<usize>,
}

impl Ord for AStarNode {
    fn cmp(&self, other: &Self) -> Ordering {
        // Min-heap by f_cost; ties broken by HIGHER g_cost (deeper/closer to goal)
        other
            .f_cost
            .cmp(&self.f_cost)
            .then_with(|| self.g_cost.cmp(&other.g_cost))
    }
}

impl PartialOrd for AStarNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Space-Time A* pathfinding algorithm with explicit vertex and edge constraints.
pub fn plan(
    grid: &GridMap,
    _robot_id: RobotId,
    start: Pos,
    start_tick: Tick,
    goal: Pos,
    constraints: &SpaceTimeConstraints,
    max_horizon: Tick,
) -> Option<Vec<(Pos, Tick)>> {
    if !grid.is_walkable(start) || !grid.is_walkable(goal) {
        return None;
    }

    if constraints.forbidden_cells.contains(&(start, start_tick)) {
        return None;
    }

    let initial_h = start.manhattan_distance(&goal);
    let mut open_set = BinaryHeap::new();
    let mut closed_set: HashSet<(Pos, Tick)> = HashSet::new();
    let mut all_nodes: Vec<AStarNode> = Vec::new();

    let start_node = AStarNode {
        pos: start,
        tick: start_tick,
        g_cost: 0,
        f_cost: initial_h,
        index: 0,
        parent_idx: None,
    };
    all_nodes.push(start_node.clone());
    open_set.push(start_node);

    while let Some(current) = open_set.pop() {
        if current.pos == goal {
            // Reconstruct path
            let mut path = Vec::new();
            let mut curr_idx = Some(current.index);
            while let Some(idx) = curr_idx {
                let n = &all_nodes[idx];
                path.push((n.pos, n.tick));
                curr_idx = n.parent_idx;
            }
            path.reverse();
            return Some(path);
        }

        if current.tick > start_tick + max_horizon {
            continue;
        }

        if closed_set.contains(&(current.pos, current.tick)) {
            continue;
        }
        closed_set.insert((current.pos, current.tick));

        // Successors: cardinal moves + wait move
        let mut successors = grid.neighbors(current.pos);
        successors.push(current.pos); // Wait move

        for next_pos in successors {
            let next_tick = current.tick + 1;

            // 1. Check vertex constraints
            if constraints.forbidden_cells.contains(&(next_pos, next_tick)) {
                continue;
            }

            // 2. Check directional edge constraints (head-on edge swaps)
            if next_pos != current.pos
                && constraints
                    .forbidden_edges
                    .contains(&(current.pos, next_pos, current.tick))
            {
                continue;
            }

            if closed_set.contains(&(next_pos, next_tick)) {
                continue;
            }

            let g_cost = current.g_cost + 1;
            let h_cost = next_pos.manhattan_distance(&goal);
            let f_cost = g_cost + h_cost;

            let next_idx = all_nodes.len();
            let next_node = AStarNode {
                pos: next_pos,
                tick: next_tick,
                g_cost,
                f_cost,
                index: next_idx,
                parent_idx: Some(current.index),
            };

            all_nodes.push(next_node.clone());
            open_set.push(next_node);
        }
    }

    None
}
