use sih26123::sim::{SimConfig, SimRunner};
use sih26123::world::{Cell, GridMap, Pos};

#[tokio::test]
async fn test_scenario_narrow_corridor_chokepoint() {
    let mut grid = GridMap::new(10, 5);
    // Create horizontal walls creating a 1-cell narrow corridor at y=2
    for x in 2..8 {
        grid.set_cell(Pos::new(x, 1), Cell::Wall);
        grid.set_cell(Pos::new(x, 3), Cell::Wall);
    }

    let config = SimConfig {
        num_robots: 2,
        grid_width: 10,
        grid_height: 5,
        aisle_spacing: 2,
        tasks: vec![
            (Pos::new(0, 2), Pos::new(9, 2)),
            (Pos::new(9, 2), Pos::new(0, 2)),
        ],
        max_ticks: 300,
        kill_robot_at: None,
        block_cell_at: None,
        start_positions: vec![Pos::new(0, 0), Pos::new(9, 4)],
    };

    let mut runner = SimRunner::new(config);
    let result = runner.run().await;

    assert_eq!(result.collisions, 0, "Chokepoint must be navigated with 0 collisions");
    assert_eq!(result.vertex_collisions, 0);
    assert_eq!(result.edge_swap_collisions, 0);
    assert!(result.tasks_completed >= 1, "At least 1 task completed in chokepoint");
}

#[tokio::test]
async fn test_scenario_blocked_aisle_replan() {
    let config = SimConfig {
        num_robots: 2,
        grid_width: 12,
        grid_height: 12,
        aisle_spacing: 3,
        tasks: vec![(Pos::new(0, 0), Pos::new(11, 11))],
        max_ticks: 200,
        kill_robot_at: None,
        block_cell_at: Some((Pos::new(3, 0), 4)), // Injects obstacle in front of path at tick 4
        start_positions: vec![Pos::new(1, 0), Pos::new(1, 1)],
    };

    let mut runner = SimRunner::new(config);
    let result = runner.run().await;

    assert_eq!(result.collisions, 0, "Blocked aisle replanned with zero collisions");
    assert!(result.tasks_completed >= 1, "Completed task via alternative route");
}

#[tokio::test]
async fn test_scenario_robot_breakdown_task_recovery() {
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
        kill_robot_at: Some((2, 6)), // Kill Robot 2 at tick 6
        block_cell_at: None,
        start_positions: vec![Pos::new(1, 0), Pos::new(2, 0), Pos::new(3, 0)],
    };

    let mut runner = SimRunner::new(config);
    let result = runner.run().await;

    assert_eq!(result.collisions, 0, "Breakdown handled with zero collisions");
    assert!(result.tasks_completed >= 1, "Task reallocated and completed by surviving robots");
}
