// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use async_once::AsyncOnce;
use celery::export::Arc;
use celery::Celery;
use std;
use tracing::{event, instrument, Level};

use crate::tasks::create_keys::create_keys;
use crate::tasks::create_vote_receipt::create_vote_receipt;
use crate::tasks::execute_tally_session::execute_tally_session;
use crate::tasks::export_election_event::export_election_event;
use crate::tasks::export_users::export_users;
use crate::tasks::import_election_event::import_election_event;
use crate::tasks::import_users::import_users;
use crate::tasks::insert_election_event::insert_election_event_t;
use crate::tasks::insert_tenant::insert_tenant;
use crate::tasks::manage_election_date::manage_election_date;
use crate::tasks::manual_verification_pdf::get_manual_verification_pdf;
use crate::tasks::process_board::process_board;
use crate::tasks::render_report::render_report;
use crate::tasks::review_boards::review_boards;
use crate::tasks::scheduled_events::scheduled_events;
use crate::tasks::send_communication::send_communication;
use crate::tasks::set_public_key::set_public_key;
use crate::tasks::update_election_event_ballot_styles::update_election_event_ballot_styles;

static mut PREFETCH_COUNT_S: u16 = 100;
static mut ACKS_LATE_S: bool = true;
static mut TASK_MAX_RETRIES: u32 = 4;

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
pub async fn generate_celery_app() -> Arc<Celery> {
    let prefetch_count: u16;
    let acks_late: bool;
    let task_max_retries: u32;
    unsafe {
        prefetch_count = PREFETCH_COUNT_S;
        acks_late = ACKS_LATE_S;
        task_max_retries = TASK_MAX_RETRIES;
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
            send_communication,
            import_users,
            export_users,
            import_election_event,
            get_manual_verification_pdf,
            scheduled_events,
            manage_election_date,
            export_election_event,
        ],
        // Route certain tasks to certain queues based on glob matching.
        task_routes = [
            "create_keys" => "short_queue",
            "get_manual_verification_pdf" => "short_queue",
            "review_boards" => "beat",
            "process_board" => "beat",
            "render_report" => "reports_queue",
            "create_vote_receipt" => "reports_queue",
            "set_public_key" => "short_queue",
            "execute_tally_session" => "tally_queue",
            "update_election_event_ballot_styles" => "short_queue",
            "update_voting_status" => "short_queue",
            "insert_election_event_t" => "short_queue",
            "insert_tenant" => "short_queue",
            "send_communication" => "communication_queue",
            "import_users" => "import_export_queue",
            "export_users" => "import_export_queue",
            "export_election_event" => "import_export_queue",
            "import_election_event" => "import_export_queue",
            "scheduled_events" => "beat",
            "manage_election_date" => "beat"
        ],
        prefetch_count = prefetch_count,
        acks_late = acks_late,
        task_max_retries = task_max_retries,
        heartbeat = Some(10),
    ).await.unwrap()
}

lazy_static! {
    static ref CELERY_APP: AsyncOnce<Arc<Celery>> =
        AsyncOnce::new(async { generate_celery_app().await });
}

pub async fn get_celery_app() -> Arc<Celery> {
    CELERY_APP.get().await.clone()
}
