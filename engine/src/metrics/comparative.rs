use crate::baseline::{CentralizedConfig, CentralizedRunner, StopAndWaitConfig, StopAndWaitRunner};
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
    /// Stop-and-wait baseline (SIH reference behavior) makespan for the same
    /// scenario, and the fractional completion-time reduction of the
    /// distributed engine over it ("Thadam makespan - SAW makespan" relative
    /// to SAW; positive = Thadam faster).
    #[serde(default)]
    pub stop_wait_makespan: u64,
    #[serde(default)]
    pub stop_wait_collisions: usize,
    #[serde(default)]
    pub stop_wait_reduction: f64,
    /// Open-set expansions across all distributed-space-time-A* plans.
    #[serde(default)]
    pub distributed_plan_expansions: usize,
    /// Expansions from plans that consulted the guidance heatmap.
    #[serde(default)]
    pub guided_plan_expansions: usize,
    /// Adaptive (LinUCB) bids placed across the fleet (0 when disabled).
    #[serde(default)]
    pub learned_bids: usize,
}

pub struct ComparativeBenchmark {
    pub scales: Vec<usize>,
    pub num_tasks: usize,
    pub grid_width: usize,
    pub grid_height: usize,
    /// Opt-in neural A* guidance for the distributed arm.
    pub use_neural_guidance: bool,
    /// Opt-in LinUCB adaptive bidding for the distributed arm.
    pub use_learned_bids: bool,
    /// Hybrid guidance routing for the distributed arm.
    pub guidance_policy: crate::sim::GuidancePolicy,
}

impl ComparativeBenchmark {
    pub fn new(scales: Vec<usize>, num_tasks: usize, width: usize, height: usize) -> Self {
        Self {
            scales,
            num_tasks,
            grid_width: width,
            grid_height: height,
            use_neural_guidance: false,
            use_learned_bids: false,
            guidance_policy: crate::sim::GuidancePolicy::default(),
        }
    }

    /// Benchmarks with neural guidance enabled for the distributed arm.
    /// The model is embedded in the binary; missing/corrupt assets only
    /// disable guidance (runs fall back to pure kinematic planning).
    pub fn with_neural_guidance(
        scales: Vec<usize>,
        num_tasks: usize,
        width: usize,
        height: usize,
    ) -> Self {
        Self {
            scales,
            num_tasks,
            grid_width: width,
            grid_height: height,
            use_neural_guidance: true,
            use_learned_bids: false,
            guidance_policy: crate::sim::GuidancePolicy::default(),
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
            .map(|i| {
                (
                    pickups[i % pickups.len()],
                    dropoffs[(i * 3 + 1) % dropoffs.len()],
                )
            })
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
                        if grid.is_walkable(p)
                            && starts.len() < n_robots
                            && !pickups.contains(&p)
                            && !starts.contains(&p)
                        {
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
                max_ticks: 2000,
                kill_robot_at: None,
                block_cell_at: None,
                start_positions: starts.clone(),
                use_neural_guidance: self.use_neural_guidance,
                use_learned_bids: self.use_learned_bids,
                task_seed: None,
                guidance_policy: self.guidance_policy,
            };
            let mut dist_runner = SimRunner::new(dist_config);
            let dist_res = dist_runner.run().await;

            // 2. Run Stop-and-Wait Baseline (SIH reference behavior)
            let saw_config = StopAndWaitConfig {
                num_robots: n_robots,
                grid_width: self.grid_width,
                grid_height: self.grid_height,
                aisle_spacing: 3,
                tasks: tasks.clone(),
                max_ticks: 2000,
                start_positions: starts.clone(),
            };
            let saw_runner = StopAndWaitRunner::new(saw_config);
            let saw_res = saw_runner.run();

            // 3. Run Centralized Baseline
            let cent_config = CentralizedConfig {
                num_robots: n_robots,
                grid_width: self.grid_width,
                grid_height: self.grid_height,
                aisle_spacing: 3,
                tasks: tasks.clone(),
                max_ticks: 2000,
                start_positions: starts,
            };
            let cent_runner = CentralizedRunner::new(cent_config);
            let cent_res = cent_runner.run();

            let ratio = if cent_res.makespan > 0 {
                cent_res.makespan as f64 / dist_res.makespan.max(1) as f64
            } else {
                1.0
            };

            let saw_reduction = if saw_res.makespan > 0 {
                (saw_res.makespan as f64 - dist_res.makespan.max(1) as f64)
                    / saw_res.makespan as f64
            } else {
                0.0
            };

            results.push(BenchmarkRow {
                num_robots: n_robots,
                num_tasks: self.num_tasks,
                distributed_makespan: dist_res.makespan,
                distributed_collisions: dist_res.collisions,
                centralized_makespan: cent_res.makespan,
                centralized_collisions: cent_res.collisions,
                stop_wait_makespan: saw_res.makespan,
                stop_wait_collisions: saw_res.collisions,
                stop_wait_reduction: saw_reduction,
                distributed_throughput_ratio: ratio,
                distributed_plan_expansions: dist_res.plan_expansions,
                guided_plan_expansions: dist_res.guided_plan_expansions,
                learned_bids: dist_res.learned_bids,
            });
        }

        results
    }
}
