// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::{
    services::{database::{get_hasura_pool, get_keycloak_pool}, reports::{activity_log::{ActivityLogsTemplate, ReportFormat}, template_renderer::{GenerateReportMode, TemplateRenderer}}},
    types::error::Result,
};
use anyhow::{anyhow, Context};
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

    let report = ActivityLogsTemplate::new(
        tenant_id.clone(),
        election_event_id.clone(),
        // TODO: add missing `format`
    );

    report
        .execute_report(
            &document_id,
            &tenant_id,
            &election_event_id,
            /* is_scheduled_task */ false,
            /* recipients */ vec![],
            /* pdf_options */ None,
            GenerateReportMode::REAL,
            &hasura_transaction,
            &keycloak_transaction,
        )
        .await
        .map_err(|err| anyhow!("error generating report: {err:?}"));

    hasura_transaction
        .commit()
        .await
        .with_context(|| "Failed to commit Hasura transaction")?;

    Ok(())
}
