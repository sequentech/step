#![allow(non_upper_case_globals)]
#![feature(result_flattening)]
#![recursion_limit = "256"]
// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use celery::beat::{CronSchedule, DeltaSchedule};
use dotenv::dotenv;
use sequent_core::services::probe::ProbeHandler;
use structopt::StructOpt;
use tokio::time::Duration;
use windmill::tasks::scheduled_events::scheduled_events;
use windmill::tasks::{review_boards::review_boards, start_stop_election::start_stop_election};

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

    setup_probe();

    // Build a `Beat` with a default scheduler backend.
    let mut beat = celery::beat!(
        broker = AMQPBroker { std::env::var("AMQP_ADDR").unwrap_or_else(|_| "amqp://rabbitmq:5672".into()) },
        tasks = [
            /*"review_boards" => {
                review_boards,
                schedule = DeltaSchedule::new(Duration::from_secs(CeleryOpt::from_args().interval)),
                args = (),
            },*/
            "scheduled_events" => {
                scheduled_events,
                schedule = DeltaSchedule::new(Duration::from_secs(30)),
                args = (),
            }
        ],
        task_routes = [
            "review_boards" => "beat",
            "scheduled_events" => "beat",
        ],
    ).await?;

    beat.start().await?;

    Ok(())
}

fn setup_probe() {
    let addr_s = std::env::var("BEAT_PROBE_ADDR").unwrap_or("0.0.0.0:3030".to_string());
    let live_path = std::env::var("BEAT_PROBE_LIVE_PATH").unwrap_or("live".to_string());
    let ready_path = std::env::var("BEAT_PROBE_READY_PATH").unwrap_or("ready".to_string());

    let addr: Result<std::net::SocketAddr, _> = addr_s.parse();

    if let Ok(addr) = addr {
        let mut ph = ProbeHandler::new(&live_path, &ready_path, addr);
        let f = ph.future();
        ph.set_live(move || true);
        tokio::spawn(f);
    }
}
