// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Context, Result};
use celery::prelude::Task;
use celery::Celery;
use lapin::{Connection, ConnectionProperties};
use std::sync::{Arc, LazyLock, RwLock};
use strum_macros::AsRefStr;
use tokio::sync::OnceCell;
use tracing::{event, info, instrument, Level};

use crate::services::plugins_manager::plugin_manager::init_plugin_manager;
use crate::tasks::activity_logs_report::generate_activity_logs_report;
use crate::tasks::create_ballot_receipt::create_ballot_receipt;
use crate::tasks::create_keys::create_keys;
use crate::tasks::delete_election_event::delete_election_event_t;
use crate::tasks::edit_user::edit_user;
use crate::tasks::electoral_log::{
    electoral_log_batch_dispatcher, enqueue_electoral_log_event, process_electoral_log_events_batch,
};
use crate::tasks::execute_tally_session::execute_tally_session;
use crate::tasks::export_application::export_application;
use crate::tasks::export_ballot_publication::export_ballot_publication;
use crate::tasks::export_certificate_authority::export_certificate_authority;
use crate::tasks::export_election_event::export_election_event;
use crate::tasks::export_tally_results::export_tally_results_to_xlsx_task;
use crate::tasks::export_tasks_execution::export_tasks_execution;
use crate::tasks::export_templates::export_templates;
use crate::tasks::export_tenant_config::export_tenant_config;
use crate::tasks::export_trustees::export_trustees_task;
use crate::tasks::export_users::export_users;
use crate::tasks::generate_report::generate_report;
use crate::tasks::generate_template::generate_template;
use crate::tasks::import_application::import_applications;
use crate::tasks::import_election_event::import_election_event;
use crate::tasks::import_templates::import_templates_task;
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
use crate::tasks::post_tally::post_tally_task;
use crate::tasks::prepare_publication_preview::prepare_publication_preview;
use crate::tasks::process_board::process_board;
use crate::tasks::process_cast_vote::process_cast_vote;
use crate::tasks::render_document_pdf::render_document_pdf;
use crate::tasks::render_report::render_report;
use crate::tasks::review_boards::review_boards;
use crate::tasks::review_cast_votes::review_cast_votes;
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

/// The main struct for global Celery configuration.
/// Set at-most once; either by command line options during startup or falls back to defaults.
pub struct CeleryConfig {
    pub prefetch_count: u16,
    pub acks_late: bool,
    pub task_max_retries: u32,
    pub broker_connection_max_retries: u32,
    pub heartbeat_secs: u16,
}

/// Global Celery configuration.
/// Expected to be either set once during startup or used with defaults.
static CELERY_CONFIG: LazyLock<RwLock<CeleryConfig>> = LazyLock::new(|| {
    RwLock::new(CeleryConfig {
        prefetch_count: 100,
        acks_late: true,
        task_max_retries: 4,
        broker_connection_max_retries: 5,
        heartbeat_secs: 10,
    })
});

/// Global Celery queues configured.
/// Expected to be either set once during startup or kept empty.
static QUEUES: LazyLock<RwLock<Vec<String>>> = LazyLock::new(|| RwLock::new(Vec::new()));

/// Globally configured worker threads.
static WORKER_THREADS: LazyLock<RwLock<usize>> = LazyLock::new(|| RwLock::new(1));

/// Global app execution status.
static IS_APP_ACTIVE: LazyLock<RwLock<bool>> = LazyLock::new(|| RwLock::new(true));

/// Update global Celery config.
/// Expected to be called at-most once, during startup.
pub fn set_config(new_config: CeleryConfig) {
    let mut config = CELERY_CONFIG
        .write()
        .expect("failed to write-lock CeleryConfig");
    *config = new_config;
}

/// Update global Celery queues.
/// Expected to be called at-most once, during startup.
pub fn set_queues(new_queues: Vec<String>) {
    *QUEUES.write().expect("failed to write-lock queues") = new_queues;
}

/// Get globally configured Celery queues.
pub fn get_queues() -> Vec<String> {
    QUEUES.read().expect("failed to read-lock queues").clone()
}

/// Update global worker threads.
/// Expected to be called at-most once, during startup.
pub fn set_worker_threads(new_val: usize) {
    *WORKER_THREADS
        .write()
        .expect("failed to write-lock worker_threads") = new_val;
}

/// Get global worker threads.
pub fn get_worker_threads() -> usize {
    *WORKER_THREADS
        .read()
        .expect("failed to read-lock worker_threads")
}

/// Update global app execution status.
#[instrument]
pub fn set_is_app_active(new_val: bool) {
    *IS_APP_ACTIVE
        .write()
        .expect("failed to write-lock is_app_active") = new_val;
}

/// Get global app execution status.
pub fn get_is_app_active() -> bool {
    *IS_APP_ACTIVE
        .read()
        .expect("failed to read-lock is_app_active")
}

/// CELERY_APP holds the high-level Celery application. Note: The Celery app is
/// built separately from the Broker because it handles task routing/scheduling.
static CELERY_APP: OnceCell<Arc<Celery>> = OnceCell::const_new();

