// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use async_once::AsyncOnce;
use celery::export::Arc;
use celery::prelude::Task;
use celery::Celery;
use std;
use std::convert::AsRef;
use strum_macros::AsRefStr;
use tracing::{event, instrument, Level};

use crate::tasks::create_keys::create_keys;
use crate::tasks::create_vote_receipt::create_vote_receipt;
use crate::tasks::delete_election_event::delete_election_event_t;
use crate::tasks::execute_tally_session::execute_tally_session;
use crate::tasks::export_election_event::export_election_event;
use crate::tasks::export_election_event_logs::export_election_event_logs;
use crate::tasks::export_tasks_execution::export_tasks_execution;
use crate::tasks::export_templates::export_templates;
use crate::tasks::export_users::export_users;
use crate::tasks::import_election_event::import_election_event;
use crate::tasks::import_users::import_users;
use crate::tasks::insert_election_event::insert_election_event_t;
use crate::tasks::insert_tenant::insert_tenant;
use crate::tasks::manage_election_dates::manage_election_date;
use crate::tasks::manage_election_event_date::manage_election_event_date;
use crate::tasks::manual_verification_report::generate_manual_verification_report;
use crate::tasks::miru_plugin_tasks::create_transmission_package_task;
use crate::tasks::miru_plugin_tasks::send_transmission_package_task;
use crate::tasks::process_board::process_board;
use crate::tasks::render_report::render_report;
use crate::tasks::review_boards::review_boards;
use crate::tasks::scheduled_events::scheduled_events;
use crate::tasks::send_template::send_template;
use crate::tasks::set_public_key::set_public_key;
use crate::tasks::update_election_event_ballot_styles::update_election_event_ballot_styles;

#[derive(AsRefStr, Debug)]
enum Queue {
    #[strum(serialize = "short_queue")]
    Short,
    #[strum(serialize = "beat")]
    Beat,
    #[strum(serialize = "communication_queue")]
    Communication,
    #[strum(serialize = "tally_queue")]
    Tally,
    #[strum(serialize = "reports_queue")]
    Reports,
    #[strum(serialize = "import_export_queue")]
    ImportExport,
}

static mut PREFETCH_COUNT_S: u16 = 100;
static mut ACKS_LATE_S: bool = true;
static mut TASK_MAX_RETRIES: u32 = 4;
static mut IS_APP_ACTIVE: bool = true;
static mut BROKER_CONNECTION_MAX_RETRIES: u32 = 5;
static mut HEARTBEAT_SECS: u16 = 10;

pub fn set_prefetch_count(new_val: u16) {
    unsafe {
        PREFETCH_COUNT_S = new_val;
    }
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
            create_vote_receipt,
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
            manage_election_date,
            export_election_event,
            export_election_event_logs,
            create_transmission_package_task,
            send_transmission_package_task,
            delete_election_event_t,
            export_tasks_execution,
            export_templates,
        ],
        // Route certain tasks to certain queues based on glob matching.
        task_routes = [
            create_keys::NAME => Queue::Short.as_ref(),
            review_boards::NAME => Queue::Beat.as_ref(),
            process_board::NAME => Queue::Beat.as_ref(),
            generate_manual_verification_report::NAME => Queue::Reports.as_ref(),
            render_report::NAME => Queue::Reports.as_ref(),
            create_vote_receipt::NAME => Queue::Reports.as_ref(),
            set_public_key::NAME => Queue::Short.as_ref(),
            execute_tally_session::NAME => Queue::Tally.as_ref(),
            update_election_event_ballot_styles::NAME => Queue::Short.as_ref(),
            insert_election_event_t::NAME => Queue::Short.as_ref(),
            insert_tenant::NAME => Queue::Short.as_ref(),
            send_template::NAME => Queue::Communication.as_ref(),
            import_users::NAME => Queue::ImportExport.as_ref(),
            export_users::NAME => Queue::ImportExport.as_ref(),
            export_election_event::NAME => Queue::ImportExport.as_ref(),
            export_election_event_logs::NAME => Queue::ImportExport.as_ref(),
            export_tasks_execution::NAME => Queue::ImportExport.as_ref(),
            import_election_event::NAME => Queue::ImportExport.as_ref(),
            export_templates::NAME => Queue::ImportExport.as_ref(),
            scheduled_events::NAME => Queue::Beat.as_ref(),
            manage_election_date::NAME => Queue::Beat.as_ref(),
            manage_election_event_date::NAME => Queue::Beat.as_ref(),
            create_transmission_package_task::NAME => Queue::Short.as_ref(),
            send_transmission_package_task::NAME => Queue::Short.as_ref(),
            delete_election_event_t::NAME => Queue::Short.as_ref(),
        ],
        prefetch_count = prefetch_count,
        acks_late = acks_late,
        task_max_retries = task_max_retries,
        heartbeat = Some(heartbeat),
        broker_connection_max_retries = broker_connection_max_retries,
    ).await.unwrap()
}

lazy_static! {
    static ref CELERY_APP: AsyncOnce<Arc<Celery>> =
        AsyncOnce::new(async { generate_celery_app().await });
}

pub async fn get_celery_app() -> Arc<Celery> {
    CELERY_APP.get().await.clone()
}
