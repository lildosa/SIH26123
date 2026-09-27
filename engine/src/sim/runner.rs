use crate::ai::{LinUcbBandit, SharedGuidance};
use crate::network::Network;
use crate::network::in_memory::{InMemoryBus, InMemoryNode};
use crate::node::actor::RobotActor;
use crate::node::environment::Environment;
use crate::node::state::RobotState;
use crate::protocol::{RobotId, TaskId, TaskState, TaskStatusMsg, Tick};
use crate::world::{Cell, GridMap, Pos};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::{Arc, RwLock};

/// How neural A* guidance is engaged for a run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum GuidancePolicy {
    /// Guidance off (default; deterministic legacy behavior).
    #[default]
    Off,
    /// Guidance always on (falls back internally past 800 expansions).
    Always,
    /// Hybrid routing: engage guidance only in the high-congestion regime
    /// where the learned detour heuristic measurably wins (large grid AND
    /// dense fleet); pure kinematic planning elsewhere.
    Auto,
}

impl GuidancePolicy {
    /// Routing rule from the 360-run sweep (10 seeds × sizes × fleets):
    /// the only statistically decisive neural-guidance win is the large,
    /// dense regime — 32×32 with ≥ 0.7 robots per 100 cells (8/1024 = 0.78
    /// engaged, 6/1024 = 0.59 did not; 85% vs 62% completion). Smaller or
    /// sparser regimes are within seed noise, so Auto conservatively keeps
    /// pure kinematic planning there.
    ///
    /// Density uses division-free integer math (robots/1000 vs cells/1000)
    /// to avoid integer-division floor traps.
    pub fn engages(&self, num_robots: usize, grid_width: usize, grid_height: usize) -> bool {
        match self {
            GuidancePolicy::Off => false,
            GuidancePolicy::Always => true,
            GuidancePolicy::Auto => {
                let cells = grid_width.max(grid_height) >= 32;
                let dense = num_robots * 1000 >= 7 * (grid_width * grid_height).max(1);
                cells && dense
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimConfig {
    pub num_robots: usize,
    pub grid_width: usize,
    pub grid_height: usize,
    pub aisle_spacing: usize,
    pub tasks: Vec<(Pos, Pos)>,
    pub max_ticks: Tick,
    pub kill_robot_at: Option<(RobotId, Tick)>,
    pub block_cell_at: Option<(Pos, Tick)>,
    pub start_positions: Vec<Pos>,
    /// Opt-in neural A* guidance (advisory heuristic only). Default off.
    #[serde(default)]
    pub use_neural_guidance: bool,
    /// Opt-in LinUCB adaptive bidding. Default off (static weights).
    #[serde(default)]
    pub use_learned_bids: bool,
    /// Seeded task/start jitter for statistical variance. `None` keeps the
    /// fully deterministic legacy layout (all existing tests unchanged).
    #[serde(default)]
    pub task_seed: Option<u64>,
    /// Hybrid guidance routing (default: Off = legacy boolean behavior).
    /// When `Auto`, `use_neural_guidance` is overridden per-regime.
    #[serde(default)]
    pub guidance_policy: GuidancePolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SimResult {
    pub collisions: usize,
    pub vertex_collisions: usize,
    pub edge_swap_collisions: usize,
    pub makespan: Tick,
    pub tasks_completed: usize,
    pub deadlocks_resolved: usize,
    pub total_wait_ticks: usize,
    /// Neural-guidance telemetry (0 when guidance is disabled).
    #[serde(default)]
    pub neural_plans: usize,
    #[serde(default)]
    pub neural_expansions: usize,
    #[serde(default)]
    pub neural_fallbacks: usize,
    /// Open-set expansions summed over every planning call (guided or not).
    #[serde(default)]
    pub plan_expansions: usize,
    /// Expansions from plans that consulted the guidance heatmap.
    #[serde(default)]
    pub guided_plan_expansions: usize,
    /// Adaptive (LinUCB) bids placed across the fleet (0 when disabled).
    #[serde(default)]
    pub learned_bids: usize,
    /// Task selections per bidding arm [conservative, speed, balanced, aggressive].
    #[serde(default)]
    pub bandit_arm_counts: [u32; 4],
    /// Mean completion reward observed per arm.
    #[serde(default)]
    pub bandit_arm_reward_sum: [f32; 4],
}

pub struct SimEnvironment {
    pub ground_truth: RwLock<GridMap>,
}

impl SimEnvironment {
    pub fn new(grid: GridMap) -> Self {
        Self {
            ground_truth: RwLock::new(grid),
        }
    }

    pub fn inject_obstacle(&self, pos: Pos) {
        let mut g = self.ground_truth.write().unwrap();
        if g.in_bounds(pos) {
            g.set_cell(pos, Cell::Wall);
        }
    }

    pub fn remove_obstacle(&self, pos: Pos) {
        let mut g = self.ground_truth.write().unwrap();
        if g.in_bounds(pos) {
            g.set_cell(pos, Cell::Free);
        }
    }
}

impl Environment for SimEnvironment {
    fn sense_obstacles(&self, center: Pos, range: usize) -> Vec<Pos> {
        let mut sensed = Vec::new();
        let g = self.ground_truth.read().unwrap();

        for dy in -(range as isize)..=(range as isize) {
            for dx in -(range as isize)..=(range as isize) {
                let nx = center.x as isize + dx;
                let ny = center.y as isize + dy;
                if nx >= 0 && ny >= 0 {
                    let p = Pos::new(nx as usize, ny as usize);
                    if g.in_bounds(p) && g.get_cell(p) == Cell::Wall {
                        sensed.push(p);
                    }
                }
            }
        }

        sensed
    }

    fn is_blocked(&self, pos: Pos) -> bool {
        let g = self.ground_truth.read().unwrap();
        !g.is_walkable(pos)
    }
}

pub struct SimRunner {
    pub config: SimConfig,
    pub grid: Arc<GridMap>,
    pub environment: Arc<SimEnvironment>,
    pub bus: Arc<InMemoryBus>,
    pub nodes: Vec<Arc<InMemoryNode>>,
    pub robots: Vec<RobotActor>,
    pub current_tick: Tick,
    pub next_task_id: TaskId,
}

/// Tiny deterministic LCG for seeded task/start jitter (no rand dep here so
/// the jitter is reproducible across platforms for a given seed).
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 16
    }

    fn shuffle<T>(&mut self, xs: &mut [T]) {
        for i in (1..xs.len()).rev() {
            let j = (self.next() % (i as u64 + 1)) as usize;
            xs.swap(i, j);
        }
    }
}

impl SimRunner {
    pub fn new(mut config: SimConfig) -> Self {
        // Seeded jitter: reorder tasks and start positions so different seeds
        // exercise different fleet/task geometry (statistical variance).
        if let Some(seed) = config.task_seed {
            let mut lcg = Lcg(seed);
            lcg.shuffle(&mut config.tasks);
            lcg.shuffle(&mut config.start_positions);
        }

        let grid = Arc::new(GridMap::generate_warehouse(
            config.grid_width,
            config.grid_height,
            config.aisle_spacing,
        ));
        let environment = Arc::new(SimEnvironment::new((*grid).clone()));
        let bus = InMemoryBus::new();

        let mut robots = Vec::new();
        let mut nodes = Vec::new();

        for i in 0..config.num_robots {
            let robot_id = (i + 1) as RobotId;
            let start_pos = if i < config.start_positions.len() {
                config.start_positions[i]
            } else {
                Pos::new(i * 2, 0)
            };

            let node = Arc::new(bus.register_node(robot_id));
            nodes.push(node.clone());

            let mut actor =
                RobotActor::new(robot_id, start_pos, grid.clone(), node, environment.clone());
            if config.use_neural_guidance
                || config.guidance_policy.engages(
                    config.num_robots,
                    config.grid_width,
                    config.grid_height,
                )
            {
                actor.guidance = SharedGuidance::global();
            }
            if config.use_learned_bids {
                actor.bid_bandit = Some(LinUcbBandit::new());
            }

            for (idx, &(pickup, dropoff)) in config.tasks.iter().enumerate() {
                let task_id = (idx + 1) as TaskId;
                actor.known_tasks.insert(
                    task_id,
                    TaskStatusMsg {
                        task_id,
                        pickup,
                        dropoff,
                        assigned_to: None,
                        status: TaskState::Open,
                    },
                );
            }

            robots.push(actor);
        }

        for i in 0..robots.len() {
            let other_poses: Vec<(RobotId, Pos)> = robots
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != i)
                .map(|(_, r)| (r.id, r.pos))
                .collect();

            for (other_id, other_pos) in other_poses {
                robots[i].peer_poses.insert(other_id, (other_pos, 0));
            }
        }

        let next_task_id = (config.tasks.len() + 1) as TaskId;

        Self {
            config,
            grid,
            environment,
            bus,
            nodes,
            robots,
            current_tick: 0,
            next_task_id,
        }
    }

    /// Reconfigures and resets fleet with new parameters preserving shared Arcs.
    pub fn reinitialize(&mut self, mut config: SimConfig) {
        if let Some(seed) = config.task_seed {
            let mut lcg = Lcg(seed);
            lcg.shuffle(&mut config.tasks);
            lcg.shuffle(&mut config.start_positions);
        }
        let mut gt = self.environment.ground_truth.write().unwrap();
        *gt = (*self.grid).clone();
        drop(gt);

        self.config = config.clone();
        self.current_tick = 0;
        self.next_task_id = (config.tasks.len() + 1) as TaskId;

        let mut robots = Vec::new();
        let mut nodes = Vec::new();

        for i in 0..config.num_robots {
            let robot_id = (i + 1) as RobotId;
            let start_pos = if i < config.start_positions.len() {
                config.start_positions[i]
            } else {
                Pos::new(i * 2, 0)
            };

            let node = Arc::new(self.bus.register_node(robot_id));
            nodes.push(node.clone());

            let mut actor = RobotActor::new(
                robot_id,
                start_pos,
                self.grid.clone(),
                node,
                self.environment.clone(),
            );
            if config.use_neural_guidance
                || config.guidance_policy.engages(
                    config.num_robots,
                    config.grid_width,
                    config.grid_height,
                )
            {
                actor.guidance = SharedGuidance::global();
            }
            if config.use_learned_bids {
                actor.bid_bandit = Some(LinUcbBandit::new());
            }

            for (idx, &(pickup, dropoff)) in config.tasks.iter().enumerate() {
                let task_id = (idx + 1) as TaskId;
                actor.known_tasks.insert(
                    task_id,
                    TaskStatusMsg {
                        task_id,
                        pickup,
                        dropoff,
                        assigned_to: None,
                        status: TaskState::Open,
                    },
                );
            }

            robots.push(actor);
        }

        for i in 0..robots.len() {
            let other_poses: Vec<(RobotId, Pos)> = robots
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != i)
                .map(|(_, r)| (r.id, r.pos))
                .collect();

            for (other_id, other_pos) in other_poses {
                robots[i].peer_poses.insert(other_id, (other_pos, 0));
            }
        }

        self.nodes = nodes;
        self.robots = robots;
    }

    /// Dynamically injects a new pickup/dropoff task into the live auction pool.
    pub fn inject_task(&mut self, pickup: Pos, dropoff: Pos) -> TaskId {
        let task_id = self.next_task_id;
        self.next_task_id += 1;

        for robot in &mut self.robots {
            robot.known_tasks.insert(
                task_id,
                TaskStatusMsg {
                    task_id,
                    pickup,
                    dropoff,
                    assigned_to: None,
                    status: TaskState::Open,
                },
            );
        }

        task_id
    }

    /// Direct manual dispatch: command a specific robot directly to a destination.
    pub fn dispatch_robot_to(&mut self, robot_id: RobotId, target: Pos) {
        if let Some(r) = self.robots.iter_mut().find(|r| r.id == robot_id) {
            let task_id = self.next_task_id;
            self.next_task_id += 1;
            r.assigned_task = Some((task_id, r.pos, target));
            r.state = RobotState::Planning {
                task_id,
                pickup: r.pos,
                dropoff: target,
            };
        }
    }

    /// Dynamically kills a robot during live execution.
    pub fn kill_robot(&mut self, robot_id: RobotId) {
        if let Some(r) = self.robots.iter_mut().find(|r| r.id == robot_id) {
            r.state = RobotState::Dead;
            r.alive = false;
            self.environment.inject_obstacle(r.pos);
        }
    }

    /// Dynamically revives a dead robot back to service.
    pub fn revive_robot(&mut self, robot_id: RobotId) {
        let mut robot_pos = None;
        if let Some(r) = self.robots.iter_mut().find(|r| r.id == robot_id) {
            r.state = RobotState::Idle;
            r.alive = true;
            r.battery = 1.0;
            r.assigned_task = None;
            r.desired_next_pos = None;
            robot_pos = Some(r.pos);
        }

        if let Some(pos) = robot_pos {
            self.environment.remove_obstacle(pos);
            for peer in &mut self.robots {
                if peer.id != robot_id {
                    peer.local_obstacles.remove(&pos);
                    peer.last_heartbeats.insert(robot_id, self.current_tick);
                    peer.peer_poses.insert(robot_id, (pos, self.current_tick));
                }
            }
        }
    }

    /// Advances the simulation by exactly one 5-phase synchronous tick.
    pub async fn step_tick(&mut self) -> (Tick, usize, usize, usize) {
        self.current_tick += 1;
        let tick = self.current_tick;

        if let Some((kill_id, kill_tick)) = self.config.kill_robot_at {
            if tick == kill_tick {
                self.kill_robot(kill_id);
            }
        }

        if let Some((block_pos, block_tick)) = self.config.block_cell_at {
            if tick == block_tick {
                self.environment.inject_obstacle(block_pos);
            }
        }

        // Phase 1: Local Sensing
        for robot in &mut self.robots {
            robot.sense_phase();
        }

        // Phase 2: State Machine Decision
        for i in 0..self.robots.len() {
            let inbox = self.nodes[i].drain().await;
            self.robots[i].decide_phase(inbox);
        }

        // Phase 3: Network Flush
        for i in 0..self.robots.len() {
            for env in self.robots[i].outbox.drain(..) {
                let _ = self.nodes[i].broadcast(env).await;
            }
        }
        self.bus.flush_tick();

        // Phase 4: Simultaneous Movement Commit
        let mut movements: Vec<(RobotId, Pos, Option<Pos>)> = Vec::new();
        for robot in &mut self.robots {
            let (prev, next) = robot.commit_movement();
            movements.push((robot.id, prev, next));
        }

        // Phase 5: Evaluation & Collision Detection
        let mut vertex_cols = 0;
        let mut edge_cols = 0;
        let num_robots = movements.len();

        for i in 0..num_robots {
            for j in (i + 1)..num_robots {
                let curr_i = movements[i].2.unwrap_or(movements[i].1);
                let curr_j = movements[j].2.unwrap_or(movements[j].1);

                if curr_i == curr_j {
                    vertex_cols += 1;
                }

                let prev_i = movements[i].1;
                let prev_j = movements[j].1;
                if let (Some(next_i), Some(next_j)) = (movements[i].2, movements[j].2) {
                    if prev_i == next_j && prev_j == next_i && prev_i != next_i {
                        edge_cols += 1;
                    }
                }
            }
        }

        let completed = self
            .robots
            .iter()
            .flat_map(|r| r.known_tasks.values())
            .filter(|t| t.status == TaskState::Completed)
            .map(|t| t.task_id)
            .collect::<HashSet<_>>()
            .len();

        (tick, vertex_cols, edge_cols, completed)
    }

    pub async fn run(&mut self) -> SimResult {
        let mut result = SimResult::default();
        let total_tasks = self.config.tasks.len();

        while self.current_tick < self.config.max_ticks {
            let (tick, vertex_cols, edge_cols, completed) = self.step_tick().await;
            result.makespan = tick;
            result.vertex_collisions += vertex_cols;
            result.edge_swap_collisions += edge_cols;
            result.collisions += vertex_cols + edge_cols;
            result.tasks_completed = completed;

            if completed >= total_tasks {
                break;
            }
        }

        result.neural_plans = self.robots.iter().map(|r| r.neural_plans).sum();
        result.neural_expansions = self.robots.iter().map(|r| r.neural_expansions).sum();
        result.neural_fallbacks = self.robots.iter().map(|r| r.neural_fallbacks).sum();
        result.plan_expansions = self.robots.iter().map(|r| r.plan_expansions).sum();
        result.guided_plan_expansions = self.robots.iter().map(|r| r.neural_expansions).sum();
        result.learned_bids = self.robots.iter().map(|r| r.learned_bids).sum();
        for r in &self.robots {
            if let Some(b) = &r.bid_bandit {
                for a in 0..4 {
                    result.bandit_arm_counts[a] =
                        result.bandit_arm_counts[a].saturating_add(b.arm_counts[a]);
                    result.bandit_arm_reward_sum[a] += b.arm_reward_sum[a];
                }
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guidance_policy_routing_rule() {
        // Off / Always are regime-independent.
        assert!(!GuidancePolicy::Off.engages(8, 32, 32));
        assert!(GuidancePolicy::Always.engages(2, 10, 10));

        // Dense large regime: 8 robots on 32x32 (0.78/100 cells) -> engaged.
        assert!(GuidancePolicy::Auto.engages(8, 32, 32));
        // 6 robots on 32x32 (0.59/100) -> below threshold, off.
        assert!(!GuidancePolicy::Auto.engages(6, 32, 32));
        // Grids below 32 never engage (sweep noise), however dense.
        assert!(!GuidancePolicy::Auto.engages(8, 15, 15));
        assert!(!GuidancePolicy::Auto.engages(8, 24, 24));
        assert!(!GuidancePolicy::Auto.engages(6, 24, 24));
    }

    #[test]
    fn sim_config_defaults_keep_legacy_behavior() {
        let config = SimConfig {
            num_robots: 4,
            grid_width: 15,
            grid_height: 15,
            aisle_spacing: 3,
            tasks: vec![],
            max_ticks: 100,
            kill_robot_at: None,
            block_cell_at: None,
            start_positions: vec![],
            use_neural_guidance: false,
            use_learned_bids: false,
            task_seed: None,
            guidance_policy: GuidancePolicy::Off,
        };
        assert!(!config.use_neural_guidance);
        assert_eq!(config.guidance_policy, GuidancePolicy::Off);
        assert!(!config.guidance_policy.engages(
            config.num_robots,
            config.grid_width,
            config.grid_height
        ));
    }
}
