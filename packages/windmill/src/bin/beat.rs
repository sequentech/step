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
use windmill::services::database::get_queue_pool;
use windmill::services::electoral_log_audit::{
    checkpoint_interval_secs, DEFAULT_CHECKPOINT_INTERVAL_SECS,
};
use windmill::services::probe::{setup_probe, AppName};
use windmill::tasks::electoral_log::electoral_log_batch_dispatcher;
use windmill::tasks::publish_electoral_log_checkpoint::publish_periodic_electoral_log_checkpoints;
use windmill::tasks::purge_queue_archives::{
    purge_interval_secs, purge_queue_archives, DEFAULT_PURGE_INTERVAL_SECS,
};
use windmill::tasks::review_boards::review_boards;
use windmill::tasks::review_cast_votes::review_cast_votes;
use windmill::tasks::scheduled_events::scheduled_events;
use windmill::tasks::scheduled_reports::scheduled_reports;
use windmill::tasks::sequence_ballot_box::schedule_ballot_box_sequencers;

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
    /// Seconds between scans for ballots waiting for the sequencer.
    #[arg(short = 'b', long, default_value = "2")]
    ballot_box_interval: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    init_log(true);
    set_is_app_active(false);
    setup_probe(AppName::BEAT).await;
    let slug = std::env::var("ENV_SLUG").with_context(|| "missing env var ENV_SLUG")?;
    // Refuse to start on an invalid interval; the schedule below cannot return errors.
    checkpoint_interval_secs()?;
    purge_interval_secs()?;

    // A dedicated pooled session holds leadership for this environment's scheduler.
    let pool = get_queue_pool().await;
    let leader = Arc::new(
        pool.get()
            .await
            .context("Error obtaining Beat leadership connection")?,
    );
    // The server ends the session soon after this process stops pinging it, so the lock of a
    // Beat whose node died does not outlive it.
    leader
        .simple_query("SET idle_session_timeout = '30s'")
        .await
        .context("Error configuring Beat leadership connection")?;
    let lock_name = format!("step:beat:{slug}");
    // A second Beat, such as a rolling update's, waits as a ready standby until the leader
    // stops.
    set_is_app_active(true);
    loop {
        let acquired: bool = leader
            .query_one(
                "SELECT pg_try_advisory_lock(hashtextextended($1, 0))",
                &[&lock_name],
            )
            .await?
            .get(0);
        if acquired {
            break;
        }
        tracing::info!("Another Beat scheduler owns this environment; waiting");
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
    set_is_app_active(false);

    let mut beat = celery::beat!(
        broker_builder = Box::new(
            PgmqBrokerBuilder::from_pool(pool.clone())
                .environment(&slug)
                .publisher_connection(leader.clone())
        ),
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
            schedule_ballot_box_sequencers::NAME => {
                schedule_ballot_box_sequencers,
                schedule = DeltaSchedule::new(Duration::from_secs(CeleryOpt::parse().ballot_box_interval)),
                args = (),
            },
            publish_periodic_electoral_log_checkpoints::NAME => {
                publish_periodic_electoral_log_checkpoints,
                schedule = DeltaSchedule::new(Duration::from_secs(
                    checkpoint_interval_secs().unwrap_or(DEFAULT_CHECKPOINT_INTERVAL_SECS),
                )),
                args = (),
            },
            purge_queue_archives::NAME => {
                purge_queue_archives,
                schedule = DeltaSchedule::new(Duration::from_secs(
                    purge_interval_secs().unwrap_or(DEFAULT_PURGE_INTERVAL_SECS),
                )),
                args = (),
            },
        ],
        task_routes = [
            review_boards::NAME => Queue::Beat.queue_name(),
            scheduled_events::NAME => Queue::Beat.queue_name(),
            scheduled_reports::NAME => Queue::Beat.queue_name(),
            review_cast_votes::NAME => Queue::Beat.queue_name(),
            electoral_log_batch_dispatcher::NAME => Queue::ElectoralLogBeat.queue_name(),
            publish_periodic_electoral_log_checkpoints::NAME => Queue::Beat.queue_name(),
            schedule_ballot_box_sequencers::NAME => Queue::ElectoralLogBeat.queue_name(),
            purge_queue_archives::NAME => Queue::Beat.queue_name(),
        ],
        default_queue = Queue::Beat.queue_name(),
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
