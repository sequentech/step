// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::{
    services::reports::electoral_log::{generate_report, ReportFormat},
    types::error::Result,
};
use celery::error::TaskError;
use tracing::instrument;

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn generate_activity_logs_report(
    tenant_id: String,
    election_event_id: String,
    document_id: String,
    format: ReportFormat,
) -> Result<()> {
    let _data = generate_report(&tenant_id, &election_event_id, &document_id, format).await?;

    Ok(())
}
