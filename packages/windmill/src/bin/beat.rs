#![allow(non_upper_case_globals)]
#![recursion_limit = "256"]
//! Celery Beat process for Windmill: registers periodic tasks and publishes them to `RabbitMQ`.
// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{Context, Result};
use celery::beat::DeltaSchedule;
use celery::prelude::Task;
use clap::Parser;
use dotenv::dotenv;
use sequent_core::util::init_log::init_log;
use tokio::time::Duration;
use windmill::services::celery_app::{set_is_app_active, Queue};
use windmill::services::probe::{setup_probe, AppName};
use windmill::tasks::electoral_log::electoral_log_batch_dispatcher;
use windmill::tasks::review_boards::review_boards;
use windmill::tasks::scheduled_events::scheduled_events;
use windmill::tasks::scheduled_reports::scheduled_reports;

/// Beat tick intervals for periodic tasks (all values are in seconds).
#[derive(Debug, Parser)]
#[command(name = "beat", about = "Windmill's periodic task scheduler.")]
struct CeleryOpt {
    /// Interval between `review_boards` dispatches (seconds).
    #[arg(short = 'r', long = "review-boards-interval", default_value = "15")]
    review_boards: u64,
    /// Interval between `scheduled_events` dispatches (seconds).
    #[arg(short = 's', long = "schedule-events-interval", default_value = "10")]
    schedule_events: u64,
    /// Interval between `scheduled_reports` dispatches (seconds).
    #[arg(short = 'c', long = "schedule-reports-interval", default_value = "60")]
    schedule_reports: u64,
    /// Interval between `electoral_log_batch_dispatcher` dispatches (seconds).
    #[arg(short = 'e', long = "electoral-log-interval", default_value = "5")]
    electoral_log: u64,
}

/// Starts the beat scheduler: loads env, wires periodic tasks, and blocks until shutdown.
#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    init_log(true);
    setup_probe(AppName::BEAT).await;
    let slug = std::env::var("ENV_SLUG").with_context(|| "missing env var ENV_SLUG")?;

    let mut beat = celery::beat!(
        broker = AMQPBroker { std::env::var("AMQP_ADDR").unwrap_or_else(|_| "amqp://rabbitmq:5672".into()) },
        tasks = [
            review_boards::NAME => {
                review_boards,
                schedule = DeltaSchedule::new(Duration::from_secs(CeleryOpt::parse().review_boards)),
                args = (),
            },
            scheduled_events::NAME => {
                scheduled_events,
                schedule = DeltaSchedule::new(Duration::from_secs(CeleryOpt::parse().schedule_events)),
                args = (CeleryOpt::parse().schedule_events),
            },
            scheduled_reports::NAME => {
                scheduled_reports,
                schedule = DeltaSchedule::new(Duration::from_secs(CeleryOpt::parse().schedule_reports)),
                args = (CeleryOpt::parse().schedule_events),
            },
            electoral_log_batch_dispatcher::NAME => {
                electoral_log_batch_dispatcher,
                schedule = DeltaSchedule::new(Duration::from_secs(CeleryOpt::parse().electoral_log)),
                args = (),
            },
        ],
        task_routes = [
            review_boards::NAME => &Queue::Beat.queue_name(&slug),
            scheduled_events::NAME => &Queue::Beat.queue_name(&slug),
            scheduled_reports::NAME => &Queue::Beat.queue_name(&slug),
            electoral_log_batch_dispatcher::NAME => &Queue::ElectoralLogBeat.queue_name(&slug),
        ],
    ).await?;

    set_is_app_active(true);
    beat.start().await?;
    set_is_app_active(false);
    Ok(())
}
