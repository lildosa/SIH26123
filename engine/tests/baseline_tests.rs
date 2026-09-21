use sih26123::baseline::{cbs_plan, CentralizedConfig, CentralizedRunner};
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
        let pos1 = paths[0].iter().find(|&&(_, tick)| tick == t).map(|&(p, _)| p);
        let pos2 = paths[1].iter().find(|&&(_, tick)| tick == t).map(|&(p, _)| p);

        if let (Some(p1), Some(p2)) = (pos1, pos2) {
            assert_ne!(p1, p2, "CBS paths must have 0 vertex collisions at tick {}", t);
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
