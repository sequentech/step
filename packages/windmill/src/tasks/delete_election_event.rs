// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::{services::delete_election_event::delete_keycloak_realm, types::error::Result};
use celery::error::TaskError;
use tracing::instrument;

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task]
pub async fn delete_election_event_t(id: String, realm: String) -> Result<()> {
    delete_keycloak_realm(&realm).await?;
    // TODO: delete all related from db
    Ok(())
}
