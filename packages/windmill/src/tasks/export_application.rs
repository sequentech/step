// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::{services::export::export_application::process_export, types::error::Result};
use anyhow::{anyhow, Context};
use celery::error::TaskError;
use tracing::{event, info, instrument, Level};

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn export_application(
    tenant_id: String,
    election_event_id: String,
    election_id: Option<String>,
    document_id: String,
) -> Result<()> {
    process_export(&tenant_id, &election_event_id, election_id, &document_id).await?;

    Ok(())
}
