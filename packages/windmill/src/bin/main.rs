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
use chrono::{Utc, Duration, DateTime};
use windmill::tasks::add::add;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "windmill",
    about = "Run a Rust Celery producer or consumer.",
    setting = structopt::clap::AppSettings::ColoredHelp,
)]
enum CeleryOpt {
    Consume,
    Produce {
        #[structopt(possible_values = &[
            "add",
            "create_ballot_style",
            "create_board",
            "create_keys",
            "insert_ballots",
            "render_report",
            "set_public_key",
            "update_voting_status",
        ])]
        tasks: Vec<String>,
    },
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

    let celery_app = get_celery_app().await;

    match opt {
        CeleryOpt::Consume => {
            celery_app.display_pretty().await;
            celery_app
                //.consume_from(&["short_queue", "reports_queue", "tally_queue"])
                .consume_from(&["beat"])
                .await?;
        }
        CeleryOpt::Produce { tasks } => {
            if tasks.is_empty() {
                event!(Level::INFO, "Task is empty, not adding any new tasks");
                // Basic task sending.
                let task1 = celery_app
                    .send_task(
                        add::new(1, 2)
                        .with_eta(get_seconds_later(100))
                    )
                    .await?;
                event!(Level::INFO, "Sent task {}", task1.task_id);

                let task2 = celery_app
                    .send_task(
                        add::new(0, 0)
                        .with_eta(get_seconds_later(5))
                    )
                    .await?;
                event!(Level::INFO, "Sent task {}", task2.task_id);
            } else {
                event!(Level::INFO, "There are {} tasks", tasks.len());
            }
        }
    };

    celery_app.close().await?;
    Ok(())
}
