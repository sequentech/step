#![allow(non_upper_case_globals)]
// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use celery::beat::{CronSchedule, DeltaSchedule};
use celery::task::TaskResult;
use tokio::time::Duration;
use dotenv::dotenv;
use std;
use windmill::tasks::add::add;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    // Build a `Beat` with a default scheduler backend.
    let mut beat = celery::beat!(
        broker = AMQPBroker { std::env::var("AMQP_ADDR").unwrap_or_else(|_| "amqp://rabbitmq:5672".into()) },
        tasks = [
            "add" => {
                add,
                schedule = DeltaSchedule::new(Duration::from_secs(60)),
                args = (1, 2),
            },
        ],
        task_routes = [
            "add" => "beat",
        ],
    ).await?;

    beat.start().await?;

    Ok(())
}