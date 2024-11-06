// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::{
    services::database::{get_hasura_pool, get_keycloak_pool},
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

    let mut keycloak_db_client = get_keycloak_pool()
        .await
        .get()
        .await
        .with_context(|| "Error acquiring Keycloak DB pool")?;

    let keycloak_transaction = keycloak_db_client
        .transaction()
        .await
        .with_context(|| "Error starting Keycloak transaction")?;

    let _data = generate_report(
        &document_id,
        &tenant_id,
        &election_event_id,
        format,
        GenerateReportMode::REAL,
        &hasura_transaction,
        &keycloak_transaction,
        false,
        None,
    )
    .await
    .with_context(|| "Error generating activity log report")?;

    Ok(())
}
