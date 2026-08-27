use sih26123::metrics::ComparativeBenchmark;

#[tokio::test]
async fn test_full_system_validation_and_safety_guarantees() {
    let bench = ComparativeBenchmark::new(vec![2, 3], 4, 12, 12);
    let results = bench.run_benchmark().await;

    assert_eq!(results.len(), 2);
    for row in results {
        assert_eq!(
            row.distributed_collisions, 0,
            "Distributed engine must maintain ZERO collisions across all scales"
        );
        assert_eq!(
            row.centralized_collisions, 0,
            "Centralized baseline must maintain ZERO collisions"
        );
        assert!(
            row.distributed_makespan > 0,
            "Distributed makespan must be non-zero"
        );
    }
}
