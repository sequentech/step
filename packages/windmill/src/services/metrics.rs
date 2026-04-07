// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Prometheus metrics for windmill.
//!
//! Two distinct metric lifecycles are tracked here:
//!
//! **Celery lifecycle** — fired by the Celery framework callbacks
//! ([`on_task_success`], [`on_task_failure`]) regardless of what the task body
//! does. Captures retries, panics, and max-retries-exceeded.
//! - `windmill_celery_tasks_total` — counter labelled by task type and outcome
//!
//! **DB task lifecycle** — fired by [`PrometheusTaskObserver`] after a DB
//! `tasks_execution` row is successfully written to a terminal state. Captures
//! only tasks that reach the application-level success/failure path.
//! - `windmill_db_tasks_total` — counter labelled by task type and outcome
//! - `windmill_task_enqueue_to_complete_seconds` — histogram of enqueue-to-completion latency
//!
//! These two lifecycles are deliberately separate. A Celery retry increments
//! `windmill_celery_tasks_total[retry]` but does not affect the DB metrics.
//! Operators should consult both sets when diagnosing task health.

use crate::services::tasks_execution::TaskObserver;
use celery::error::TaskError;
use celery::prelude::Task;
use chrono::Local;
use lazy_static::lazy_static;
use prometheus::{
    register_counter_vec, register_gauge_vec, register_histogram_vec, CounterVec, GaugeVec,
    HistogramVec,
};
use sequent_core::types::hasura::core::TasksExecution;
use strum_macros::Display;

/// Outcome label shared by both Celery-level and DB-level task counters.
///
/// `Retry` applies only to the Celery lifecycle; it is never used for
/// [`DB_TASKS_TOTAL`].
#[derive(Display, Debug)]
#[strum(serialize_all = "lowercase")]
pub enum TaskOutcome {
    Success,
    Failure,
    Retry,
}

// ── Celery lifecycle ──────────────────────────────────────────────────────────

const METRIC_CELERY_TASKS_TOTAL: &str = "windmill_celery_tasks_total";

// ── DB task lifecycle ─────────────────────────────────────────────────────────

const METRIC_DB_TASKS_TOTAL: &str = "windmill_db_tasks_total";
const METRIC_TASK_ENQUEUE_TO_COMPLETE_SECONDS: &str = "windmill_task_enqueue_to_complete_seconds";

// ── Queue metrics (probe-sampled) ─────────────────────────────────────────────

const METRIC_QUEUE_MESSAGES: &str = "windmill_queue_messages";
const METRIC_QUEUE_CONSUMERS: &str = "windmill_queue_consumers";

lazy_static! {
    /// Celery-framework task outcomes, labelled by task type and outcome
    /// (success | failure | retry).
    ///
    /// Incremented by [`on_task_success`] / [`on_task_failure`] callbacks.
    /// Captures framework-level events including panics and max-retries-exceeded.
    pub static ref CELERY_TASKS_TOTAL: CounterVec = register_counter_vec!(
        METRIC_CELERY_TASKS_TOTAL,
        "Total Celery task outcomes by type and result (includes retries and panics)",
        &["task_type", "status"]
    )
    .expect("Failed to register windmill_celery_tasks_total metric");

    /// DB-level task outcomes, labelled by task type and outcome
    /// (success | failure).
    ///
    /// Incremented by [`PrometheusTaskObserver`] after the `tasks_execution`
    /// row is written to a terminal state. Retries are not represented here.
    pub static ref DB_TASKS_TOTAL: CounterVec = register_counter_vec!(
        METRIC_DB_TASKS_TOTAL,
        "Total DB task terminal-state transitions by type and result (excludes retries)",
        &["task_type", "status"]
    )
    .expect("Failed to register windmill_db_tasks_total metric");

    /// Elapsed seconds from `tasks_execution.created_at` to terminal state.
    ///
    /// This is enqueue-to-completion latency: `created_at` is set when the
    /// row is first inserted (before the task starts executing), so the
    /// histogram captures queue wait time plus execution time combined.
    pub static ref TASK_ENQUEUE_TO_COMPLETE_SECONDS: HistogramVec = register_histogram_vec!(
        METRIC_TASK_ENQUEUE_TO_COMPLETE_SECONDS,
        "Elapsed seconds from task DB creation to terminal state (enqueue-to-complete latency)",
        &["task_type"],
        // Buckets cover quick ops (1 s) up to long-running tally ceremonies (30 min).
        vec![1.0, 5.0, 15.0, 30.0, 60.0, 120.0, 300.0, 600.0, 1800.0]
    )
    .expect("Failed to register windmill_task_enqueue_to_complete_seconds metric");

    /// Number of messages waiting in each RabbitMQ queue (sampled on each
    /// liveness probe).
    pub static ref QUEUE_MESSAGES: GaugeVec = register_gauge_vec!(
        METRIC_QUEUE_MESSAGES,
        "Number of messages in each RabbitMQ queue",
        &["queue"]
    )
    .expect("Failed to register windmill_queue_messages metric");

    /// Number of active consumers per RabbitMQ queue (sampled on each
    /// liveness probe).
    pub static ref QUEUE_CONSUMERS: GaugeVec = register_gauge_vec!(
        METRIC_QUEUE_CONSUMERS,
        "Number of active consumers per RabbitMQ queue",
        &["queue"]
    )
    .expect("Failed to register windmill_queue_consumers metric");
}

