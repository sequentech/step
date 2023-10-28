// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::hasura;
use crate::types::task_error::into_task_error;
use celery::error::TaskError;
use celery::task::TaskResult;
use chrono::Utc;
use sequent_core::services::connection::AuthHeaders;
use sequent_core::services::openid;
use tracing::instrument;
use tracing::{event, Level};

#[instrument]
#[celery::task]
pub async fn process_board(election_event_id: String) -> TaskResult<()> {
    let auth_headers = openid::get_client_credentials()
        .await
        .map_err(into_task_error)?;
    Ok(())
}
