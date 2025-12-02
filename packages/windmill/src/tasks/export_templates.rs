// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::{services::export::export_template::process_export, types::error::Result};
use anyhow::{anyhow, Context};
use celery::error::TaskError;
use tracing::{event, info, instrument, Level};

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn export_templates(tenant_id: String, document_id: String) -> Result<()> {
    let data = process_export(&tenant_id, &document_id).await?;

    Ok(())
}
