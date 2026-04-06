// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use celery::error::TaskError;
use celery::prelude::Task;
use lazy_static::lazy_static;
use prometheus::{
    register_counter_vec, register_gauge_vec, register_histogram_vec, CounterVec, GaugeVec,
    HistogramVec,
};

lazy_static! {
    /// Total tasks processed, labelled by task type and outcome
    /// (success | failure | retry).
    pub static ref TASKS_TOTAL: CounterVec = register_counter_vec!(
        "windmill_tasks_total",
        "Total number of tasks processed by windmill",
        &["task_type", "status"]
    )
    .unwrap();

    /// Wall-clock duration from task enqueue to completion, in seconds.
    pub static ref TASK_DURATION_SECONDS: HistogramVec = register_histogram_vec!(
        "windmill_task_duration_seconds",
        "Task execution duration in seconds",
        &["task_type"],
        // Buckets cover quick ops (1 s) up to long-running tally ceremonies (30 min).
        vec![1.0, 5.0, 15.0, 30.0, 60.0, 120.0, 300.0, 600.0, 1800.0]
    )
    .unwrap();

    /// Number of messages waiting in each RabbitMQ queue (sampled on each
    /// liveness probe).
    pub static ref QUEUE_MESSAGES: GaugeVec = register_gauge_vec!(
        "windmill_queue_messages",
        "Number of messages in each RabbitMQ queue",
        &["queue"]
    )
    .unwrap();

    /// Number of active consumers per RabbitMQ queue (sampled on each
    /// liveness probe).
    pub static ref QUEUE_CONSUMERS: GaugeVec = register_gauge_vec!(
        "windmill_queue_consumers",
        "Number of active consumers per RabbitMQ queue",
        &["queue"]
    )
    .unwrap();
}

/// Celery `on_failure` callback — add to every `#[celery::task]` annotation.
/// Fired by the celery worker framework after any task failure, including panics
/// and max-retries-exceeded, regardless of what the task body does.
/// Retries are recorded separately so operators can distinguish transient noise
/// from permanent failures.
pub async fn on_task_failure<T: Task>(task: &T, err: &TaskError) {
    let status = if matches!(err, TaskError::Retry(_)) {
        "retry"
    } else {
        "failure"
    };
    TASKS_TOTAL.with_label_values(&[task.name(), status]).inc();
}

/// Celery `on_success` callback — add to every `#[celery::task]` annotation.
pub async fn on_task_success<T: Task>(task: &T, _ret: &T::Returns) {
    TASKS_TOTAL
        .with_label_values(&[task.name(), "success"])
        .inc();
}

#[cfg(test)]
mod tests {
    use super::*;
    use prometheus::core::Collector;

    #[test]
    fn test_all_metrics_are_registered() {
        let _ = &*TASKS_TOTAL;
        let _ = &*TASK_DURATION_SECONDS;
        let _ = &*QUEUE_MESSAGES;
        let _ = &*QUEUE_CONSUMERS;

        assert!(!TASKS_TOTAL.desc().is_empty());
        assert!(!TASK_DURATION_SECONDS.desc().is_empty());
        assert!(!QUEUE_MESSAGES.desc().is_empty());
        assert!(!QUEUE_CONSUMERS.desc().is_empty());
    }

    #[test]
    fn test_task_counter_labels() {
        let _ = &*TASKS_TOTAL;
        TASKS_TOTAL
            .with_label_values(&["IMPORT_USERS", "success"])
            .inc();
        assert!(
            TASKS_TOTAL
                .with_label_values(&["IMPORT_USERS", "success"])
                .get()
                >= 1.0
        );
    }
}