// ── DB task observer (PrometheusTaskObserver) ─────────────────────────────────

/// Prometheus-backed implementation of [`TaskObserver`].
///
/// Records DB-lifecycle metrics after each successful terminal-state write:
/// - Increments [`DB_TASKS_TOTAL`] with the appropriate outcome label.
/// - Observes enqueue-to-completion latency into
///   [`TASK_ENQUEUE_TO_COMPLETE_SECONDS`].
///
/// This is separate from the Celery callbacks ([`on_task_success`] /
/// [`on_task_failure`]), which track Celery-framework outcomes into
/// [`CELERY_TASKS_TOTAL`]. See the module-level docs for the full picture.
pub struct PrometheusTaskObserver;

impl TaskObserver for PrometheusTaskObserver {
    fn on_completed(&self, task: &TasksExecution) {
        record_db_task(&task.task_type, task, TaskOutcome::Success);
    }

    fn on_failed(&self, task: &TasksExecution) {
        record_db_task(&task.task_type, task, TaskOutcome::Failure);
    }
}

fn record_db_task(task_type: &str, task: &TasksExecution, outcome: TaskOutcome) {
    let duration_secs = (Local::now() - task.created_at).num_milliseconds().max(0) as f64 / 1000.0;
    DB_TASKS_TOTAL
        .with_label_values(&[task_type, &outcome.to_string()])
        .inc();
    TASK_ENQUEUE_TO_COMPLETE_SECONDS
        .with_label_values(&[task_type])
        .observe(duration_secs);
}

// ── Celery callbacks ──────────────────────────────────────────────────────────

/// Celery `on_failure` callback — add to every `#[celery::task]` annotation.
///
/// Fired by the Celery framework after any task failure, including panics and
/// max-retries-exceeded, regardless of what the task body does. Retries are
/// recorded separately so operators can distinguish transient noise from
/// permanent failures.
pub async fn on_task_failure<T: Task>(task: &T, err: &TaskError) {
    let outcome = if matches!(err, TaskError::Retry(_)) {
        TaskOutcome::Retry
    } else {
        TaskOutcome::Failure
    };
    CELERY_TASKS_TOTAL
        .with_label_values(&[task.name(), &outcome.to_string()])
        .inc();
}

/// Celery `on_success` callback — add to every `#[celery::task]` annotation.
pub async fn on_task_success<T: Task>(task: &T, _ret: &T::Returns) {
    CELERY_TASKS_TOTAL
        .with_label_values(&[task.name(), &TaskOutcome::Success.to_string()])
        .inc();
}

#[cfg(test)]
mod tests {
    use super::*;
    use prometheus::core::Collector;

    #[test]
    fn test_all_metrics_are_registered() {
        let _ = &*CELERY_TASKS_TOTAL;
        let _ = &*DB_TASKS_TOTAL;
        let _ = &*TASK_ENQUEUE_TO_COMPLETE_SECONDS;
        let _ = &*QUEUE_MESSAGES;
        let _ = &*QUEUE_CONSUMERS;

        assert!(!CELERY_TASKS_TOTAL.desc().is_empty());
        assert!(!DB_TASKS_TOTAL.desc().is_empty());
        assert!(!TASK_ENQUEUE_TO_COMPLETE_SECONDS.desc().is_empty());
        assert!(!QUEUE_MESSAGES.desc().is_empty());
        assert!(!QUEUE_CONSUMERS.desc().is_empty());
    }

    #[test]
    fn test_celery_counter_labels() {
        let _ = &*CELERY_TASKS_TOTAL;
        let success = TaskOutcome::Success.to_string();
        CELERY_TASKS_TOTAL
            .with_label_values(&["IMPORT_USERS", &success])
            .inc();
        assert!(
            CELERY_TASKS_TOTAL
                .with_label_values(&["IMPORT_USERS", &success])
                .get()
                >= 1.0
        );
    }

    #[test]
    fn test_db_counter_labels() {
        let _ = &*DB_TASKS_TOTAL;
        let failure = TaskOutcome::Failure.to_string();
        DB_TASKS_TOTAL
            .with_label_values(&["EXPORT_USERS", &failure])
            .inc();
        assert!(
            DB_TASKS_TOTAL
                .with_label_values(&["EXPORT_USERS", &failure])
                .get()
                >= 1.0
        );
    }
}
