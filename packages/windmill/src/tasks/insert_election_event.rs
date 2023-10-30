// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use celery::prelude::*;
use sequent_core::services::connection;
use tracing::instrument;

use crate::hasura::election_event::insert_election_event;

#[instrument(skip_all)]
#[celery::task]
pub async fn insert_election_event_t(
    auth_headers: connection::AuthHeaders,
    object: insert_election_event::sequent_backend_election_event_insert_input,
) -> TaskResult<()> {
    // TODO
    Ok(())
}
