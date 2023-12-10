// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::hasura::tally_session::update_tally_session_status;
use crate::hasura::tally_session_execution::insert_tally_session_execution;
use crate::services::ceremonies::tally_ceremony::*;
use crate::types::error::Result;
use celery::prelude::TaskError;
use sequent_core::services::keycloak;
use sequent_core::types::ceremonies::*;
use tracing::{event, instrument, Level};

#[instrument]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(time_limit = 120000)]
pub async fn connect_tally_ceremony(
    tenant_id: String,
    election_event_id: String,
    tally_session_id: String,
) -> Result<()> {
    let auth_headers = keycloak::get_client_credentials().await?;

    let Some((tally_session_execution, tally_session)) = find_last_tally_session_execution(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
        tally_session_id.clone(),
    ).await? else {
        event!(Level::INFO, "Missing tally session/execution");
        return Ok(());
    };
    let execution_status = tally_session.execution_status.unwrap_or("".into());
    if execution_status != TallyExecutionStatus::STARTED.to_string() {
        event!(
            Level::INFO,
            "Unexpected execution status {}",
            execution_status
        );
        return Ok(());
    }
    let status = get_tally_ceremony_status(tally_session_execution.status)?;
    let mut new_status = status.clone();
    new_status.trustees = new_status
        .trustees
        .iter()
        .map(|trustee| {
            let mut new_trustee = trustee.clone();
            new_trustee.status = TallyTrusteeStatus::KEY_RESTORED;
            new_trustee
        })
        .collect();
    update_tally_session_status(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
        tally_session_id.clone(),
        TallyExecutionStatus::CONNECTED,
    )
    .await?;
    insert_tally_session_execution(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
        -1,
        tally_session_id.clone(),
        None,
        Some(new_status),
        None,
    )
    .await?;

    Ok(())
}
