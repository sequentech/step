// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use async_once::AsyncOnce;
use celery::export::Arc;
use celery::Celery;
use std;
use tracing::{event, instrument, Level};

use crate::tasks::create_ballot_style::create_ballot_style;
use crate::tasks::create_keys::create_keys;
use crate::tasks::execute_tally_session::execute_tally_session;
use crate::tasks::insert_ballots::insert_ballots;
use crate::tasks::insert_election_event::insert_election_event_t;
use crate::tasks::insert_tenant::insert_tenant;
use crate::tasks::process_board::process_board;
use crate::tasks::render_report::render_report;
use crate::tasks::review_boards::review_boards;
use crate::tasks::set_public_key::set_public_key;
use crate::tasks::tally_election_event::tally_election_event;
use crate::tasks::update_election_event_ballot_styles::update_election_event_ballot_styles;
use crate::tasks::update_voting_status::update_voting_status;

static mut PREFETCH_COUNT_S: u16 = 100;
static mut ACKS_LATE_S: bool = true;

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

#[instrument]
pub async fn generate_celery_app() -> Arc<Celery> {
    let prefetch_count: u16;
    let acks_late: bool;
    unsafe {
        prefetch_count = PREFETCH_COUNT_S;
        acks_late = ACKS_LATE_S;
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
            create_ballot_style,
            create_keys,
            insert_ballots,
            review_boards,
            process_board,
            render_report,
            set_public_key,
            tally_election_event,
            execute_tally_session,
            update_election_event_ballot_styles,
            update_voting_status,
            insert_election_event_t,
            insert_tenant,
        ],
        // Route certain tasks to certain queues based on glob matching.
        task_routes = [
            "create_ballot_style" => "short_queue",
            "create_keys" => "short_queue",
            "insert_ballots" => "tally_queue",
            "review_boards" => "beat",
            "process_board" => "beat",
            "render_report" => "reports_queue",
            "set_public_key" => "short_queue",
            "tally_election_event" => "tally_queue",
            "execute_tally_session" => "tally_queue",
            "update_election_event_ballot_styles" => "short_queue",
            "update_voting_status" => "short_queue",
            "insert_election_event_t" => "short_queue",
            "insert_tenant" => "short_queue",
        ],
        prefetch_count = prefetch_count,
        acks_late = acks_late,
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
