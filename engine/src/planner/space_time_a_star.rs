use crate::ai::{SharedGuidance, heatmap_cost_at};
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
    } else if from.x == to.x + 1 && from.y == from.y {
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

// ---------------------------------------------------------------------------
// Neural guidance wiring (advisory only — ISO 3691-4 invariants untouched)
// ---------------------------------------------------------------------------

/// Expansion budget for the neural-guided phase. If the guided search
/// exceeds this many popped states, the planner immediately falls back to
/// the pure kinematic A* (hard safety invariant from the design audit).
pub const NEURAL_FALLBACK_EXPANSIONS: usize = 800;

/// Telemetry for planner benchmarking (neural vs kinematic expansions).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PlanStats {
    /// Total nodes popped from the open set across all search phases.
    pub expansions: usize,
    /// True when the guided phase overran its budget and the pure kinematic
    /// A* produced the final path.
    pub used_fallback: bool,
    /// True when a guidance heatmap was available and consulted.
    pub used_guidance: bool,
}

struct SearchResult {
    path: Option<Vec<(Pos, Tick)>>,
    expansions: usize,
    /// Open set exhausted the budget before proving unreachability.
    hit_cap: bool,
}

/// Core kinematic space-time search. `h` decides open-set priorities;
/// `max_expansions` caps popped states (`usize::MAX` = unbounded).
fn kinematic_search(
    grid: &GridMap,
    start: Pos,
    start_tick: Tick,
    start_orientation: Orientation,
    goal: Pos,
    constraints: &SpaceTimeConstraints,
    max_horizon: Tick,
    h: &dyn Fn(Pos, Orientation) -> usize,
    max_expansions: usize,
) -> SearchResult {
    if !grid.is_walkable(start) || !grid.is_walkable(goal) {
        return SearchResult {
            path: None,
            expansions: 0,
            hit_cap: false,
        };
    }

    let mut open_set = BinaryHeap::new();
    let mut closed_set: HashSet<(Pos, Tick, Orientation)> = HashSet::new();
    let mut all_nodes: Vec<KinematicNode> = Vec::new();
    let mut expansions: usize = 0;

    let start_node = KinematicNode {
        pos: start,
        tick: start_tick,
        orientation: start_orientation,
        g_cost: 0,
        f_cost: h(start, start_orientation),
        index: 0,
        parent_idx: None,
    };
    all_nodes.push(start_node.clone());
    open_set.push(start_node);

    while let Some(current) = open_set.pop() {
        expansions += 1;
        if expansions > max_expansions {
            return SearchResult {
                path: None,
                expansions,
                hit_cap: true,
            };
        }

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
            return SearchResult {
                path: Some(path),
                expansions,
                hit_cap: false,
            };
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
            let f = g + h(current.pos, current.orientation);
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
            let f = g + h(next_pos, required);
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

    SearchResult {
        path: None,
        expansions,
        hit_cap: false,
    }
}

/// Kinematic A* — identical behavior to the pre-guidance planner.
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
    plan_with_orientation_stats(
        grid,
        _robot_id,
        start,
        start_tick,
        start_orientation,
        goal,
        constraints,
        max_horizon,
        None,
    )
    .map(|(path, _)| path)
}

/// Neural-guided variant: one advisory cost-to-go heatmap per planning call
/// reorders the BinaryHeap open set. Safety invariants are untouched:
///   * hard vertex reservations + directional edge-swap checks enforced
///     exactly as in `kinematic_search`,
///   * if the guided phase pops more than `NEURAL_FALLBACK_EXPANSIONS`
///     states, the planner immediately falls back to pure kinematic A*.
///
/// Returns the path plus expansion/fallback telemetry for benchmarking.
pub fn plan_with_orientation_stats(
    grid: &GridMap,
    _robot_id: RobotId,
    start: Pos,
    start_tick: Tick,
    start_orientation: Orientation,
    goal: Pos,
    constraints: &SpaceTimeConstraints,
    max_horizon: Tick,
    guidance: Option<&SharedGuidance>,
) -> Option<(Vec<(Pos, Tick)>, PlanStats)> {
    let kinematic = |pos: Pos, o: Orientation| kinematic_heuristic(pos, o, goal);

    if let Some(engine) = guidance {
        if let Some(heat) = engine.infer_heatmap(grid, goal) {
            // Advisory heuristic: residual Neural A*. The model predicts the
            // obstacle-detour residual on top of Manhattan (h = kinematic +
            // residual·32), so in open fields ordering degrades gracefully to
            // plain A*, and around shelves the residual steers the open set
            // along the detour side early. The 800-expansion budget + pure
            // kinematic fallback contain any ordering pathology (ISO 3691-4
            // invariants remain enforced).
            let neural = |pos: Pos, o: Orientation| {
                if pos.x < crate::ai::GUIDANCE_GRID && pos.y < crate::ai::GUIDANCE_GRID {
                    kinematic_heuristic(pos, o, goal) + heatmap_cost_at(&heat, pos)
                } else {
                    kinematic_heuristic(pos, o, goal)
                }
            };

            let guided = kinematic_search(
                grid,
                start,
                start_tick,
                start_orientation,
                goal,
                constraints,
                max_horizon,
                &neural,
                NEURAL_FALLBACK_EXPANSIONS,
            );

            if let Some(path) = guided.path {
                return Some((
                    path,
                    PlanStats {
                        expansions: guided.expansions,
                        used_fallback: false,
                        used_guidance: true,
                    },
                ));
            }

            // Open-set exhausted within budget = provably no path; no point
            // rerunning the same search without guidance.
            if !guided.hit_cap {
                return None;
            }

            // Safety fallback: budget overrun → pure kinematic A*.
            let plain = kinematic_search(
                grid,
                start,
                start_tick,
                start_orientation,
                goal,
                constraints,
                max_horizon,
                &kinematic,
                usize::MAX,
            );
            if let Some(path) = plain.path {
                return Some((
                    path,
                    PlanStats {
                        expansions: guided.expansions + plain.expansions,
                        used_fallback: true,
                        used_guidance: true,
                    },
                ));
            }
            return None;
        }
    }

    let plain = kinematic_search(
        grid,
        start,
        start_tick,
        start_orientation,
        goal,
        constraints,
        max_horizon,
        &kinematic,
        usize::MAX,
    );
    plain.path.map(|path| {
        (
            path,
            PlanStats {
                expansions: plain.expansions,
                used_fallback: false,
                used_guidance: false,
            },
        )
    })
}
