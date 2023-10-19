#![allow(non_upper_case_globals)]
// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use async_trait::async_trait;
use braid::util::init_log;
use celery::prelude::*;
use dotenv::dotenv;
use structopt::StructOpt;
use tracing::{event, instrument, Level};

use crate::tasks::set_public_key::set_public_key_task;

pub mod connection;
pub mod hasura;
pub mod services;
pub mod tasks;
pub mod types;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "windmill",
    about = "Run a Rust Celery producer or consumer.",
    setting = structopt::clap::AppSettings::ColoredHelp,
)]
enum CeleryOpt {
    Consume,
    Produce {
        #[structopt(possible_values = &["set_public_key_task"])]
        tasks: Vec<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    init_log(true);
    let opt = CeleryOpt::from_args();

    let celery_app = celery::app!(
        broker = AMQPBroker { std::env::var("AMQP_ADDR").unwrap_or_else(|_| "amqp://rabbitmq:5672".into()) },
        tasks = [
            set_public_key_task,
        ],
        // Route certain tasks to certain queues based on glob matching.
        task_routes = [
            "set_public_key_task" => "short_queue",
        ],
        prefetch_count = 2,
        heartbeat = Some(10),
    ).await?;

    match opt {
        CeleryOpt::Consume => {
            celery_app.display_pretty().await;
            celery_app.consume_from(&["short_queue"]).await?;
        }
        CeleryOpt::Produce { tasks } => {
            if tasks.is_empty() {
                event!(Level::INFO, "Task is empty, not adding any new tasks");
                // Basic task sending.
                //let task1 = celery_app.send_task(add::new(1, 2)).await?;
                //event!(Level::INFO, "Sent task {}", task1.task_id);
            } else {
                event!(Level::INFO, "There are {} tasks", tasks.len());
            }
        }
    };

    celery_app.close().await?;
    Ok(())
}
