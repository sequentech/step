#![allow(non_upper_case_globals)]
#![feature(result_flattening)]
#![recursion_limit = "256"]
// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use celery::beat::DeltaSchedule;
use dotenv::dotenv;
use sequent_core::util::init_log::init_log;
use structopt::StructOpt;
use tokio::time::Duration;
use windmill::services::celery_app::set_is_app_active;
use windmill::services::probe::{setup_probe, AppName};
use windmill::tasks::review_boards::review_boards;
use windmill::tasks::scheduled_events::scheduled_events;
use windmill::tasks::scheduled_reports::scheduled_reports;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "windmill",
    about = "Run a Rust Celery producer or consumer.",
    setting = structopt::clap::AppSettings::ColoredHelp,
)]
struct CeleryOpt {
    #[structopt(short, long, default_value = "15")]
    interval: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    init_log(true);

    setup_probe(AppName::BEAT).await;

    // Build a `Beat` with a default scheduler backend.
    let mut beat = celery::beat!(
        broker = AMQPBroker { std::env::var("AMQP_ADDR").unwrap_or_else(|_| "amqp://rabbitmq:5672".into()) },
        tasks = [
            "review_boards" => {
                review_boards,
                schedule = DeltaSchedule::new(Duration::from_secs(CeleryOpt::from_args().interval)),
                args = (),
            },
            "scheduled_events" => {
                scheduled_events,
                schedule = DeltaSchedule::new(Duration::from_secs(10)),
                args = (),
            },
            "scheduled_reports" => {
                scheduled_reports,
                schedule = DeltaSchedule::new(Duration::from_secs(10)),
                args = (),
            },
        ],
        task_routes = [
            "review_boards" => "beat",
            "scheduled_events" => "beat",
            "scheduled_reports" => "beat",
        ],
    ).await?;

    set_is_app_active(true);
    beat.start().await?;
    set_is_app_active(false);

    Ok(())
}
