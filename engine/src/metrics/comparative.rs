use crate::baseline::{CentralizedConfig, CentralizedRunner};
use crate::sim::{SimConfig, SimRunner};
use crate::world::{GridMap, Pos};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkRow {
    pub num_robots: usize,
    pub num_tasks: usize,
    pub distributed_makespan: u64,
    pub distributed_collisions: usize,
    pub centralized_makespan: u64,
    pub centralized_collisions: usize,
    pub distributed_throughput_ratio: f64,
}

pub struct ComparativeBenchmark {
    pub scales: Vec<usize>,
    pub num_tasks: usize,
    pub grid_width: usize,
    pub grid_height: usize,
}

impl ComparativeBenchmark {
    pub fn new(scales: Vec<usize>, num_tasks: usize, width: usize, height: usize) -> Self {
        Self {
            scales,
            num_tasks,
            grid_width: width,
            grid_height: height,
        }
    }

    pub async fn run_benchmark(&self) -> Vec<BenchmarkRow> {
        let mut results = Vec::new();
        let grid = GridMap::generate_warehouse(self.grid_width, self.grid_height, 3);

        let mut pickups = Vec::new();
        let mut dropoffs = Vec::new();
        for y in 0..self.grid_height {
            for x in 0..self.grid_width {
                let p = Pos::new(x, y);
                if grid.is_walkable(p) {
                    if x < self.grid_width / 3 {
                        pickups.push(p);
                    } else if x >= (self.grid_width * 2) / 3 {
                        dropoffs.push(p);
                    }
                }
            }
        }

        let tasks: Vec<(Pos, Pos)> = (0..self.num_tasks)
            .map(|i| (pickups[i % pickups.len()], dropoffs[(i * 3 + 1) % dropoffs.len()]))
            .collect();

        for &n_robots in &self.scales {
            let mut starts = Vec::new();
            for y in (0..self.grid_height).step_by(3) {
                for x in (0..self.grid_width).step_by(3) {
                    let p = Pos::new(x, y);
                    if grid.is_walkable(p) && starts.len() < n_robots && !pickups.contains(&p) {
                        starts.push(p);
                    }
                }
            }
            if starts.len() < n_robots {
                for y in 0..self.grid_height {
                    for x in 0..self.grid_width {
                        let p = Pos::new(x, y);
                        if grid.is_walkable(p) && starts.len() < n_robots && !pickups.contains(&p) && !starts.contains(&p) {
                            starts.push(p);
                        }
                    }
                }
            }

            // 1. Run Distributed
            let dist_config = SimConfig {
                num_robots: n_robots,
                grid_width: self.grid_width,
                grid_height: self.grid_height,
                aisle_spacing: 3,
                tasks: tasks.clone(),
                max_ticks: 300,
                kill_robot_at: None,
                block_cell_at: None,
                start_positions: starts.clone(),
            };
            let mut dist_runner = SimRunner::new(dist_config);
            let dist_res = dist_runner.run().await;

            // 2. Run Centralized Baseline
            let cent_config = CentralizedConfig {
                num_robots: n_robots,
                grid_width: self.grid_width,
                grid_height: self.grid_height,
                aisle_spacing: 3,
                tasks: tasks.clone(),
                max_ticks: 300,
                start_positions: starts,
            };
            let cent_runner = CentralizedRunner::new(cent_config);
            let cent_res = cent_runner.run();

            let ratio = if cent_res.makespan > 0 {
                cent_res.makespan as f64 / dist_res.makespan.max(1) as f64
            } else {
                1.0
            };

            results.push(BenchmarkRow {
                num_robots: n_robots,
                num_tasks: self.num_tasks,
                distributed_makespan: dist_res.makespan,
                distributed_collisions: dist_res.collisions,
                centralized_makespan: cent_res.makespan,
                centralized_collisions: cent_res.collisions,
                distributed_throughput_ratio: ratio,
            });
        }

        results
    }
}
