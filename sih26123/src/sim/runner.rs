use crate::auction::{compute_bid_cost, Auction};
use crate::network::InMemoryBus;
use crate::node::environment::Environment;
use crate::node::actor::{RobotActor, RobotTelemetry};
use crate::protocol::{
    AuctionOpenMsg, BidMsg, FleetMessage, RobotId, TaskId, TaskState, TaskStatusMsg, Tick,
};
use crate::world::{Cell, GridMap, Pos};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use tokio::sync::watch;

pub struct SimEnvironment {
    pub ground_truth: RwLock<GridMap>,
}

impl SimEnvironment {
    pub fn new(grid: GridMap) -> Arc<Self> {
        Arc::new(Self {
            ground_truth: RwLock::new(grid),
        })
    }

    pub fn inject_obstacle(&self, pos: Pos) {
        let mut grid = self.ground_truth.write().unwrap();
        if grid.in_bounds(pos) {
            grid.set_cell(pos, Cell::Wall);
        }
    }
}

impl Environment for SimEnvironment {
    fn sense_obstacles(&self, robot_pos: Pos, sense_radius: usize) -> Vec<Pos> {
        let grid = self.ground_truth.read().unwrap();
        let mut obstacles = Vec::new();

        let min_x = robot_pos.x.saturating_sub(sense_radius);
        let max_x = (robot_pos.x + sense_radius).min(grid.width.saturating_sub(1));
        let min_y = robot_pos.y.saturating_sub(sense_radius);
        let max_y = (robot_pos.y + sense_radius).min(grid.height.saturating_sub(1));

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let p = Pos::new(x, y);
                if p.manhattan_distance(&robot_pos) <= sense_radius {
                    if grid.get_cell(p) == Cell::Wall {
                        obstacles.push(p);
                    }
                }
            }
        }

        obstacles
    }

    fn is_blocked(&self, pos: Pos) -> bool {
        let grid = self.ground_truth.read().unwrap();
        if !grid.in_bounds(pos) {
            return true;
        }
        grid.get_cell(pos) == Cell::Wall
    }
}

#[derive(Debug, Clone)]
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
}

pub struct SimRunner {
    pub config: SimConfig,
    pub grid: Arc<GridMap>,
    pub environment: Arc<SimEnvironment>,
    pub bus: Arc<InMemoryBus>,
    pub robots: Vec<RobotActor>,
    pub telemetry_receivers: HashMap<RobotId, watch::Receiver<RobotTelemetry>>,
}

impl SimRunner {
    pub fn new(config: SimConfig) -> Self {
        let grid_map = GridMap::generate_warehouse(
            config.grid_width,
            config.grid_height,
            config.aisle_spacing,
        );
        let grid = Arc::new(grid_map.clone());
        let environment = SimEnvironment::new(grid_map);
        let bus = InMemoryBus::new();

        let mut robots = Vec::new();
        let mut telemetry_receivers = HashMap::new();

        for i in 0..config.num_robots {
            let robot_id = (i + 1) as RobotId;
            let start_pos = config
                .start_positions
                .get(i)
                .cloned()
                .unwrap_or_else(|| Pos::new(0, i * 2));

            let node = bus.register_node(robot_id);
            let actor = RobotActor::new(
                robot_id,
                start_pos,
                grid.clone(),
                Arc::new(node),
                environment.clone(),
            );

            telemetry_receivers.insert(robot_id, actor.telemetry_rx.clone());
            robots.push(actor);
        }

        // Seed initial peer poses so robots are aware of each other's initial location
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

        Self {
            config,
            grid,
            environment,
            bus,
            robots,
            telemetry_receivers,
        }
    }

