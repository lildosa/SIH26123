//! Stop-and-Wait baseline — the reference behavior mandated by the SIH 2026
//! problem statement (BEL/SIH26123): "minimum 20% reduction in total task
//! completion time compared to traditional stop-and-wait methods".
//!
//! This runner models a naive, no-lookahead fleet: each AMR independently
//! follows a free-space shortest path toward its current destination and
//! stops in place whenever the next cell on its path is occupied by another
//! robot (bump-and-stop). Robots neither broadcast intent nor reserve cells a
//! priori; if one has been boxed in for `WAIT_TIMEOUT` ticks it attempts a
//! reactive detour replan around the current obstacle set. Zero collisions
//! hold by construction (a robot never enters a cell that is occupied the
//! instant it moves), which makes it the fair "traditional" baseline THADAM
//! must outperform by >= 20% on overlapping-path scenarios.

use crate::planner::{SpaceTimeConstraints, plan};
use crate::protocol::{RobotId, Tick};
use crate::sim::SimResult;
use crate::world::{GridMap, Pos};

/// Ticks a robot waits before attempting a reactive detour replan.
const WAIT_TIMEOUT: Tick = 6;
/// Planning horizon for free-space (no peer constraints) plans.
const MAX_HORIZON: Tick = 300;

pub struct StopAndWaitConfig {
    pub num_robots: usize,
    pub grid_width: usize,
    pub grid_height: usize,
    pub aisle_spacing: usize,
    pub tasks: Vec<(Pos, Pos)>,
    pub max_ticks: Tick,
    pub start_positions: Vec<Pos>,
}

#[derive(Clone)]
struct SawRobot {
    id: RobotId,
    pos: Pos,
    home: Pos,
    /// Current destination: pickup while `stage == 0`, dropoff while `stage == 1`.
    target: Option<Pos>,
    stage: u8,
    assigned_task: usize,
    /// Free-space waypoints (replanned reactively on wait-timeout).
    path: Vec<(Pos, Tick)>,
    path_idx: usize,
    waiting_since: Option<Tick>,
    /// True while robot is driving back to its home parking cell between tasks.
    returning_home: bool,
}

pub struct StopAndWaitRunner {
    pub config: StopAndWaitConfig,
    grid: GridMap,
}

impl StopAndWaitRunner {
    pub fn new(config: StopAndWaitConfig) -> Self {
        let grid = GridMap::generate_warehouse(
            config.grid_width,
            config.grid_height,
            config.aisle_spacing,
        );
        Self { config, grid }
    }

    /// (Re)plans the free-space shortest path toward the robot's target,
    /// treating `blockers` (other robots' current positions) as transient
    /// obstacles. Returns true if a usable path exists (or the robot is
    /// already at its target).
    fn replan(&self, r: &mut SawRobot, start_tick: Tick, blockers: &[Pos]) -> bool {
        let Some(goal) = r.target else { return true };
        if r.pos == goal {
            r.path = vec![(r.pos, start_tick)];
            r.path_idx = 0;
            return true;
        }

        let mut constraints = SpaceTimeConstraints::default();
        for &b in blockers {
            if b != r.pos {
                constraints.forbidden_cells.insert((b, start_tick));
                constraints.forbidden_cells.insert((b, start_tick + 1));
            }
        }

        match plan(&self.grid, r.id, r.pos, start_tick, goal, &constraints, MAX_HORIZON) {
            Some(p) if !p.is_empty() => {
                r.path = p;
                r.path_idx = 0;
                true
            }
            _ => false,
        }
    }

