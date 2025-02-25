// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::reports::Report;
use crate::services::database::{get_hasura_pool, get_keycloak_pool, PgConfig};
use crate::services::reports::manual_verification::ManualVerificationTemplate;
use crate::services::reports::template_renderer::{
    GenerateReportMode, ReportOriginatedFrom, ReportOrigins, TemplateRenderer,
};
use crate::types::error::Error;
use crate::types::error::Result;
use anyhow::{anyhow, Context, Result as AnyhowResult};
use celery::error::TaskError;
use deadpool_postgres::{Client as DbClient, Transaction};
use tracing::instrument;

#[instrument(err)]
pub async fn generate_report(
    document_id: &str,
    tenant_id: &str,
    election_event_id: &str,
    voter_id: &str,
    mode: GenerateReportMode,
    report_clone: Option<Report>,
) -> AnyhowResult<()> {
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

    let report = ManualVerificationTemplate::new(ReportOrigins {
        tenant_id: tenant_id.to_string(),
        election_event_id: election_event_id.to_string(),
        election_id: None,
        template_alias: None,
        voter_id: Some(voter_id.to_string()),
        report_origin: ReportOriginatedFrom::ExportFunction,
        executer_username: None, //TODO: fix?
        tally_session_id: None,
        user_timezone: None,
    });

    report
        .execute_report(
            &document_id,
            &tenant_id,
            &election_event_id,
            /* is_scheduled_task */ false,
            /* recipients */ vec![],
            GenerateReportMode::REAL,
            report_clone,
            &hasura_transaction,
            &keycloak_transaction,
            None,
        )
        .await
        .map_err(|err| anyhow!("Error generating ballot receipt report: {err:?}"))?;

    hasura_transaction
        .commit()
        .await
        .with_context(|| "Failed to commit Hasura transaction")?;

    Ok(())
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task]
pub async fn generate_manual_verification_report(
    document_id: String,
    tenant_id: String,
    election_event_id: String,
    voter_id: String,
    report: Option<Report>,
) -> Result<()> {
    // Spawn the task using an async block
    let handle = tokio::task::spawn_blocking({
        move || {
            tokio::runtime::Handle::current().block_on(async move {
                generate_report(
                    &document_id,
                    &tenant_id,
                    &election_event_id,
                    &voter_id,
                    GenerateReportMode::REAL,
                    report,
                )
                .await
            })
        }
    });

    // Await the result and handle JoinError explicitly
    match handle.await {
        Ok(inner_result) => inner_result.map_err(|err| Error::from(err.context("Task failed"))),
        Err(join_error) => Err(Error::from(anyhow!("Task panicked: {}", join_error))),
    }?;

    Ok(())
}