    /// Executes the 5-phase synchronous simulation loop.
    pub async fn run(&mut self) -> SimResult {
        let mut result = SimResult::default();
        let total_tasks = self.config.tasks.len();

        // 0. Seed tasks into the auction system at tick 0
        for (idx, &(pickup, dropoff)) in self.config.tasks.iter().enumerate() {
            let task_id = (idx + 1) as TaskId;
            let task_msg = TaskStatusMsg {
                task_id,
                pickup,
                dropoff,
                assigned_to: None,
                status: TaskState::Open,
            };
            let auction_open = AuctionOpenMsg {
                task_id,
                pickup,
                dropoff,
                deadline_tick: 3,
            };

            // Seed to robot 1
            if let Some(r1) = self.robots.first_mut() {
                r1.known_tasks.insert(task_id, task_msg.clone());
                let mut auction = Auction::new(task_id, pickup, dropoff, 0, 3);
                let r1_cost = compute_bid_cost(&r1.auction_config, r1.pos, r1.battery, pickup, dropoff, 0.0, 0, Some(3), 0);
                auction.add_bid(1, &BidMsg { task_id, cost: r1_cost });
                r1.pending_auctions.insert(task_id, auction);

                r1.send(FleetMessage::TaskStatus(task_msg));
                r1.send(FleetMessage::AuctionOpen(auction_open));
            }
        }

        let mut completed_tasks: HashSet<TaskId> = HashSet::new();

        for current_tick in 1..=self.config.max_ticks {
            // Dynamic obstacle injection
            if let Some((blocked_pos, at_tick)) = self.config.block_cell_at {
                if current_tick == at_tick {
                    self.environment.inject_obstacle(blocked_pos);
                }
            }

            // Node kill injection
            if let Some((killed_id, at_tick)) = self.config.kill_robot_at {
                if current_tick == at_tick {
                    if let Some(r) = self.robots.iter_mut().find(|r| r.id == killed_id) {
                        r.alive = false;
                        r.state = crate::node::RobotState::Dead;
                    }
                }
            }

            // -------------------------------------------------------------
            // PHASE 1: SENSE (all robots observe environment)
            // -------------------------------------------------------------
            for robot in &mut self.robots {
                robot.sense_phase();
            }

            // -------------------------------------------------------------
            // PHASE 2: DECIDE (process inbox, make decision, populate outbox)
            // -------------------------------------------------------------
            for robot in &mut self.robots {
                let inbox = robot.network.drain().await;
                robot.decide_phase(inbox);
            }

            // -------------------------------------------------------------
            // PHASE 3: FLUSH / DELIVER MESSAGES (deliver tick t messages for tick t+1)
            // -------------------------------------------------------------
            for robot in &mut self.robots {
                for env in robot.outbox.drain(..) {
                    robot.network.broadcast(env).await;
                }
            }
            self.bus.flush_tick();

            // -------------------------------------------------------------
            // PHASE 4: SIMULTANEOUS MOVEMENT (atomic commit)
            // -------------------------------------------------------------
            let mut movements = Vec::new();
            for robot in &mut self.robots {
                if robot.alive {
                    let (prev, next_opt) = robot.commit_movement();
                    let current = next_opt.unwrap_or(prev);
                    movements.push((robot.id, prev, current));
                }
            }

            // -------------------------------------------------------------
            // PHASE 5: EVALUATION (observational collision detection & task completion)
            // -------------------------------------------------------------
            // 5a. Vertex collisions: two robots at same position after movement
            for i in 0..movements.len() {
                for j in (i + 1)..movements.len() {
                    let (_, _, curr_i) = movements[i];
                    let (_, _, curr_j) = movements[j];
                    if curr_i == curr_j {
                        result.vertex_collisions += 1;
                        result.collisions += 1;
                    }
                }
            }

            // 5b. Edge-swap collisions: robot i moved A->B while robot j moved B->A
            for i in 0..movements.len() {
                for j in (i + 1)..movements.len() {
                    let (_, prev_i, curr_i) = movements[i];
                    let (_, prev_j, curr_j) = movements[j];
                    if prev_i == curr_j && prev_j == curr_i && prev_i != curr_i {
                        result.edge_swap_collisions += 1;
                        result.collisions += 1;
                    }
                }
            }

            // 5c. Track completed tasks across all robots
            for robot in &self.robots {
                for (&t_id, status_msg) in &robot.known_tasks {
                    if status_msg.status == TaskState::Completed {
                        completed_tasks.insert(t_id);
                    }
                }
            }

            if completed_tasks.len() >= total_tasks {
                result.makespan = current_tick;
                result.tasks_completed = completed_tasks.len();
                return result;
            }
        }

        result.makespan = self.config.max_ticks;
        result.tasks_completed = completed_tasks.len();
        result
    }
}
