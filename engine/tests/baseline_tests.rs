use sih26123::baseline::{
    CentralizedConfig, CentralizedRunner, StopAndWaitConfig, StopAndWaitRunner, cbs_plan,
};
use sih26123::world::{GridMap, Pos};

#[test]
fn test_cbs_two_robots_crossing() {
    let grid = GridMap::generate_warehouse(10, 10, 3);
    // Two robots crossing paths
    let agents = vec![
        (1, Pos::new(0, 0), Pos::new(9, 0)),
        (2, Pos::new(9, 0), Pos::new(0, 0)),
    ];

    let paths = cbs_plan(&grid, &agents, 0).expect("CBS must find valid crossing path");
    assert_eq!(paths.len(), 2);

    // Verify zero collisions in CBS paths
    for t in 0..100 {
        let pos1 = paths[0]
            .iter()
            .find(|&&(_, tick)| tick == t)
            .map(|&(p, _)| p);
        let pos2 = paths[1]
            .iter()
            .find(|&&(_, tick)| tick == t)
            .map(|&(p, _)| p);

        if let (Some(p1), Some(p2)) = (pos1, pos2) {
            assert_ne!(
                p1, p2,
                "CBS paths must have 0 vertex collisions at tick {}",
                t
            );
        }
    }
}

#[test]
fn test_centralized_runner_completes_tasks() {
    let config = CentralizedConfig {
        num_robots: 2,
        grid_width: 10,
        grid_height: 10,
        aisle_spacing: 3,
        tasks: vec![
            (Pos::new(0, 0), Pos::new(9, 0)),
            (Pos::new(0, 9), Pos::new(9, 9)),
        ],
        max_ticks: 200,
        start_positions: vec![Pos::new(1, 0), Pos::new(2, 0)],
    };

    let runner = CentralizedRunner::new(config);
    let result = runner.run();

    assert_eq!(result.collisions, 0);
    assert_eq!(result.tasks_completed, 2);
}

#[test]
fn test_stop_and_wait_runner_zero_collisions_and_completes() {
    // Same warehouse/task geometry the benchmark suite uses: pickups in the
    // left third, dropoffs in the right third -> every path crosses the
    // central aisles, so fleets genuinely overlap.
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
    let tasks: Vec<(Pos, Pos)> = (0..6)
        .map(|i| (pickups[i % pickups.len()], dropoffs[(i * 3 + 1) % dropoffs.len()]))
        .collect();
    let starts = vec![
        Pos::new(1, 1),
        Pos::new(2, 2),
        Pos::new(1, 4),
        Pos::new(2, 5),
    ];

    let config = StopAndWaitConfig {
        num_robots: 4,
        grid_width: 15,
        grid_height: 15,
        aisle_spacing: 3,
        tasks: tasks.clone(),
        max_ticks: 2000,
        start_positions: starts,
    };

    let runner = StopAndWaitRunner::new(config);
    let result = runner.run();

    assert_eq!(result.collisions, 0, "stop-and-wait must never collide");
    assert_eq!(
        result.tasks_completed,
        tasks.len(),
        "stop-and-wait baseline must complete all tasks"
    );
    assert!(
        result.makespan > 0 && result.makespan <= 2000,
        "makespan {} out of range",
        result.makespan
    );
}
