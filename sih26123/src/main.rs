use clap::{Parser, Subcommand};
use sih26123::dashboard::{start_dashboard_server, DashboardFrame};
use sih26123::metrics::ComparativeBenchmark;
use sih26123::sim::{SimConfig, SimRunner};
use sih26123::world::{Cell, GridMap, Pos};
use std::time::Duration;
use tokio::sync::broadcast;

#[derive(Parser)]
#[command(name = "sih26123", about = "SIH26123 Distributed AMR Fleet Coordination Engine")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Run headless multi-robot simulation
    Sim {
        #[arg(long, default_value_t = 3)]
        robots: usize,
        #[arg(long, default_value_t = 15)]
        width: usize,
        #[arg(long, default_value_t = 15)]
        height: usize,
        #[arg(long, default_value_t = 3)]
        aisle_spacing: usize,
        #[arg(long, default_value_t = 5)]
        tasks: usize,
        #[arg(long, default_value_t = 300)]
        max_ticks: u64,
    },
    /// Run comparative benchmark against Centralized CBS Baseline
    Bench {
        #[arg(long, default_value_t = 15)]
        width: usize,
        #[arg(long, default_value_t = 15)]
        height: usize,
        #[arg(long, default_value_t = 5)]
        tasks: usize,
    },
    /// Launch real-time interactive web dashboard
    Dashboard {
        #[arg(long, default_value_t = 3000)]
        port: u16,
        #[arg(long, default_value_t = 4)]
        robots: usize,
        #[arg(long, default_value_t = 15)]
        width: usize,
        #[arg(long, default_value_t = 15)]
        height: usize,
        #[arg(long, default_value_t = 3)]
        aisle_spacing: usize,
        #[arg(long, default_value_t = 6)]
        tasks: usize,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Sim {
            robots,
            width,
            height,
            aisle_spacing,
            tasks,
            max_ticks,
        }) => {
            let grid = GridMap::generate_warehouse(width, height, aisle_spacing);

            let mut pickups = Vec::new();
            let mut dropoffs = Vec::new();

            for y in 0..height {
                for x in 0..width {
                    let p = Pos::new(x, y);
                    if grid.is_walkable(p) {
                        if x < width / 3 {
                            pickups.push(p);
                        } else if x >= (width * 2) / 3 {
                            dropoffs.push(p);
                        }
                    }
                }
            }

            let task_list: Vec<(Pos, Pos)> = (0..tasks)
                .map(|i| {
                    let p = pickups[i % pickups.len()];
                    let d = dropoffs[(i * 3 + 1) % dropoffs.len()];
                    (p, d)
                })
                .collect();

            let mut starts = Vec::new();
            for y in 0..height {
                for x in 0..width {
                    let p = Pos::new(x, y);
                    if grid.is_walkable(p) && starts.len() < robots && !pickups.contains(&p) {
                        starts.push(p);
                    }
                }
            }

            let config = SimConfig {
                num_robots: robots,
                grid_width: width,
                grid_height: height,
                aisle_spacing,
                tasks: task_list,
                max_ticks,
                kill_robot_at: None,
                block_cell_at: None,
                start_positions: starts,
            };

            let mut runner = SimRunner::new(config);
            let result = runner.run().await;
            println!("{}", serde_json::to_string_pretty(&result).unwrap());
        }
        Some(Commands::Bench {
            width,
            height,
            tasks,
        }) => {
            println!("Running comparative benchmark (Distributed vs Centralized CBS)...");
            let bench = ComparativeBenchmark::new(vec![2, 4, 6], tasks, width, height);
            let results = bench.run_benchmark().await;
            println!("{}", serde_json::to_string_pretty(&results).unwrap());
        }
        Some(Commands::Dashboard {
            port,
            robots,
            width,
            height,
            aisle_spacing,
            tasks,
        }) => {
            let grid = GridMap::generate_warehouse(width, height, aisle_spacing);

            let mut pickups = Vec::new();
            let mut dropoffs = Vec::new();

            for y in 0..height {
                for x in 0..width {
                    let p = Pos::new(x, y);
                    if grid.is_walkable(p) {
                        if x < width / 3 {
                            pickups.push(p);
                        } else if x >= (width * 2) / 3 {
                            dropoffs.push(p);
                        }
                    }
                }
            }

            let task_list: Vec<(Pos, Pos)> = (0..tasks)
                .map(|i| {
                    let p = pickups[i % pickups.len()];
                    let d = dropoffs[(i * 3 + 1) % dropoffs.len()];
                    (p, d)
                })
                .collect();

            let mut starts = Vec::new();
            for y in 0..height {
                for x in 0..width {
                    let p = Pos::new(x, y);
                    if grid.is_walkable(p) && starts.len() < robots && !pickups.contains(&p) {
                        starts.push(p);
                    }
                }
            }

            let config = SimConfig {
                num_robots: robots,
                grid_width: width,
                grid_height: height,
                aisle_spacing,
                tasks: task_list.clone(),
                max_ticks: 1000,
                kill_robot_at: None,
                block_cell_at: None,
                start_positions: starts.clone(),
            };

            let mut runner = SimRunner::new(config);
            let (telemetry_tx, _) = broadcast::channel(100);

            let grid_arc = runner.grid.clone();
            let env_arc = runner.environment.clone();
            let telemetry_tx_clone = telemetry_tx.clone();

            // Spawn web server
            tokio::spawn(async move {
                start_dashboard_server(port, grid_arc, env_arc, telemetry_tx_clone).await;
            });

            println!("Web Dashboard running at http://localhost:{}", port);
            println!("Streaming real-time animation...");

            loop {
                let (tick, _, _, _) = runner.step_tick().await;

                let mut telemetry_list = Vec::new();
                for r in &runner.robots {
                    telemetry_list.push(r.telemetry_rx.borrow().clone());
                }

                let mut obs_list = Vec::new();
                let g = runner.environment.ground_truth.read().unwrap();
                for y in 0..g.height {
                    for x in 0..g.width {
                        let p = Pos::new(x, y);
                        if g.get_cell(p) == Cell::Wall {
                            obs_list.push(p);
                        }
                    }
                }

                let _ = telemetry_tx.send(DashboardFrame {
                    tick,
                    robots: telemetry_list,
                    obstacles: obs_list,
                });

                tokio::time::sleep(Duration::from_millis(150)).await;
            }
        }
        None => {
            println!("SIH26123 AMR Fleet Coordination Engine ready. Run with `sim`, `bench`, `dashboard`, or `--help`.");
        }
    }
}
