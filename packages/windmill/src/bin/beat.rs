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
use sequent_core::monitoring::cadence::{Cadence, SNAPSHOT_INTERVAL_ENV, VOTER_FULL_PASS_ENV};
use sequent_core::util::init_log::init_log;
use tokio::time::Duration;
use windmill::services::celery_app::{set_is_app_active, Queue};
use windmill::services::monitoring::cadence;
use windmill::services::probe::{setup_probe, AppName};
use windmill::tasks::electoral_log::electoral_log_batch_dispatcher;
use windmill::tasks::refresh_monitoring_snapshot::{
    refresh_monitoring_snapshots, scheduled_fan_out,
};
use windmill::tasks::refresh_staff_crls::refresh_staff_crls;
use windmill::tasks::review_boards::review_boards;
use windmill::tasks::review_cast_votes::review_cast_votes;
use windmill::tasks::scheduled_events::scheduled_events;
use windmill::tasks::scheduled_reports::scheduled_reports;
use windmill::tasks::signing_log_outbox::post_signing_log_outbox;
use windmill::tasks::signing_requests::{
    expire_signing_requests, sweep_signing_executions, SIGNING_JOBS_INTERVAL_SECONDS,
};

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
    /// Seconds between two monitoring snapshot passes; bounds and default
    /// in `sequent_core::monitoring::cadence`.
    #[arg(short = 'm', long, env = SNAPSHOT_INTERVAL_ENV)]
    monitoring_snapshot_interval: Option<String>,
    /// Seconds between two passes of the signing log outbox.
    #[arg(short = 'g', long, default_value = "5")]
    signing_log_interval: u64,
    /// Seconds between two refreshes of the staff issuers' revocation lists.
    #[arg(long, env = "STAFF_CRL_INTERVAL", default_value = "3600")]
    staff_crl_interval: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    init_log(true);
    setup_probe(AppName::BEAT).await;
    let slug = std::env::var("ENV_SLUG").with_context(|| "missing env var ENV_SLUG")?;
    let monitoring_cadence = Cadence::parse(
        CeleryOpt::parse().monitoring_snapshot_interval.as_deref(),
        std::env::var(VOTER_FULL_PASS_ENV).ok().as_deref(),
    );
    cadence::log(&monitoring_cadence);

    let mut beat = celery::beat!(
        broker = AMQPBroker { std::env::var("AMQP_ADDR").unwrap_or_else(|_| "amqp://rabbitmq:5672".into()) },
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
                args = (CeleryOpt::parse().schedule_events_interval),
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
            post_signing_log_outbox::NAME => {
                post_signing_log_outbox,
                schedule = DeltaSchedule::new(Duration::from_secs(CeleryOpt::parse().signing_log_interval)),
                args = (),
            },
            expire_signing_requests::NAME => {
                expire_signing_requests,
                schedule = DeltaSchedule::new(Duration::from_secs(SIGNING_JOBS_INTERVAL_SECONDS)),
                args = (),
            },
            sweep_signing_executions::NAME => {
                sweep_signing_executions,
                schedule = DeltaSchedule::new(Duration::from_secs(SIGNING_JOBS_INTERVAL_SECONDS)),
                args = (),
            },
            refresh_staff_crls::NAME => {
                refresh_staff_crls,
                schedule = DeltaSchedule::new(Duration::from_secs(CeleryOpt::parse().staff_crl_interval)),
                args = (),
            },
        ],
        task_routes = [
            review_boards::NAME => &Queue::Beat.queue_name(&slug),
            scheduled_events::NAME => &Queue::Beat.queue_name(&slug),
            scheduled_reports::NAME => &Queue::Beat.queue_name(&slug),
            review_cast_votes::NAME => &Queue::Beat.queue_name(&slug),
            electoral_log_batch_dispatcher::NAME => &Queue::ElectoralLogBeat.queue_name(&slug),
            refresh_monitoring_snapshots::NAME => &Queue::Beat.queue_name(&slug),
            post_signing_log_outbox::NAME => &Queue::ElectoralLogBeat.queue_name(&slug),
            expire_signing_requests::NAME => &Queue::Beat.queue_name(&slug),
            sweep_signing_executions::NAME => &Queue::Beat.queue_name(&slug),
            refresh_staff_crls::NAME => &Queue::Beat.queue_name(&slug),
        ],
    ).await?;
    // Scheduled outside the macro, which cannot give a message its expiry.
    beat.schedule_named_task(
        refresh_monitoring_snapshots::NAME.to_string(),
        scheduled_fan_out(&monitoring_cadence),
        DeltaSchedule::new(Duration::from_secs(
            monitoring_cadence.snapshot_interval.seconds,
        )),
    );

    set_is_app_active(true);
    beat.start().await?;
    set_is_app_active(false);
    Ok(())
}
