#![allow(non_upper_case_globals)]
#![recursion_limit = "256"]
// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{Context, Result};
use celery::beat::DeltaSchedule;
use celery::prelude::Task;
use clap::Parser;
use dotenv::dotenv;
use pgmq_broker::PgmqBrokerBuilder;
use sequent_core::util::init_log::init_log;
use std::sync::Arc;
use tokio::time::Duration;
use windmill::services::celery_app::{set_is_app_active, Queue};
use windmill::services::database::get_keycloak_pool;
use windmill::services::probe::{setup_probe, AppName};
use windmill::tasks::electoral_log::electoral_log_batch_dispatcher;
use windmill::tasks::review_boards::review_boards;
use windmill::tasks::review_cast_votes::review_cast_votes;
use windmill::tasks::scheduled_events::scheduled_events;
use windmill::tasks::scheduled_reports::scheduled_reports;

#[derive(Debug, Parser)]
#[command(name = "beat", about = "Windmill's periodic task scheduler.")]
struct CeleryOpt {
    #[arg(short = 'r', long, default_value = "15")]
    review_boards_interval: u64,
    #[arg(short = 's', long, default_value = "10")]
    schedule_events_interval: u64,
    #[arg(short = 'c', long, default_value = "60")]
    schedule_reports_interval: u64,
    #[arg(short = 'v', long, default_value = "90")]
    review_cast_votes_interval: u64,
    #[arg(short = 'e', long, default_value = "5")]
    electoral_log_interval: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    init_log(true);
    set_is_app_active(false);
    setup_probe(AppName::BEAT).await;
    let slug = std::env::var("ENV_SLUG").with_context(|| "missing env var ENV_SLUG")?;

    // A dedicated pooled session holds leadership for this environment's scheduler.
    let pool = get_keycloak_pool().await;
    let leader = Arc::new(
        pool.get()
            .await
            .context("Error obtaining Beat leadership connection")?,
    );
    let lock_name = format!("step:beat:{slug}");
    let acquired: bool = leader
        .query_one(
            "SELECT pg_try_advisory_lock(hashtextextended($1, 0))",
            &[&lock_name],
        )
        .await?
        .get(0);
    if !acquired {
        anyhow::bail!("Another Beat scheduler already owns this environment");
    }

    let mut beat = celery::beat!(
        broker_builder = Box::new(PgmqBrokerBuilder::from_pool(pool.clone()).publisher_connection(leader.clone())),
        tasks = [
            review_boards::NAME => {
                review_boards,
                schedule = DeltaSchedule::new(Duration::from_secs(CeleryOpt::parse().review_boards_interval)),
                args = (),
            },
            scheduled_events::NAME => {
                scheduled_events,
                schedule = DeltaSchedule::new(Duration::from_secs(CeleryOpt::parse().schedule_events_interval)),
                args = (CeleryOpt::parse().schedule_events_interval),
            },
            scheduled_reports::NAME => {
                scheduled_reports,
                schedule = DeltaSchedule::new(Duration::from_secs(CeleryOpt::parse().schedule_reports_interval)),
                args = (CeleryOpt::parse().schedule_reports_interval),
            },
            review_cast_votes::NAME => {
                review_cast_votes,
                schedule = DeltaSchedule::new(Duration::from_secs(CeleryOpt::parse().review_cast_votes_interval)),
                args = (),
            },
            electoral_log_batch_dispatcher::NAME => {
                electoral_log_batch_dispatcher,
                schedule = DeltaSchedule::new(Duration::from_secs(CeleryOpt::parse().electoral_log_interval)),
                args = (),
            },
        ],
        task_routes = [
            review_boards::NAME => &Queue::Beat.queue_name(&slug),
            scheduled_events::NAME => &Queue::Beat.queue_name(&slug),
            scheduled_reports::NAME => &Queue::Beat.queue_name(&slug),
            review_cast_votes::NAME => &Queue::Beat.queue_name(&slug),
            electoral_log_batch_dispatcher::NAME => &Queue::ElectoralLogBeat.queue_name(&slug),
        ],
        default_queue = &Queue::Beat.queue_name(&slug),
    ).await?;

    set_is_app_active(true);
    let monitor = async {
        loop {
            tokio::time::sleep(Duration::from_secs(5)).await;
            tokio::time::timeout(Duration::from_secs(5), leader.simple_query("SELECT 1"))
                .await
                .context("Beat leadership connection timed out")?
                .context("Beat leadership connection lost")?;
        }
        #[allow(unreachable_code)]
        Ok::<(), anyhow::Error>(())
    };
    let result = tokio::select! {
        result = beat.start() => result.map_err(anyhow::Error::from),
        result = monitor => result,
    };
    set_is_app_active(false);
    let _ = tokio::time::timeout(
        Duration::from_secs(5),
        leader.query_one(
            "SELECT pg_advisory_unlock(hashtextextended($1, 0))",
            &[&lock_name],
        ),
    )
    .await;
    result
}
