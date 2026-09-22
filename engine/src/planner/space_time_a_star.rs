use crate::planner::reservations::SpaceTimeConstraints;
use crate::protocol::{Orientation, RobotId, Tick};
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

/// Heading required to step from `from` to an adjacent `to`.
/// Returns `None` for stationary (same-cell) steps.
pub fn orientation_between(from: Pos, to: Pos) -> Option<Orientation> {
    if to.x == from.x + 1 && to.y == from.y {
        Some(Orientation::East)
    } else if from.x == to.x + 1 && from.y == to.y {
        Some(Orientation::West)
    } else if to.y == from.y + 1 && to.x == from.x {
        Some(Orientation::South)
    } else if from.y == to.y + 1 && to.x == from.x {
        Some(Orientation::North)
    } else {
        None
    }
}

/// Admissible rotational heuristic: Manhattan distance plus 0 when the goal
/// lies straight ahead in the current heading, else 1 (never overestimates:
/// a 90-degree turn costs exactly 1, a 180 costs 2).
pub fn kinematic_heuristic(pos: Pos, orientation: Orientation, goal: Pos) -> usize {
    let manhattan = pos.manhattan_distance(&goal);
    if manhattan == 0 {
        return 0;
    }
    let aligned = match orientation {
        Orientation::East => goal.x > pos.x && goal.y == pos.y,
        Orientation::West => pos.x > goal.x && goal.y == pos.y,
        Orientation::South => goal.y > pos.y && goal.x == pos.x,
        Orientation::North => pos.y > goal.y && goal.x == pos.x,
    };
    manhattan + if aligned { 0 } else { 1 }
}

#[derive(Clone, Eq, PartialEq)]
struct KinematicNode {
    pos: Pos,
    tick: Tick,
    orientation: Orientation,
    g_cost: usize,
    f_cost: usize,
    index: usize,
    parent_idx: Option<usize>,
}

impl Ord for KinematicNode {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .f_cost
            .cmp(&self.f_cost)
            .then_with(|| self.g_cost.cmp(&other.g_cost))
    }
}

impl PartialOrd for KinematicNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Discrete Space-Time Reservation Grid with Heading Change Latency over `(Pos, Tick, Orientation)`.
///
/// When the required heading differs from the current heading, the planner
/// applies a Turn-Delay Cost Matrix, injecting `rotation_cost` stationary waits
/// at the current cell (1 tick for 90° turn, 2 ticks for 180° turnaround) before the move.
/// Those `(u, t+1 ..= t+dt)` entries are part of the returned path, so `reserve_own_path`
/// locks them as stationary vertex reservations and peers route around the turning robot.
pub fn plan_with_orientation(
    grid: &GridMap,
    _robot_id: RobotId,
    start: Pos,
    start_tick: Tick,
    start_orientation: Orientation,
    goal: Pos,
    constraints: &SpaceTimeConstraints,
    max_horizon: Tick,
) -> Option<Vec<(Pos, Tick)>> {
    if !grid.is_walkable(start) || !grid.is_walkable(goal) {
        return None;
    }

    let mut open_set = BinaryHeap::new();
    let mut closed_set: HashSet<(Pos, Tick, Orientation)> = HashSet::new();
    let mut all_nodes: Vec<KinematicNode> = Vec::new();

    let start_node = KinematicNode {
        pos: start,
        tick: start_tick,
        orientation: start_orientation,
        g_cost: 0,
        f_cost: kinematic_heuristic(start, start_orientation, goal),
        index: 0,
        parent_idx: None,
    };
    all_nodes.push(start_node.clone());
    open_set.push(start_node);

    while let Some(current) = open_set.pop() {
        if current.pos == goal {
            // Reconstruct node chain, expanding in-place turns into explicit
            // stationary waits so the ribbon locks (u, t+1 ..= t+dt).
            let mut chain = Vec::new();
            let mut curr_idx = Some(current.index);
            while let Some(idx) = curr_idx {
                chain.push(all_nodes[idx].clone());
                curr_idx = all_nodes[idx].parent_idx;
            }
            chain.reverse();
            let mut path = Vec::new();
            for (i, n) in chain.iter().enumerate() {
                if i == 0 {
                    path.push((n.pos, n.tick));
                    continue;
                }
                let prev = &chain[i - 1];
                if prev.pos == n.pos {
                    path.push((n.pos, n.tick));
                } else {
                    let turn = prev.orientation.rotation_cost(n.orientation) as u64;
                    for t in (prev.tick + 1)..n.tick {
                        let _ = turn;
                        path.push((prev.pos, t));
                    }
                    path.push((n.pos, n.tick));
                }
            }
            return Some(path);
        }
        if current.tick > start_tick + max_horizon {
            continue;
        }
        if closed_set.contains(&(current.pos, current.tick, current.orientation)) {
            continue;
        }
        closed_set.insert((current.pos, current.tick, current.orientation));

        // Wait move: same cell, same heading, +1 tick.
        if !constraints
            .forbidden_cells
            .contains(&(current.pos, current.tick + 1))
        {
            let g = current.g_cost + 1;
            let f = g + kinematic_heuristic(current.pos, current.orientation, goal);
            let idx = all_nodes.len();
            let node = KinematicNode {
                pos: current.pos,
                tick: current.tick + 1,
                orientation: current.orientation,
                g_cost: g,
                f_cost: f,
                index: idx,
                parent_idx: Some(current.index),
            };
            all_nodes.push(node.clone());
            open_set.push(node);
        }

        // Cardinal moves with turning delays.
        for next_pos in grid.neighbors(current.pos) {
            let Some(required) = orientation_between(current.pos, next_pos) else {
                continue;
            };
            let turn = current.orientation.rotation_cost(required) as usize;
            let arrival_tick = current.tick + 1 + turn as u64;

            // Turning invariant: intermediate stationary ticks at `current.pos`
            // must be free, plus the arrival cell at `arrival_tick`.
            let mut blocked = false;
            for t in (current.tick + 1)..arrival_tick {
                if constraints.forbidden_cells.contains(&(current.pos, t)) {
                    blocked = true;
                    break;
                }
            }
            if blocked {
                continue;
            }
            if constraints
                .forbidden_cells
                .contains(&(next_pos, arrival_tick))
            {
                continue;
            }
            let step_tick = current.tick + turn as u64;
            if constraints
                .forbidden_edges
                .contains(&(current.pos, next_pos, step_tick))
            {
                continue;
            }
            if closed_set.contains(&(next_pos, arrival_tick, required)) {
                continue;
            }

            let g = current.g_cost + 1 + turn;
            let f = g + kinematic_heuristic(next_pos, required, goal);
            let idx = all_nodes.len();
            let node = KinematicNode {
                pos: next_pos,
                tick: arrival_tick,
                orientation: required,
                g_cost: g,
                f_cost: f,
                index: idx,
                parent_idx: Some(current.index),
            };
            all_nodes.push(node.clone());
            open_set.push(node);
        }
    }

    None
}
