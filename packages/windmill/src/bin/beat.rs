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
use sequent_core::monitoring::cadence::{Cadence, SNAPSHOT_INTERVAL_ENV, VOTER_FULL_PASS_ENV};
use sequent_core::util::init_log::init_log;
use std::sync::Arc;
use tokio::time::Duration;
use windmill::services::celery_app::{set_is_app_active, Queue};
use windmill::services::database::get_queue_pool;
use windmill::services::electoral_log_audit::{
    checkpoint_interval_secs, DEFAULT_CHECKPOINT_INTERVAL_SECS,
};
use windmill::services::electoral_log_audit_policy::AuditPolicy;
use windmill::services::messaging::checks::{
    check_messaging_accounts, CHECK_PASS_INTERVAL_SECONDS,
};
use windmill::services::monitoring::cadence;
use windmill::services::probe::{setup_probe, AppName};
use windmill::tasks::audit_electoral_log::audit_open_elections_periodically;
use windmill::tasks::electoral_log::electoral_log_batch_dispatcher;
use windmill::tasks::migrate_messaging_attributes::migrate_messaging_attributes;
use windmill::tasks::migrate_realm_permissions::{migrate_realm_permissions, RunOnce};
use windmill::tasks::migrate_registration_flows::migrate_registration_flows;
use windmill::tasks::publish_electoral_log_checkpoint::publish_periodic_electoral_log_checkpoints;
use windmill::tasks::purge_queue_archives::{
    purge_interval_secs, purge_queue_archives, DEFAULT_PURGE_INTERVAL_SECS,
};
use windmill::tasks::recompute_schedule_instants::{
    recompute_schedule_instants, RECOMPUTE_INTERVAL_SECONDS,
};
use windmill::tasks::reconcile_messages::reconcile_messages;
use windmill::tasks::recount_messaging_voters::{
    recount_messaging_voters_due, RECOUNT_PASS_INTERVAL_SECONDS,
};
use windmill::tasks::refresh_monitoring_snapshot::{
    refresh_monitoring_snapshots, scheduled_fan_out,
};
use windmill::tasks::refresh_staff_crls::refresh_staff_crls;
use windmill::tasks::review_boards::review_boards;
use windmill::tasks::review_cast_votes::review_cast_votes;
use windmill::tasks::scheduled_events::scheduled_events;
use windmill::tasks::scheduled_reports::scheduled_reports;
use windmill::tasks::seal_ballot_boxes::{seal_ballot_boxes, SEAL_BALLOT_BOXES_INTERVAL_SECONDS};
use windmill::tasks::sequence_ballot_box::schedule_ballot_box_sequencers;
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
    /// Seconds between two passes of the messaging sweep (retries,
    /// reconciliation, unresolved outcomes, Messenger link expiry).
    #[arg(long, default_value = "60")]
    reconcile_messages_interval: u64,
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
    let monitoring_cadence = Cadence::parse(
        CeleryOpt::parse().monitoring_snapshot_interval.as_deref(),
        std::env::var(VOTER_FULL_PASS_ENV).ok().as_deref(),
    );
    cadence::log(&monitoring_cadence);

    // Refuse to start on an invalid interval; the schedule below cannot return errors.
    checkpoint_interval_secs()?;
    let audit_policy = AuditPolicy::from_env().map_err(anyhow::Error::msg)?;
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
            recompute_schedule_instants::NAME => {
                recompute_schedule_instants,
                schedule = DeltaSchedule::new(Duration::from_secs(RECOMPUTE_INTERVAL_SECONDS)),
                args = (),
            },
            reconcile_messages::NAME => {
                reconcile_messages,
                schedule = DeltaSchedule::new(Duration::from_secs(CeleryOpt::parse().reconcile_messages_interval)),
                args = (),
            },
            seal_ballot_boxes::NAME => {
                seal_ballot_boxes,
                schedule = DeltaSchedule::new(Duration::from_secs(SEAL_BALLOT_BOXES_INTERVAL_SECONDS)),
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
            // Hourly passes; each checks only the accounts due (daily, or
            // after an identity change).
            check_messaging_accounts::NAME => {
                check_messaging_accounts,
                schedule = DeltaSchedule::new(Duration::from_secs(CHECK_PASS_INTERVAL_SECONDS)),
                args = (),
            },
            // Hourly passes; each asks for the voter recounts due, staggered.
            recount_messaging_voters_due::NAME => {
                recount_messaging_voters_due,
                schedule = DeltaSchedule::new(Duration::from_secs(RECOUNT_PASS_INTERVAL_SECONDS)),
                args = (),
            },
        ],
        task_routes = [
            review_boards::NAME => Queue::Beat.queue_name(),
            scheduled_events::NAME => Queue::Beat.queue_name(),
            scheduled_reports::NAME => Queue::Beat.queue_name(),
            review_cast_votes::NAME => Queue::Beat.queue_name(),
            electoral_log_batch_dispatcher::NAME => Queue::ElectoralLogBeat.queue_name(),
            refresh_monitoring_snapshots::NAME => Queue::Beat.queue_name(),
            migrate_realm_permissions::NAME => Queue::Short.queue_name(),
            migrate_registration_flows::NAME => Queue::Short.queue_name(),
            migrate_messaging_attributes::NAME => Queue::Short.queue_name(),
            post_signing_log_outbox::NAME => Queue::ElectoralLogBeat.queue_name(),
            expire_signing_requests::NAME => Queue::Beat.queue_name(),
            sweep_signing_executions::NAME => Queue::Beat.queue_name(),
            refresh_staff_crls::NAME => Queue::Beat.queue_name(),
            recompute_schedule_instants::NAME => Queue::Beat.queue_name(),
            reconcile_messages::NAME => Queue::Beat.queue_name(),
            seal_ballot_boxes::NAME => Queue::Beat.queue_name(),
            publish_periodic_electoral_log_checkpoints::NAME => Queue::Beat.queue_name(),
            audit_open_elections_periodically::NAME => Queue::Reports.queue_name(),
            schedule_ballot_box_sequencers::NAME => Queue::ElectoralLogBeat.queue_name(),
            purge_queue_archives::NAME => Queue::Beat.queue_name(),
            check_messaging_accounts::NAME => Queue::Beat.queue_name(),
            recount_messaging_voters_due::NAME => Queue::Beat.queue_name(),
        ],
        default_queue = Queue::Beat.queue_name(),
    ).await?;
    beat.schedule_named_task(
        audit_open_elections_periodically::NAME.to_string(),
        audit_open_elections_periodically::new().with_expires_in(60),
        DeltaSchedule::new(Duration::from_secs(audit_policy.interval_secs)),
    );
    // Scheduled outside the macro, which cannot give a message its expiry.
    beat.schedule_named_task(
        refresh_monitoring_snapshots::NAME.to_string(),
        scheduled_fan_out(&monitoring_cadence),
        DeltaSchedule::new(Duration::from_secs(
            monitoring_cadence.snapshot_interval.seconds,
        )),
    );
    // Tenant realms made before a release get its new permissions once,
    // whenever beat starts; the migration skips the roles a realm has.
    beat.schedule_task(migrate_realm_permissions::new(), RunOnce);
    // Event realms made before the per-Post enrollment check get it in their
    // registration form, once whenever beat starts (VOTE-LIFECYCLE).
    beat.schedule_task(migrate_registration_flows::new(), RunOnce);
    // Voters of event realms get the single-channel attributes once,
    // whenever beat starts (INFRA-MSG).
    beat.schedule_task(migrate_messaging_attributes::new(), RunOnce);

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
