// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{Context, Result};
use celery::error::TaskError;
use celery::export::Arc;
use celery::prelude::*;
use celery::Celery;
use sequent_core::ballot::ElectionEventStatus;
use sequent_core::services::connection;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{event, instrument, Level};

use crate::hasura;
use crate::hasura::election_event::{insert_election_event, insert_election_event_f};
use crate::services::celery_app::*;
use crate::services::election_event_board::{get_election_event_board, BoardSerializable};
use crate::services::public_keys;
use crate::tasks::set_public_key::set_public_key;
use crate::types::scheduled_event::ScheduledEvent;

#[instrument(skip_all)]
#[celery::task]
pub async fn insert_election_event_t(
    auth_headers: connection::AuthHeaders,
    object: insert_election_event::sequent_backend_election_event_insert_input,
) -> TaskResult<()> {
    // TODO
    Ok(())
}
