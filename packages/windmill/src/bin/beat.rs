#![allow(non_upper_case_globals)]
// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use celery::beat::DeltaSchedule;
use dotenv::dotenv;
use std;
use tokio::time::Duration;
use windmill::tasks::review_boards::review_boards;
use sequent_core::util::date::get_seconds_later;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    // Build a `Beat` with a default scheduler backend.
    let mut beat = celery::beat!(
        broker = AMQPBroker { std::env::var("AMQP_ADDR").unwrap_or_else(|_| "amqp://rabbitmq:5672".into()) },
        tasks = [
            "review_boards" => {
                review_boards,
                schedule = DeltaSchedule::new(Duration::from_secs(60)),
                args = (),
            },
        ],
        task_routes = [
            "review_boards" => "beat",
        ],
    ).await?;

    beat.start().await?;

    Ok(())
}
