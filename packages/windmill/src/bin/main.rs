#![allow(non_upper_case_globals)]
// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

#[macro_use]
extern crate lazy_static;

use anyhow::Result;
use async_trait::async_trait;
use braid::util::init_log;
use celery::prelude::*;
use dotenv::dotenv;
use structopt::StructOpt;
use tracing::{event, instrument, Level};
use windmill::services::celery_app::*;
extern crate chrono;
use chrono::{DateTime, Duration, Utc};
use windmill::tasks::add::add;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "windmill",
    about = "Run a Rust Celery producer or consumer.",
    setting = structopt::clap::AppSettings::ColoredHelp,
)]
enum CeleryOpt {
    Consume {
        #[structopt(short, long, possible_values = &[
            "short_queue", "reports_queue", "tally_queue", "beat"
        ], default_value = "beat")]
        queues: Vec<String>,
        #[structopt(short, long, default_value = "100")]
        prefetch_count: u16,
        #[structopt(short, long)]
        acks_late: bool,
    },
    Produce,
}

fn get_seconds_later(seconds: i64) -> DateTime<Utc> {
    let current_time = Utc::now();
    current_time + Duration::seconds(seconds)
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

            celery_app
                //.consume_from(&["short_queue", "reports_queue", "tally_queue"])
                .consume_from(&vec_str[..])
                .await?;
            celery_app.close().await?;
        }
        CeleryOpt::Produce => {
            let celery_app = get_celery_app().await;
            event!(Level::INFO, "Task is empty, not adding any new tasks");
            // Basic task sending.
            let task1 = celery_app
                .send_task(add::new(1, 2).with_eta(get_seconds_later(100)))
                .await?;
            event!(Level::INFO, "Sent task {}", task1.task_id);

            let task2 = celery_app
                .send_task(add::new(0, 0).with_eta(get_seconds_later(5)))
                .await?;
            event!(Level::INFO, "Sent task {}", task2.task_id);
            celery_app.close().await?;
        }
    };

    Ok(())
}
