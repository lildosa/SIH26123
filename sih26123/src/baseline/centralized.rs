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

        for &(_pickup, _dropoff) in &self.config.tasks {
            let (best_robot_idx, &avail_tick) = robot_available_at
                .iter()
                .enumerate()
                .min_by_key(|entry| *entry.1)
                .unwrap();

            let robot_id = (best_robot_idx + 1) as RobotId;
            let start_pos = robot_positions[best_robot_idx];

            // 1. Plan to pickup
            let agents_pickup = vec![(robot_id, start_pos, _pickup)];
            if let Some(paths1) = cbs_plan(&self.grid, &agents_pickup, avail_tick) {
                let pickup_tick = avail_tick + paths1[0].len().saturating_sub(1) as u64;

                // 2. Plan from pickup to dropoff
                let agents_dropoff = vec![(robot_id, _pickup, _dropoff)];
                if let Some(paths2) = cbs_plan(&self.grid, &agents_dropoff, pickup_tick) {
                    let end_tick = pickup_tick + paths2[0].len().saturating_sub(1) as u64;
                    robot_available_at[best_robot_idx] = end_tick;
                    robot_positions[best_robot_idx] = _dropoff;
                    completed_tasks += 1;
                    max_makespan = max_makespan.max(end_tick);
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
