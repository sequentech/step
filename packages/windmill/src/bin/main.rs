#![allow(non_upper_case_globals)]
#![feature(result_flattening)]
// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

extern crate lazy_static;

use anyhow::Result;
use sequent_core::util::init_log::init_log;

use dotenv::dotenv;
use structopt::StructOpt;
use tracing::{event, Level};
use windmill::services::celery_app::*;
extern crate chrono;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "windmill",
    about = "Run a Rust Celery producer or consumer.",
    setting = structopt::clap::AppSettings::ColoredHelp,
)]
enum CeleryOpt {
    Consume {
        #[structopt(short, long, possible_values = &[
            "short_queue", "reports_queue", "tally_queue", "beat", "communication_queue"
        ], default_value = "beat")]
        queues: Vec<String>,
        #[structopt(short, long, default_value = "100")]
        prefetch_count: u16,
        #[structopt(short, long)]
        acks_late: bool,
    },
    Produce,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    init_log(true);
    let opt = CeleryOpt::from_args();

    match opt {
        CeleryOpt::Consume {
            queues,
            prefetch_count,
            acks_late,
        } => {
            set_prefetch_count(prefetch_count);
            set_acks_late(acks_late);
            let celery_app = get_celery_app().await;
            celery_app.display_pretty().await;

            let vec_str: Vec<&str> = queues.iter().map(AsRef::as_ref).collect();

            celery_app.consume_from(&vec_str[..]).await?;
            celery_app.close().await?;
        }
        CeleryOpt::Produce => {
            let celery_app = get_celery_app().await;
            event!(Level::INFO, "Task is empty, not adding any new tasks");
            // Basic task sending.
            /*let task1 = celery_app
                .send_task(add::new(1, 2, get_seconds_later(5)).with_eta(get_seconds_later(100)))
                .await?;
            event!(Level::INFO, "Sent task {}", task1.task_id);

            let task2 = celery_app
                .send_task(add::new(0, 0, get_seconds_later(5)).with_eta(get_seconds_later(5)))
                .await?;
            event!(Level::INFO, "Sent task {}", task2.task_id);*/
            celery_app.close().await?;
        }
    };

    Ok(())
}
