use clap::{Parser, Subcommand};
use sih26123::sim::{SimConfig, SimRunner};
use sih26123::world::{GridMap, Pos};

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

            // Find valid distinct start positions
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
        None => {
            println!("SIH26123 AMR Fleet Coordination Engine ready. Run with `sim` subcommand or `--help`.");
        }
    }
}
