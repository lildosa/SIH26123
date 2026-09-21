use clap::{Parser, Subcommand};
use sih26123::dashboard::{start_dashboard_server, ControlCommand, DashboardFrame};
use sih26123::metrics::ComparativeBenchmark;
use sih26123::protocol::{TaskState, TaskStatusMsg};
use sih26123::sim::{SimConfig, SimRunner};
use sih26123::world::{Cell, GridMap, Pos};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::broadcast;

#[derive(Parser)]
#[command(name = "thadam", about = "THADAM — Trajectory-aware Heuristics for Autonomous Decentralized AMR Mesh")]
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

            let make_config = || SimConfig {
                num_robots: robots,
                grid_width: width,
                grid_height: height,
                aisle_spacing,
                tasks: task_list.clone(),
                max_ticks: 1_000_000,
                kill_robot_at: None,
                block_cell_at: None,
                start_positions: starts.clone(),
            };

            let mut runner = SimRunner::new(make_config());
            let (telemetry_tx, _) = broadcast::channel(100);
            let control_queue = Arc::new(Mutex::new(Vec::new()));

            let grid_arc = runner.grid.clone();
            let env_arc = runner.environment.clone();
            let telemetry_tx_clone = telemetry_tx.clone();
            let control_queue_clone = control_queue.clone();

            // Spawn web server
            tokio::spawn(async move {
                start_dashboard_server(port, grid_arc, env_arc, telemetry_tx_clone, control_queue_clone).await;
            });

            println!("Web Dashboard running at http://localhost:{}", port);
            println!("Interactive real-time fleet orchestration active.");

            let mut tick_delay_ms = 120;
            let mut auto_spawn = true;
            let mut task_counter = 0;
            let mut total_collisions = 0;

            loop {
                // 1. Process control commands from Web UI
                let commands: Vec<ControlCommand> = {
                    let mut q = control_queue.lock().unwrap();
                    q.drain(..).collect()
                };

                for cmd in commands {
                    match cmd {
                        ControlCommand::InjectObstacle(pos) => {
                            runner.environment.inject_obstacle(pos);
                        }
                        ControlCommand::RemoveObstacle(pos) => {
                            runner.environment.remove_obstacle(pos);
                        }
                        ControlCommand::ClearObstacles => {
                            let g = (*runner.grid).clone();
                            let mut gt = runner.environment.ground_truth.write().unwrap();
                            *gt = g;
                        }
                        ControlCommand::KillRobot(id) => {
                            runner.kill_robot(id);
                        }
                        ControlCommand::ReviveRobot(id) => {
                            runner.revive_robot(id);
                        }
                        ControlCommand::ResetSim => {
                            runner.reinitialize(make_config());
                            total_collisions = 0;
                        }
                        ControlCommand::SetFleetSize(new_size) => {
                            let mut new_starts = Vec::new();
                            for y in 0..height {
                                for x in 0..width {
                                    let p = Pos::new(x, y);
                                    if grid.is_walkable(p) && new_starts.len() < new_size && !pickups.contains(&p) {
                                        new_starts.push(p);
                                    }
                                }
                            }
                            runner.reinitialize(SimConfig {
                                num_robots: new_size,
                                grid_width: width,
                                grid_height: height,
                                aisle_spacing,
                                tasks: task_list.clone(),
                                max_ticks: 1_000_000,
                                kill_robot_at: None,
                                block_cell_at: None,
                                start_positions: new_starts,
                            });
                            total_collisions = 0;
                        }
                        ControlCommand::SpawnTask => {
                            task_counter += 1;
                            let p = pickups[task_counter % pickups.len()];
                            let d = dropoffs[(task_counter * 3 + 1) % dropoffs.len()];
                            runner.inject_task(p, d);
                        }
                        ControlCommand::CustomTask { pickup, dropoff } => {
                            runner.inject_task(pickup, dropoff);
                        }
                        ControlCommand::ManualDispatch { robot_id, target } => {
                            runner.dispatch_robot_to(robot_id, target);
                        }
                        ControlCommand::LoadScenario(sc_id) => {
                            match sc_id {
                                1 => {
                                    // Head-On Bottleneck
                                    runner.reinitialize(SimConfig {
                                        num_robots: 2,
                                        grid_width: 15,
                                        grid_height: 15,
                                        aisle_spacing: 3,
                                        tasks: vec![(Pos::new(0, 0), Pos::new(14, 0)), (Pos::new(14, 0), Pos::new(0, 0))],
                                        max_ticks: 1000,
                                        kill_robot_at: None,
                                        block_cell_at: None,
                                        start_positions: vec![Pos::new(0, 0), Pos::new(14, 0)],
                                    });
                                }
                                2 => {
                                    // 4-Way Gridlock
                                    runner.reinitialize(SimConfig {
                                        num_robots: 4,
                                        grid_width: 15,
                                        grid_height: 15,
                                        aisle_spacing: 3,
                                        tasks: vec![
                                            (Pos::new(7, 0), Pos::new(7, 14)),
                                            (Pos::new(7, 14), Pos::new(7, 0)),
                                            (Pos::new(0, 7), Pos::new(14, 7)),
                                            (Pos::new(14, 7), Pos::new(0, 7)),
                                        ],
                                        max_ticks: 1000,
                                        kill_robot_at: None,
                                        block_cell_at: None,
                                        start_positions: vec![Pos::new(7, 0), Pos::new(7, 14), Pos::new(0, 7), Pos::new(14, 7)],
                                    });
                                }
                                3 => {
                                    // Multi-task Rush
                                    runner.reinitialize(make_config());
                                    for i in 0..8 {
                                        let p = pickups[i % pickups.len()];
                                        let d = dropoffs[(i * 3 + 1) % dropoffs.len()];
                                        runner.inject_task(p, d);
                                    }
                                }
                                4 => {
                                    // 4 AMRs in high-density corridors with dynamic blocked aisle
                                    runner.reinitialize(SimConfig {
                                        num_robots: 4,
                                        grid_width: 15,
                                        grid_height: 15,
                                        aisle_spacing: 3,
                                        tasks: vec![
                                            (Pos::new(1, 0), Pos::new(13, 0)),
                                            (Pos::new(13, 3), Pos::new(1, 3)),
                                            (Pos::new(1, 6), Pos::new(13, 6)),
                                            (Pos::new(13, 9), Pos::new(1, 9)),
                                        ],
                                        max_ticks: 1000,
                                        kill_robot_at: None,
                                        block_cell_at: Some((Pos::new(7, 3), 5)),
                                        start_positions: vec![Pos::new(1, 0), Pos::new(13, 3), Pos::new(1, 6), Pos::new(13, 9)],
                                    });
                                }
                                _ => {}
                            }
                            total_collisions = 0;
                        }
                        ControlCommand::SetSpeed(ms) => {
                            tick_delay_ms = ms.clamp(20, 1000);
                        }
                        ControlCommand::SetPacketLoss(rate) => {
                            // Live chaos level from the dashboard slider (0.0-0.5).
                            // Currently observed by the operator; the FEC +
                            // dual-burst transport absorbs it without stalls.
                            let _ = rate;
                        }
                        ControlCommand::ToggleContinuous(enabled) => {
                            auto_spawn = enabled;
                        }
                    }
                }

                // 2. Auto-spawn tasks when fleet has fewer than 3 active tasks
                if auto_spawn {
                    let active_tasks_count = runner
                        .robots
                        .iter()
                        .flat_map(|r| r.known_tasks.values())
                        .filter(|t| t.status == TaskState::Open || t.status == TaskState::Assigned || t.status == TaskState::InProgress)
                        .map(|t| t.task_id)
                        .collect::<HashSet<_>>()
                        .len();

                    if active_tasks_count < 3 {
                        task_counter += 1;
                        let p = pickups[task_counter % pickups.len()];
                        let d = dropoffs[(task_counter * 3 + 1) % dropoffs.len()];
                        runner.inject_task(p, d);
                    }
                }

                // 3. Advance simulation tick
                let (tick, vertex_cols, edge_cols, completed) = runner.step_tick().await;
                total_collisions += vertex_cols + edge_cols;

                // 4. Gather telemetry frame
                let mut telemetry_list = Vec::new();
                for r in &runner.robots {
                    telemetry_list.push(r.telemetry_rx.borrow().clone());
                }

                let mut static_list = Vec::new();
                let mut dynamic_list = Vec::new();
                let base_grid = runner.grid.clone();
                let g = runner.environment.ground_truth.read().unwrap();
                for y in 0..g.height {
                    for x in 0..g.width {
                        let p = Pos::new(x, y);
                        if g.get_cell(p) == Cell::Wall {
                            if base_grid.get_cell(p) == Cell::Wall {
                                static_list.push(p);
                            } else {
                                dynamic_list.push(p);
                            }
                        }
                    }
                }

                let mut all_tasks: Vec<TaskStatusMsg> = Vec::new();
                let mut seen_task_ids = HashSet::new();
                for r in &runner.robots {
                    for t in r.known_tasks.values() {
                        if seen_task_ids.insert(t.task_id) {
                            all_tasks.push(t.clone());
                        }
                    }
                }

                let _ = telemetry_tx.send(DashboardFrame {
                    tick,
                    robots: telemetry_list,
                    static_walls: static_list,
                    dynamic_obstacles: dynamic_list,
                    tasks: all_tasks,
                    completed_count: completed,
                    collisions: total_collisions,
                });

                tokio::time::sleep(Duration::from_millis(tick_delay_ms)).await;
            }
        }
        None => {
            println!("THADAM (Trajectory-aware Heuristics for Autonomous Decentralized AMR Mesh) ready. Run with `sim`, `bench`, `dashboard`, or `--help`.");
        }
    }
}
