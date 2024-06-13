// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::{services::export_election_event_logs::process_export, types::error::Result};
use anyhow::{anyhow, Context};
use celery::error::TaskError;
use tracing::{event, instrument, Level};

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn export_election_event_logs(
    tenant_id: String,
    election_event_id: String,
    document_id: String,
    log_data:String
) -> Result<()> {
    let data = process_export(&tenant_id, &election_event_id, &document_id, &log_data).await?;

    Ok(())
}
