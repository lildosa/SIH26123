use sih26123::sim::{SimConfig, SimRunner};
use sih26123::world::{GridMap, Pos};

#[tokio::test]
async fn test_multi_robot_multi_task_zero_collisions() {
    let grid = GridMap::generate_warehouse(15, 15, 3);
    let mut pickups = Vec::new();
    let mut dropoffs = Vec::new();

    for y in 0..15 {
        for x in 0..15 {
            let p = Pos::new(x, y);
            if grid.is_walkable(p) {
                if x < 5 {
                    pickups.push(p);
                } else if x >= 10 {
                    dropoffs.push(p);
                }
            }
        }
    }

    let tasks: Vec<(Pos, Pos)> = (0..5)
        .map(|i| (pickups[i % pickups.len()], dropoffs[(i * 3 + 1) % dropoffs.len()]))
        .collect();

    let mut starts = Vec::new();
    for y in 0..15 {
        for x in 0..15 {
            let p = Pos::new(x, y);
            if grid.is_walkable(p) && starts.len() < 3 && !pickups.contains(&p) {
                starts.push(p);
            }
        }
    }

    let config = SimConfig {
        num_robots: 3,
        grid_width: 15,
        grid_height: 15,
        aisle_spacing: 3,
        tasks,
        max_ticks: 300,
        kill_robot_at: None,
        block_cell_at: None,
        start_positions: starts,
    };

    let mut runner = SimRunner::new(config);
    let result = runner.run().await;

    assert_eq!(result.collisions, 0, "Collisions must be strictly 0");
    assert_eq!(result.vertex_collisions, 0, "Vertex collisions must be 0");
    assert_eq!(result.edge_swap_collisions, 0, "Edge-swap collisions must be 0");
    assert_eq!(result.tasks_completed, 5, "All 5 tasks must be completed");
}

#[tokio::test]
async fn test_dynamic_obstacle_replan() {
    let config = SimConfig {
        num_robots: 2,
        grid_width: 10,
        grid_height: 10,
        aisle_spacing: 3,
        tasks: vec![(Pos::new(0, 0), Pos::new(9, 9))],
        max_ticks: 300,
        kill_robot_at: None,
        block_cell_at: Some((Pos::new(4, 0), 3)), // Block cell on path at tick 3
        start_positions: vec![Pos::new(1, 0), Pos::new(2, 0)],
    };

    let mut runner = SimRunner::new(config);
    let result = runner.run().await;

    assert_eq!(result.collisions, 0, "Zero collisions with dynamic obstacle");
    assert!(result.tasks_completed >= 1, "Task completed despite obstacle");
}

#[tokio::test]
async fn test_robot_failure_task_reassignment() {
    let config = SimConfig {
        num_robots: 3,
        grid_width: 15,
        grid_height: 15,
        aisle_spacing: 3,
        tasks: vec![
            (Pos::new(0, 0), Pos::new(14, 14)),
            (Pos::new(0, 3), Pos::new(14, 3)),
        ],
        max_ticks: 300,
        kill_robot_at: Some((2, 5)), block_cell_at: None, // Kill Robot 2 at tick 5
        start_positions: vec![Pos::new(1, 0), Pos::new(2, 0), Pos::new(3, 0)],
    };

    let mut runner = SimRunner::new(config);
    let result = runner.run().await;

    assert_eq!(result.collisions, 0, "Zero collisions during peer kill");
    assert!(result.tasks_completed >= 1, "Surviving robots complete tasks");
}