    pub fn run(&self) -> SimResult {
        let mut result = SimResult::default();
        let total = self.config.tasks.len();
        let mut robots: Vec<SawRobot> = self
            .config
            .start_positions
            .iter()
            .take(self.config.num_robots)
            .enumerate()
            .map(|(i, &p)| SawRobot {
                id: i as RobotId + 1,
                pos: p,
                home: p,
                target: None,
                stage: 0,
                assigned_task: usize::MAX,
                path: Vec::new(),
                path_idx: 0,
                waiting_since: None,
                returning_home: false,
            })
            .collect();

        let mut next_task = 0usize;
        let mut completed = 0usize;
        let mut tick: Tick = 0;

        while tick < self.config.max_ticks && completed < total {
            tick += 1;

            // Occupancy snapshot — robots commit moves in id order and the
            // snapshot is updated incrementally, so later-id robots observe a
            // slightly fresher world (a defining stop-and-wait property: no
            // shared intent, only reactive sensing).
            let mut occupied: Vec<Pos> = robots.iter().map(|r| r.pos).collect();

            for i in 0..robots.len() {
                // 1. FIFO task assignment to idle robots.
                if robots[i].target.is_none() && next_task < total {
                    let (pickup, _dropoff) = self.config.tasks[next_task];
                    next_task += 1;
                    robots[i].assigned_task = next_task - 1;
                    robots[i].stage = 0;
                    robots[i].target = Some(pickup);
                    let _ = self.replan(&mut robots[i], tick, &[]);
                }

                if robots[i].target.is_none() {
                    continue;
                }

                // Keep the path toward the current target fresh if exhausted.
                if robots[i].path_idx + 1 >= robots[i].path.len() {
                    let _ = self.replan(&mut robots[i], tick, &occupied);
                }

                let goal = robots[i].target.unwrap();
                if robots[i].pos == goal {
                    // 2. Arrived at current destination.
                    if robots[i].returning_home {
                        // Reached home parking spot — idle until a task arrives.
                        robots[i].target = None;
                        robots[i].returning_home = false;
                        robots[i].path.clear();
                        robots[i].path_idx = 0;
                    } else if robots[i].stage == 0 {
                        // Reached pickup -> head to dropoff.
                        let task = self.config.tasks[robots[i].assigned_task];
                        robots[i].stage = 1;
                        robots[i].target = Some(task.1);
                        let _ = self.replan(&mut robots[i], tick, &occupied);
                    } else {
                        // Reached dropoff -> task complete.
                        completed += 1;
                        robots[i].stage = 0;
                        robots[i].path.clear();
                        robots[i].path_idx = 0;
                        robots[i].waiting_since = None;
                        // Clear the target so the top-of-tick
                        // assignment picks up the next task next turn.
                        // If no tasks remain, drive back home so the
                        // dropoff cell doesn't block other robots.
                        robots[i].target = None;
                        if next_task >= total {
                            let home = robots[i].home;
                            if robots[i].pos != home {
                                robots[i].returning_home = true;
                                robots[i].target = Some(home);
                                let _ = self.replan(&mut robots[i], tick, &occupied);
                            }
                        }
                    }
                    continue;
                }

                // 3. Stop-and-wait: advance only into an unoccupied cell.
                if robots[i].path_idx + 1 < robots[i].path.len() {
                    let next_pos = robots[i].path[robots[i].path_idx + 1].0;
                    if !occupied.contains(&next_pos) {
                        let old = robots[i].pos;
                        robots[i].pos = next_pos;
                        robots[i].path_idx += 1;
                        robots[i].waiting_since = None;
                        if let Some(slot) = occupied.iter_mut().find(|p| **p == old) {
                            *slot = next_pos;
                        }
                    } else {
                        // Blocked -> wait. After WAIT_TIMEOUT ticks, attempt
                        // recovery (the standard industrial "stall resolution"
                        // for stop-and-wait AGVs): nudge the blocking robot to
                        // detour around us; if we cannot even identify one
                        // blocker, detour ourselves around the current
                        // occupiers.
                        let since = robots[i].waiting_since.unwrap_or(tick);
                        if robots[i].waiting_since.is_none() {
                            robots[i].waiting_since = Some(tick);
                        }
                        if robots[i].path_idx + 1 < robots[i].path.len()
                            && tick - since >= WAIT_TIMEOUT
                        {
                            let blocker_ids: Vec<RobotId> = robots
                                .iter()
                                .filter(|j| j.id != robots[i].id && j.pos == next_pos)
                                .map(|j| j.id)
                                .collect();
                            if let Some(&bid) = blocker_ids.first() {
                                if let Some(bi) = robots.iter().position(|j| j.id == bid) {
                                    if robots[bi].target.is_some() {
                                        // Blocked robot detours around the waiter.
                                        let _ = self.replan(&mut robots[bi], tick, &occupied);
                                        robots[bi].waiting_since = None;
                                    } else {
                                        // Idle blocker parked in the lane:
                                        // side-park it one cell to the nearest
                                        // free adjacent cell (the operator
                                        // clearing a parked unit).
                                        let parked = robots[bi].pos;
                                        let neighbors = [
                                            Pos::new(parked.x + 1, parked.y),
                                            Pos::new(parked.x.saturating_sub(1), parked.y),
                                            Pos::new(parked.x, parked.y + 1),
                                            Pos::new(parked.x, parked.y.saturating_sub(1)),
                                        ];
                                        for c in neighbors {
                                            if self.grid.is_walkable(c)
                                                && !occupied.contains(&c)
                                                && c != robots[i].pos
                                            {
                                                robots[bi].pos = c;
                                                if let Some(slot) = occupied
                                                    .iter_mut()
                                                    .find(|p| **p == parked)
                                                {
                                                    *slot = c;
                                                }
                                                break;
                                            }
                                        }
                                        robots[bi].waiting_since = None;
                                    }
                                }
                            } else {
                                let _ = self.replan(&mut robots[i], tick, &occupied);
                            }
                            robots[i].waiting_since = None;
                        }
                    }
                }
            }
        }

        result.makespan = tick;
        result.tasks_completed = completed;
        result.collisions = 0;
        result.vertex_collisions = 0;
        result.edge_swap_collisions = 0;
        result
    }
}