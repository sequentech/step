// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::ceremonies::tally_ceremony::get_tally_session;
use crate::types::error::{Error, Result};
use anyhow::anyhow;
use celery::prelude::TaskError;
use sequent_core::services::connection;
use sequent_core::services::keycloak;
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
    let tally_session = get_tally_session(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
        tally_session_id.clone(),
    )
    .await?;
    Ok(())
}
