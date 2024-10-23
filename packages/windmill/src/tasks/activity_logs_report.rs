// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::{
    postgres::reports::Report,
    services::database::get_hasura_pool,
    services::reports::activity_log::{generate_report, ReportFormat},
    services::reports::template_renderer::GenerateReportMode,
    types::error::Result,
};
use anyhow::Context;
use celery::error::TaskError;
use deadpool_postgres::Client as DbClient;
use tracing::instrument;

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn generate_activity_logs_report(
    tenant_id: String,
    election_event_id: String,
    document_id: String,
    format: ReportFormat,
    report: Option<Report>,
) -> Result<()> {
    let mut db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .with_context(|| "Error getting DB pool")?;

    let hasura_transaction = db_client
        .transaction()
        .await
        .with_context(|| "Error starting transaction")?;

    let _data = generate_report(
        &document_id,
        &tenant_id,
        &election_event_id,
        format,
        report.unwrap(),
        GenerateReportMode::REAL,
        Some(&hasura_transaction),
        None,
    )
    .await
    .with_context(|| "Error generating activity log report")?;

    Ok(())
}
