use crate::baseline::cbs::cbs_plan;
use crate::protocol::{RobotId, Tick};
use crate::sim::SimResult;
use crate::world::{GridMap, Pos};

pub struct CentralizedConfig {
    pub num_robots: usize,
    pub grid_width: usize,
    pub grid_height: usize,
    pub aisle_spacing: usize,
    pub tasks: Vec<(Pos, Pos)>,
    pub max_ticks: Tick,
    pub start_positions: Vec<Pos>,
}

/// Centralized coordinator executing FIFO task allocation and global CBS planning.
pub struct CentralizedRunner {
    pub config: CentralizedConfig,
    pub grid: GridMap,
}

impl CentralizedRunner {
    pub fn new(config: CentralizedConfig) -> Self {
        let grid = GridMap::generate_warehouse(
            config.grid_width,
            config.grid_height,
            config.aisle_spacing,
        );
        Self { config, grid }
    }

    pub fn run(&self) -> SimResult {
        let mut result = SimResult::default();
        let mut robot_positions = self.config.start_positions.clone();
        let mut robot_available_at: Vec<Tick> = vec![0; self.config.num_robots];

        let mut completed_tasks = 0;
        let mut max_makespan: Tick = 0;
        let mut task_idx = 0;

        while task_idx < self.config.tasks.len() {
            let min_tick = *robot_available_at.iter().min().unwrap_or(&0);
            let mut batch_agents_pickup = Vec::new();
            let mut assigned_robots = Vec::new();
            let mut task_pairs = Vec::new();

            for r_idx in 0..self.config.num_robots {
                if robot_available_at[r_idx] <= min_tick && task_idx < self.config.tasks.len() {
                    let robot_id = (r_idx + 1) as RobotId;
                    let start_pos = robot_positions[r_idx];
                    let (pickup, dropoff) = self.config.tasks[task_idx];
                    batch_agents_pickup.push((robot_id, start_pos, pickup));
                    assigned_robots.push(r_idx);
                    task_pairs.push((pickup, dropoff));
                    task_idx += 1;
                }
            }

            if batch_agents_pickup.is_empty() {
                break;
            }

            // Multi-agent CBS across all concurrent active agents
            if let Some(pickup_paths) = cbs_plan(&self.grid, &batch_agents_pickup, min_tick) {
                let mut batch_agents_dropoff = Vec::new();
                let mut dropoff_start_tick = min_tick;

                for (i, p_path) in pickup_paths.iter().enumerate() {
                    let r_idx = assigned_robots[i];
                    let robot_id = (r_idx + 1) as RobotId;
                    let p_tick = min_tick + p_path.len().saturating_sub(1) as u64;
                    dropoff_start_tick = dropoff_start_tick.max(p_tick);
                    let (pickup, dropoff) = task_pairs[i];
                    batch_agents_dropoff.push((robot_id, pickup, dropoff));
                }

                if let Some(dropoff_paths) =
                    cbs_plan(&self.grid, &batch_agents_dropoff, dropoff_start_tick)
                {
                    for (i, d_path) in dropoff_paths.iter().enumerate() {
                        let r_idx = assigned_robots[i];
                        let end_tick = dropoff_start_tick + d_path.len().saturating_sub(1) as u64;
                        robot_available_at[r_idx] = end_tick;
                        robot_positions[r_idx] = task_pairs[i].1;
                        completed_tasks += 1;
                        max_makespan = max_makespan.max(end_tick);
                    }
                } else {
                    for (i, &(robot_id, pickup, dropoff)) in batch_agents_dropoff.iter().enumerate()
                    {
                        let r_idx = assigned_robots[i];
                        let p_tick = min_tick + pickup_paths[i].len().saturating_sub(1) as u64;
                        if let Some(single_path) =
                            cbs_plan(&self.grid, &[(robot_id, pickup, dropoff)], p_tick)
                        {
                            let end_tick = p_tick + single_path[0].len().saturating_sub(1) as u64;
                            robot_available_at[r_idx] = end_tick;
                            robot_positions[r_idx] = dropoff;
                            completed_tasks += 1;
                            max_makespan = max_makespan.max(end_tick);
                        }
                    }
                }
            } else {
                for (i, &(robot_id, start_pos, pickup)) in batch_agents_pickup.iter().enumerate() {
                    let r_idx = assigned_robots[i];
                    let (_pickup, dropoff) = task_pairs[i];
                    if let Some(paths1) =
                        cbs_plan(&self.grid, &[(robot_id, start_pos, pickup)], min_tick)
                    {
                        let p_tick = min_tick + paths1[0].len().saturating_sub(1) as u64;
                        if let Some(paths2) =
                            cbs_plan(&self.grid, &[(robot_id, pickup, dropoff)], p_tick)
                        {
                            let end_tick = p_tick + paths2[0].len().saturating_sub(1) as u64;
                            robot_available_at[r_idx] = end_tick;
                            robot_positions[r_idx] = dropoff;
                            completed_tasks += 1;
                            max_makespan = max_makespan.max(end_tick);
                        }
                    }
                }
            }
        }

        result.makespan = max_makespan;
        result.tasks_completed = completed_tasks;
        result.collisions = 0;
        result.vertex_collisions = 0;
        result.edge_swap_collisions = 0;

        result
    }
}