/// Returns the global Celery app.
#[instrument]
pub async fn get_celery_app() -> Arc<Celery> {
    CELERY_APP
        .get_or_init(|| async {
            generate_celery_app().await.unwrap_or_else(|err| {
                tracing::error!("{:#}", err);
                panic!("{:#}", err);
            })
        })
        .await
        .clone()
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
                let mut conn_guard = CELERY_CONNECTION.write().await;
                *conn_guard = Some(arc_conn.clone());
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
    let CeleryConfig {
        prefetch_count,
        acks_late,
        task_max_retries,
        broker_connection_max_retries,
        heartbeat_secs,
    } = *CELERY_CONFIG
        .read()
        .map_err(|_| anyhow!("failed to read-lock CeleryConfig"))?;

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
            scheduled_events,
            manage_election_event_date,
            manage_election_event_enrollment,
            manage_election_event_lockdown,
            manage_election_init_report,
            manage_election_voting_period_end,
            generate_manual_verification_report,
            manage_election_allow_tally,
            manage_election_date,
            export_election_event,
            generate_activity_logs_report,
            export_certificate_authority,
            create_transmission_package_task,
            send_transmission_package_task,
            delete_election_event_t,
            export_tasks_execution,
            scheduled_reports,
            review_cast_votes,
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
            process_cast_vote,
            edit_user,
            render_document_pdf,
            execute_plugin_task,
            prepare_publication_preview,
            export_tally_results_to_xlsx_task,
            post_tally_task,
            import_templates_task,
        ],
        task_routes = [
            create_keys::NAME => &Queue::Short.queue_name(&slug),
            review_boards::NAME => &Queue::Beat.queue_name(&slug),
            process_board::NAME => &Queue::Beat.queue_name(&slug),
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
            generate_activity_logs_report::NAME => &Queue::Reports.queue_name(&slug), // Using reports queue because there is more memory allocated for that queue
            export_tasks_execution::NAME => &Queue::ImportExport.queue_name(&slug),
            export_trustees_task::NAME => &Queue::ImportExport.queue_name(&slug),
            import_election_event::NAME => &Queue::ImportExport.queue_name(&slug),
            export_templates::NAME => &Queue::ImportExport.queue_name(&slug),
            export_tenant_config::NAME => &Queue::ImportExport.queue_name(&slug),
            import_tenant_config::NAME => &Queue::ImportExport.queue_name(&slug),
            scheduled_events::NAME => &Queue::Beat.queue_name(&slug),
            scheduled_reports::NAME => &Queue::Beat.queue_name(&slug),
            review_cast_votes::NAME => &Queue::Beat.queue_name(&slug),
            manage_election_date::NAME => &Queue::Beat.queue_name(&slug),
            manage_election_event_date::NAME => &Queue::Beat.queue_name(&slug),
            manage_election_event_enrollment::NAME => &Queue::Beat.queue_name(&slug),
            manage_election_event_lockdown::NAME => &Queue::Beat.queue_name(&slug),
            manage_election_init_report::NAME => &Queue::Beat.queue_name(&slug),
            manage_election_voting_period_end::NAME => &Queue::Beat.queue_name(&slug),
            generate_manual_verification_report::NAME => &Queue::Reports.queue_name(&slug),
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
            prepare_publication_preview::NAME => &Queue::Beat.queue_name(&slug),
            export_tally_results_to_xlsx_task::NAME => &Queue::ImportExport.queue_name(&slug),
            post_tally_task::NAME => &Queue::Reports.queue_name(&slug),
            import_templates_task::NAME => &Queue::ImportExport.queue_name(&slug),
            export_certificate_authority::NAME => &Queue::ImportExport.queue_name(&slug),
            process_cast_vote::NAME => &Queue::Communication.queue_name(&slug),
            edit_user::NAME => &Queue::Short.queue_name(&slug),
        ],
        prefetch_count = prefetch_count,
        acks_late = acks_late,
        task_max_retries = task_max_retries,
        heartbeat = Some(heartbeat_secs),
        broker_connection_max_retries = broker_connection_max_retries,
    )
    .await
    .map_err(|err| anyhow!("{:?}", err))
}

static CELERY_CONNECTION: tokio::sync::RwLock<Option<Arc<Connection>>> =
    tokio::sync::RwLock::const_new(None);

/// Returns a reused AMQP connection wrapped in an Arc.
/// If no connection exists (or if it’s disconnected), a new connection is created and stored.
#[instrument]
pub async fn get_celery_connection() -> Result<Arc<Connection>> {
    let conn_guard = CELERY_CONNECTION.read().await;

    if let Some(conn) = conn_guard.as_ref() {
        if !conn.status().connected() {
            drop(conn_guard); // Release read lock before acquiring write lock

            info!("Existing AMQP connection is disconnected, creating new connection");
            // Create and return a new connection (this will replace the old one)
            return create_connection().await.map(|(connection, _)| connection);
        }
        // Connection is still valid, return clone
        return Ok(conn.clone());
    }
    drop(conn_guard); // Release read lock

    // No connection exists, create a new one
    create_connection().await.map(|(connection, _)| connection)
}
