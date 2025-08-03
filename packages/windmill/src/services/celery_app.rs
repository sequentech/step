// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Context, Result};
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
use tracing::{event, info, instrument, Level};

use crate::services::plugins_manager::plugin_manager::init_plugin_manager;
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
use crate::tasks::plugins_tasks::execute_plugin_task;
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

impl Queue {
    pub fn queue_name(&self, slug: &str) -> String {
        format!("{}_{}", slug, self.as_ref())
    }
}

static mut PREFETCH_COUNT_S: u16 = 100;
static mut ACKS_LATE_S: bool = true;
static mut TASK_MAX_RETRIES: u32 = 4;
static mut IS_APP_ACTIVE: bool = true;
static mut BROKER_CONNECTION_MAX_RETRIES: u32 = 5;
static mut HEARTBEAT_SECS: u16 = 10;
static mut WORKER_THREADS: usize = 1;
static mut QUEUES: Vec<String> = vec![];

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

pub fn set_queues(new_val: Vec<String>) {
    unsafe {
        QUEUES = new_val;
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

pub fn get_queues() -> Vec<String> {
    unsafe { QUEUES.clone() }
}

/// CELERY_APP holds the high-level Celery application. Note: The Celery app is
/// built separately from the Broker because it handles task routing/scheduling.
lazy_static! {
    static ref CELERY_APP: AsyncOnce<Arc<Celery>> =
        AsyncOnce::new(async { generate_celery_app().await.unwrap() });
}

/// Returns the global Celery app.
#[instrument]
pub async fn get_celery_app() -> Arc<Celery> {
    CELERY_APP.get().await.clone()
}

#[instrument]
async fn create_connection() -> Result<(Arc<Connection>, String)> {
    // you can use "amqp://rabbitmq2:5672,amqp://rabbitmq:5672" for $AMQP_ADDR to configure multiple nodes, separated by comma
    let amqp_urls: Vec<String> = std::env::var("AMQP_ADDR")?
        .split(',')
        .map(String::from)
        .collect();

    let mut last_error = None;
    for amqp_url in amqp_urls {
        match Connection::connect(&amqp_url, ConnectionProperties::default())
            .await
            .with_context(|| format!("Failed to connect to any AMQP server {}", amqp_url))
        {
            Ok(connection) => {
                let arc_conn = Arc::new(connection);
                // Set the global connection so it can be reused.
                let _ = CELERY_CONNECTION.set(arc_conn.clone());
                return Ok((arc_conn, amqp_url));
            }
            Err(e) => {
                // Log the error and try the next URL.
                info!("Failed to connect to AMQP server '{}': {:?}", amqp_url, e);
                last_error = Some(e);
            }
        }
    }

    // If no connection was successful, return an error.
    Err(last_error.unwrap_or(anyhow!("Failed to connect to any AMQP server")))
}

#[instrument]
pub async fn generate_celery_app() -> Result<Arc<Celery>> {
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
    let slug = std::env::var("ENV_SLUG").with_context(|| "missing env var ENV_SLUG")?;
    let amqp_addr = create_connection()
        .await
        .with_context(|| "error creating rabbitmq connection")?
        .1;

    init_plugin_manager().await?;

    celery::app!(
        broker = AMQPBroker { amqp_addr },
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
            execute_plugin_task,

        ],
        task_routes = [
            create_keys::NAME => &Queue::Short.queue_name(&slug),
            review_boards::NAME => &Queue::Beat.queue_name(&slug),
            process_board::NAME => &Queue::Beat.queue_name(&slug),
            generate_manual_verification_report::NAME => &Queue::Reports.queue_name(&slug),
            render_report::NAME => &Queue::Reports.queue_name(&slug),
            create_ballot_receipt::NAME => &Queue::Reports.queue_name(&slug),
            generate_report::NAME => &Queue::Reports.queue_name(&slug),
            generate_template::NAME => &Queue::Reports.queue_name(&slug),
            render_document_pdf::NAME => &Queue::Reports.queue_name(&slug),
            set_public_key::NAME => &Queue::Short.queue_name(&slug),
            execute_tally_session::NAME => &Queue::Tally.queue_name(&slug),
            update_election_event_ballot_styles::NAME => &Queue::Short.queue_name(&slug),
            insert_election_event_t::NAME => &Queue::Short.queue_name(&slug),
            insert_tenant::NAME => &Queue::Short.queue_name(&slug),
            send_template::NAME => &Queue::Communication.queue_name(&slug),
            import_users::NAME => &Queue::ImportExport.queue_name(&slug),
            export_users::NAME => &Queue::ImportExport.queue_name(&slug),
            export_election_event::NAME => &Queue::ImportExport.queue_name(&slug),
            generate_activity_logs_report::NAME => &Queue::ImportExport.queue_name(&slug),
            export_tasks_execution::NAME => &Queue::ImportExport.queue_name(&slug),
            export_trustees_task::NAME => &Queue::ImportExport.queue_name(&slug),
            import_election_event::NAME => &Queue::ImportExport.queue_name(&slug),
            export_templates::NAME => &Queue::ImportExport.queue_name(&slug),
            export_tenant_config::NAME => &Queue::ImportExport.queue_name(&slug),
            import_tenant_config::NAME => &Queue::ImportExport.queue_name(&slug),
            scheduled_events::NAME => &Queue::Beat.queue_name(&slug),
            scheduled_reports::NAME => &Queue::Beat.queue_name(&slug),
            manage_election_date::NAME => &Queue::Beat.queue_name(&slug),
            manage_election_event_date::NAME => &Queue::Beat.queue_name(&slug),
            manage_election_event_enrollment::NAME => &Queue::Beat.queue_name(&slug),
            manage_election_event_lockdown::NAME => &Queue::Beat.queue_name(&slug),
            manage_election_init_report::NAME => &Queue::Beat.queue_name(&slug),
            manage_election_voting_period_end::NAME => &Queue::Beat.queue_name(&slug),
            manage_election_allow_tally::NAME => &Queue::Beat.queue_name(&slug),
            create_transmission_package_task::NAME => &Queue::Short.queue_name(&slug),
            send_transmission_package_task::NAME => &Queue::Short.queue_name(&slug),
            delete_election_event_t::NAME => &Queue::Short.queue_name(&slug),
            export_ballot_publication::NAME => &Queue::ImportExport.queue_name(&slug),
            export_application::NAME => &Queue::ImportExport.queue_name(&slug),
            import_applications::NAME => &Queue::ImportExport.queue_name(&slug),
            enqueue_electoral_log_event::NAME => &Queue::ElectoralLogEvent.queue_name(&slug),
            process_electoral_log_events_batch::NAME => &Queue::ElectoralLogBatch.queue_name(&slug),
            electoral_log_batch_dispatcher::NAME => &Queue::ElectoralLogBeat.queue_name(&slug),
            execute_plugin_task::NAME => &Queue::Short.queue_name(&slug),
        ],
        prefetch_count = prefetch_count,
        acks_late = acks_late,
        task_max_retries = task_max_retries,
        heartbeat = Some(heartbeat),
        broker_connection_max_retries = broker_connection_max_retries,
    )
    .await
    .map_err(|err| anyhow!("{:?}", err))
}

static CELERY_CONNECTION: OnceLock<Arc<Connection>> = OnceLock::new();

/// Returns a reused AMQP connection wrapped in an Arc.
/// If no connection exists (or if it’s disconnected), a new connection is created and stored.
#[instrument]
pub async fn get_celery_connection() -> Result<Arc<Connection>> {
    if let Some(conn) = CELERY_CONNECTION.get() {
        // For simplicity we assume the connection is still valid.
        return Ok(conn.clone());
    }

    create_connection().await.map(|(connection, _)| connection)
}
