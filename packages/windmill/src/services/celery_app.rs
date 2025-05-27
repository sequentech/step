// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use async_once::AsyncOnce;
use celery::prelude::Task;
use celery::Celery;
use futures::future::Lazy;
use lapin::{Channel, Connection, ConnectionProperties};
use std;
use std::convert::AsRef;
use std::sync::Arc;
use std::sync::OnceLock;
use strum_macros::AsRefStr;
use tokio::sync::{Mutex, RwLock};
use tracing::{event, instrument, Level};

use crate::tasks::activity_logs_report::generate_activity_logs_report;
use crate::tasks::create_ballot_receipt::create_ballot_receipt;
use crate::tasks::create_keys::create_keys;
use crate::tasks::delete_election_event::delete_election_event_t;
use crate::tasks::electoral_log::{
    electoral_log_batch_dispatcher, enqueue_electoral_log_event, process_electoral_log_events_batch,
};
use crate::tasks::execute_tally_session::execute_tally_session;
use crate::tasks::export_application::export_application;
use crate::tasks::export_ballot_publication::export_ballot_publication;
use crate::tasks::export_election_event::export_election_event;
use crate::tasks::export_tasks_execution::export_tasks_execution;
use crate::tasks::export_templates::export_templates;
use crate::tasks::export_tenant_config::export_tenant_config;
use crate::tasks::export_trustees::export_trustees_task;
use crate::tasks::export_users::export_users;
use crate::tasks::generate_report::generate_report;
use crate::tasks::generate_template::generate_template;
use crate::tasks::import_application::import_applications;
use crate::tasks::import_election_event::import_election_event;
use crate::tasks::import_tenant_config::import_tenant_config;
use crate::tasks::import_users::import_users;
use crate::tasks::insert_election_event::insert_election_event_t;
use crate::tasks::insert_tenant::insert_tenant;
use crate::tasks::manage_election_allow_tally::manage_election_allow_tally;
use crate::tasks::manage_election_dates::manage_election_date;
use crate::tasks::manage_election_event_date::manage_election_event_date;
use crate::tasks::manage_election_event_enrollment::manage_election_event_enrollment;
use crate::tasks::manage_election_event_lockdown::manage_election_event_lockdown;
use crate::tasks::manage_election_init_report::manage_election_init_report;
use crate::tasks::manage_election_voting_period_end::manage_election_voting_period_end;
use crate::tasks::manual_verification_report::generate_manual_verification_report;
use crate::tasks::miru_plugin_tasks::create_transmission_package_task;
use crate::tasks::miru_plugin_tasks::send_transmission_package_task;
use crate::tasks::prepare_publication_preview::prepare_publication_preview;
use crate::tasks::process_board::process_board;
use crate::tasks::render_document_pdf::render_document_pdf;
use crate::tasks::render_report::render_report;
use crate::tasks::review_boards::review_boards;
use crate::tasks::scheduled_events::scheduled_events;
use crate::tasks::scheduled_reports::scheduled_reports;
use crate::tasks::send_template::send_template;
use crate::tasks::set_public_key::set_public_key;
use crate::tasks::update_election_event_ballot_styles::update_election_event_ballot_styles;

#[derive(AsRefStr, Debug)]
pub enum Queue {
    #[strum(serialize = "beat")]
    Beat,
    #[strum(serialize = "short_queue")]
    Short,
    #[strum(serialize = "communication_queue")]
    Communication,
    #[strum(serialize = "tally_queue")]
    Tally,
    #[strum(serialize = "reports_queue")]
    Reports,
    #[strum(serialize = "import_export_queue")]
    ImportExport,

    #[strum(serialize = "electoral_log_beat_queue")]
    ElectoralLogBeat,
    #[strum(serialize = "electoral_log_batch_queue")]
    ElectoralLogBatch,
    #[strum(serialize = "electoral_log_event_queue")]
    ElectoralLogEvent,
}

