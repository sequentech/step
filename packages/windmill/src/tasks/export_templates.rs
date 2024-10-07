// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::{services::export_template::process_export, types::error::Result};
use anyhow::{anyhow, Context};
use celery::error::TaskError;
use tracing::{event, info, instrument, Level};

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn export_templates(tenant_id: String, document_id: String) -> Result<()> {
    info!("Exporting templates task");
    println!("Exporting templates task");
    let data = process_export(&tenant_id, None, &document_id).await?;

    Ok(())
}
