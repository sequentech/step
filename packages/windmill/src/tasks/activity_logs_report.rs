// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::{
    services::reports::election_event_activity_logs::generate_activity_logs_report,
    types::error::Result,
};
use anyhow::{anyhow, Context};
use celery::error::TaskError;
use tracing::{event, instrument, Level};

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn generate_activity_logs_report(
    tenant_id: String,
    election_event_id: String,
    document_id: String,
    format: String,
) -> Result<()> {
    let data = generate_activity_logs_report(&tenant_id, &election_event_id, &document_id, &format)
        .await?;

    Ok(())
}
