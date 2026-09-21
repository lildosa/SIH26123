use sih26123::metrics::{ComparativeBenchmark, MetricCollector};

#[test]
fn test_metric_collector_summary() {
    let mut collector = MetricCollector::new();
    collector.record_message(1);
    collector.record_message(1);
    collector.record_message(2);

    collector.record_task_completed(10);
    collector.record_task_completed(20);

    let summary = collector.summarize(50);
    assert_eq!(summary.tasks_completed, 2);
    assert_eq!(summary.total_messages_sent, 3);
    assert_eq!(summary.avg_task_completion_ticks, 15.0);
    assert_eq!(summary.total_collisions, 0);
}

#[tokio::test]
async fn test_comparative_benchmark_run() {
    let bench = ComparativeBenchmark::new(vec![2, 3], 2, 10, 10);
    let rows = bench.run_benchmark().await;

    assert_eq!(rows.len(), 2);
    for row in rows {
        assert_eq!(row.distributed_collisions, 0);
        assert_eq!(row.centralized_collisions, 0);
        assert!(row.distributed_makespan > 0);
    }
}
