#![allow(non_upper_case_globals)]
#![feature(result_flattening)]
#![recursion_limit = "256"]
// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{Context, Result};
use celery::beat::DeltaSchedule;
use celery::prelude::Task;
use dotenv::dotenv;
use sequent_core::util::init_log::init_log;
use structopt::StructOpt;
use tokio::time::Duration;
use windmill::services::celery_app::{set_is_app_active, Queue};
use windmill::services::probe::{setup_probe, AppName};
use windmill::tasks::electoral_log::electoral_log_batch_dispatcher;
use windmill::tasks::review_boards::review_boards;
use windmill::tasks::scheduled_events::scheduled_events;
use windmill::tasks::scheduled_reports::scheduled_reports;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "beat",
    about = "Windmill's periodic task scheduler.",
    setting = structopt::clap::AppSettings::ColoredHelp,
)]
struct CeleryOpt {
    #[structopt(short = "r", long, default_value = "15")]
    review_boards_interval: u64,
    #[structopt(short = "s", long, default_value = "10")]
    schedule_events_interval: u64,
    #[structopt(short = "c", long, default_value = "10")]
    schedule_reports_interval: u64,
    #[structopt(short = "e", long, default_value = "5")]
    electoral_log_interval: u64,
}

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
                schedule = DeltaSchedule::new(Duration::from_secs(CeleryOpt::from_args().review_boards_interval)),
                args = (),
            },
            scheduled_events::NAME => {
                scheduled_events,
                schedule = DeltaSchedule::new(Duration::from_secs(CeleryOpt::from_args().schedule_events_interval)),
                args = (CeleryOpt::from_args().schedule_events_interval),
            },
            scheduled_reports::NAME => {
                scheduled_reports,
                schedule = DeltaSchedule::new(Duration::from_secs(CeleryOpt::from_args().schedule_reports_interval)),
                args = (CeleryOpt::from_args().schedule_reports_interval),
            },
            electoral_log_batch_dispatcher::NAME => {
                electoral_log_batch_dispatcher,
                schedule = DeltaSchedule::new(Duration::from_secs(CeleryOpt::from_args().electoral_log_interval)),
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
