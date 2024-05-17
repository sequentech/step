// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::{services::export_election_event::process_export, types::error::Result};
use anyhow::{anyhow, Context};
use celery::error::TaskError;
use tracing::{event, instrument, Level};

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn export_election_event(
    tenant_id: String,
    election_event_id: String,
    document_id: String,
) -> Result<()> {
    let data = process_export(&tenant_id, &election_event_id, &document_id).await?;

    Ok(())
}
