// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use celery::error::TaskError;
use celery::task::TaskResult;
use chrono::Utc;
use sequent_core::services::connection::AuthHeaders;
use sequent_core::services::openid;
use tracing::instrument;
use tracing::{event, Level};

use crate::hasura;
use crate::types::task_error::into_task_error;

#[instrument]
#[celery::task]
pub async fn process_board(election_event_id: String, tenant_id: String) -> TaskResult<()> {
    let auth_headers = openid::get_client_credentials()
        .await
        .map_err(into_task_error)?;
        // fetch election_event
    let hasura_response = hasura::election_event::get_election_event(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
    )
    .await
    .map_err(into_task_error)?;
    let election_event = &hasura_response
        .data
        .expect("expected data".into())
        .sequent_backend_election_event[0];
    Ok(())
}