static mut PREFETCH_COUNT_S: u16 = 100;
static mut ACKS_LATE_S: bool = true;
static mut TASK_MAX_RETRIES: u32 = 4;
static mut IS_APP_ACTIVE: bool = true;
static mut BROKER_CONNECTION_MAX_RETRIES: u32 = 5;
static mut HEARTBEAT_SECS: u16 = 10;
static mut WORKER_THREADS: usize = 1;

pub fn set_prefetch_count(new_val: u16) {
    unsafe {
        PREFETCH_COUNT_S = new_val;
    }
}

pub fn set_worker_threads(new_val: usize) {
    unsafe {
        WORKER_THREADS = new_val;
    }
}

pub fn get_worker_threads() -> usize {
    unsafe { WORKER_THREADS }
}

pub fn set_acks_late(new_val: bool) {
    unsafe {
        ACKS_LATE_S = new_val;
    }
}

pub fn set_task_max_retries(new_val: u32) {
    unsafe {
        TASK_MAX_RETRIES = new_val;
    }
}

#[instrument]
pub fn set_is_app_active(new_val: bool) {
    unsafe {
        IS_APP_ACTIVE = new_val;
    }
}

pub fn set_broker_connection_max_retries(new_val: u32) {
    unsafe {
        BROKER_CONNECTION_MAX_RETRIES = new_val;
    }
}

pub fn set_heartbeat(new_val: u16) {
    unsafe {
        HEARTBEAT_SECS = new_val;
    }
}

pub fn get_is_app_active() -> bool {
    unsafe { IS_APP_ACTIVE }
}

/// CELERY_APP holds the high-level Celery application. Note: The Celery app is
/// built separately from the Broker because it handles task routing/scheduling.
lazy_static! {
    static ref CELERY_APP: AsyncOnce<Arc<Celery>> =
        AsyncOnce::new(async { generate_celery_app().await });
}

/// Returns the global Celery app.
pub async fn get_celery_app() -> Arc<Celery> {
    CELERY_APP.get().await.clone()
}

