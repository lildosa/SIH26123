use crate::protocol::{RobotId, Tick};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MetricSummary {
    pub total_ticks: Tick,
    pub tasks_completed: usize,
    pub total_collisions: usize,
    pub total_messages_sent: usize,
    pub messages_per_robot: HashMap<RobotId, usize>,
    pub throughput_tasks_per_100_ticks: f64,
    pub avg_task_completion_ticks: f64,
}

pub struct MetricCollector {
    pub start_tick: Tick,
    pub end_tick: Tick,
    pub tasks_completed: usize,
    pub collisions: usize,
    pub message_counts: HashMap<RobotId, usize>,
    pub task_durations: Vec<Tick>,
}

impl MetricCollector {
    pub fn new() -> Self {
        Self {
            start_tick: 0,
            end_tick: 0,
            tasks_completed: 0,
            collisions: 0,
            message_counts: HashMap::new(),
            task_durations: Vec::new(),
        }
    }

    pub fn record_message(&mut self, sender: RobotId) {
        *self.message_counts.entry(sender).or_insert(0) += 1;
    }

    pub fn record_task_completed(&mut self, duration: Tick) {
        self.tasks_completed += 1;
        self.task_durations.push(duration);
    }

    pub fn record_collision(&mut self) {
        self.collisions += 1;
    }

    pub fn summarize(&self, total_ticks: Tick) -> MetricSummary {
        let total_messages: usize = self.message_counts.values().sum();
        let throughput = if total_ticks > 0 {
            (self.tasks_completed as f64 / total_ticks as f64) * 100.0
        } else {
            0.0
        };

        let avg_duration = if !self.task_durations.is_empty() {
            self.task_durations.iter().sum::<u64>() as f64 / self.task_durations.len() as f64
        } else {
            0.0
        };

        MetricSummary {
            total_ticks,
            tasks_completed: self.tasks_completed,
            total_collisions: self.collisions,
            total_messages_sent: total_messages,
            messages_per_robot: self.message_counts.clone(),
            throughput_tasks_per_100_ticks: throughput,
            avg_task_completion_ticks: avg_duration,
        }
    }
}