#[instrument]
pub async fn generate_celery_app() -> Arc<Celery> {
    let prefetch_count: u16;
    let acks_late: bool;
    let task_max_retries: u32;
    let broker_connection_max_retries: u32;
    let heartbeat: u16;
    unsafe {
        prefetch_count = PREFETCH_COUNT_S;
        acks_late = ACKS_LATE_S;
        task_max_retries = TASK_MAX_RETRIES;
        broker_connection_max_retries = BROKER_CONNECTION_MAX_RETRIES;
        heartbeat = HEARTBEAT_SECS;
    }
    event!(
        Level::INFO,
        "prefetch_count: {}, acks_late: {}",
        prefetch_count,
        acks_late
    );
    celery::app!(
        broker = AMQPBroker { std::env::var("AMQP_ADDR").unwrap_or_else(|_| "amqp://rabbitmq:5672".into()) },
        tasks = [
            create_keys,
            review_boards,
            process_board,
            render_report,
            generate_report,
            generate_template,
            create_ballot_receipt,
            set_public_key,
            execute_tally_session,
            update_election_event_ballot_styles,
            insert_election_event_t,
            insert_tenant,
            send_template,
            import_users,
            export_users,
            import_election_event,
            generate_manual_verification_report,
            scheduled_events,
            manage_election_event_date,
            manage_election_event_enrollment,
            manage_election_event_lockdown,
            manage_election_init_report,
            manage_election_voting_period_end,
            manage_election_allow_tally,
            manage_election_date,
            export_election_event,
            generate_activity_logs_report,
            create_transmission_package_task,
            send_transmission_package_task,
            delete_election_event_t,
            export_tasks_execution,
            scheduled_reports,
            export_templates,
            export_ballot_publication,
            export_application,
            import_applications,
            export_trustees_task,
            export_tenant_config,
            import_tenant_config,
            enqueue_electoral_log_event,
            process_electoral_log_events_batch,
            electoral_log_batch_dispatcher,
            render_document_pdf,
            prepare_publication_preview,
        ],
        task_routes = [
            create_keys::NAME => Queue::Short.as_ref(),
            review_boards::NAME => Queue::Beat.as_ref(),
            process_board::NAME => Queue::Beat.as_ref(),
            generate_manual_verification_report::NAME => Queue::Reports.as_ref(),
            render_report::NAME => Queue::Reports.as_ref(),
            create_ballot_receipt::NAME => Queue::Reports.as_ref(),
            generate_report::NAME => Queue::Reports.as_ref(),
            generate_template::NAME => Queue::Reports.as_ref(),
            render_document_pdf::NAME => Queue::Reports.as_ref(),
            set_public_key::NAME => Queue::Short.as_ref(),
            execute_tally_session::NAME => Queue::Tally.as_ref(),
            update_election_event_ballot_styles::NAME => Queue::Short.as_ref(),
            insert_election_event_t::NAME => Queue::Short.as_ref(),
            insert_tenant::NAME => Queue::Short.as_ref(),
            send_template::NAME => Queue::Communication.as_ref(),
            import_users::NAME => Queue::ImportExport.as_ref(),
            export_users::NAME => Queue::ImportExport.as_ref(),
            export_election_event::NAME => Queue::ImportExport.as_ref(),
            generate_activity_logs_report::NAME => Queue::ImportExport.as_ref(),
            export_tasks_execution::NAME => Queue::ImportExport.as_ref(),
            export_trustees_task::NAME => Queue::ImportExport.as_ref(),
            import_election_event::NAME => Queue::ImportExport.as_ref(),
            export_templates::NAME => Queue::ImportExport.as_ref(),
            export_tenant_config::NAME => Queue::ImportExport.as_ref(),
            import_tenant_config::NAME => Queue::ImportExport.as_ref(),
            scheduled_events::NAME => Queue::Beat.as_ref(),
            scheduled_reports::NAME => Queue::Beat.as_ref(),
            manage_election_date::NAME => Queue::Beat.as_ref(),
            manage_election_event_date::NAME => Queue::Beat.as_ref(),
            manage_election_event_enrollment::NAME => Queue::Beat.as_ref(),
            manage_election_event_lockdown::NAME => Queue::Beat.as_ref(),
            manage_election_init_report::NAME => Queue::Beat.as_ref(),
            manage_election_voting_period_end::NAME => Queue::Beat.as_ref(),
            manage_election_allow_tally::NAME => Queue::Beat.as_ref(),
            create_transmission_package_task::NAME => Queue::Short.as_ref(),
            send_transmission_package_task::NAME => Queue::Short.as_ref(),
            delete_election_event_t::NAME => Queue::Short.as_ref(),
            export_ballot_publication::NAME => Queue::ImportExport.as_ref(),
            export_application::NAME => Queue::ImportExport.as_ref(),
            import_applications::NAME => Queue::ImportExport.as_ref(),
            enqueue_electoral_log_event::NAME => Queue::ElectoralLogEvent.as_ref(),
            process_electoral_log_events_batch::NAME => Queue::ElectoralLogBatch.as_ref(),
            electoral_log_batch_dispatcher::NAME => Queue::ElectoralLogBeat.as_ref(),
            prepare_publication_preview::NAME => Queue::Beat.as_ref(),
        ],
        prefetch_count = prefetch_count,
        acks_late = acks_late,
        task_max_retries = task_max_retries,
        heartbeat = Some(heartbeat),
        broker_connection_max_retries = broker_connection_max_retries,
    ).await.unwrap()
}

static CELERY_CONNECTION: OnceLock<Arc<Connection>> = OnceLock::new();

/// Returns a reused AMQP connection wrapped in an Arc.
/// If no connection exists (or if it’s disconnected), a new connection is created and stored.
pub async fn get_celery_connection() -> Result<Arc<Connection>> {
    if let Some(conn) = CELERY_CONNECTION.get() {
        // For simplicity we assume the connection is still valid.
        return Ok(conn.clone());
    }
    let amqp_url = std::env::var("AMQP_ADDR").unwrap_or_else(|_| "amqp://rabbitmq:5672".into());
    let connection = Connection::connect(&amqp_url, ConnectionProperties::default()).await?;
    let arc_conn = Arc::new(connection);
    let _ = CELERY_CONNECTION.set(arc_conn.clone());
    Ok(arc_conn)
}
