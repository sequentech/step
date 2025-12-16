// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::tasks_semaphore::acquire_semaphore;
use crate::{
    postgres::reports::Report,
    services::{
        database::{get_hasura_pool, get_keycloak_pool},
        reports::{
            activity_log::{ActivityLogsTemplate, ReportFormat},
            template_renderer::{
                GenerateReportMode, ReportOriginatedFrom, ReportOrigins, TemplateRenderer,
            },
        },
    },
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
    report_clone: Option<Report>,
) -> Result<()> {
    let _permit = acquire_semaphore().await?;
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
        ReportOrigins {
            tenant_id: tenant_id.clone(),
            election_event_id: election_event_id.clone(),
            election_id: None,
            template_alias: None,
            voter_id: None,
            report_origin: ReportOriginatedFrom::ExportFunction,
            executer_username: None,
            tally_session_id: None,
        },
        format,
    );

    let _ = report
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
            /* task_execution */ None,
        )
        .await
        .map_err(|err| anyhow!("error generating report: {err:?}"));

    hasura_transaction
        .commit()
        .await
        .with_context(|| "Failed to commit Hasura transaction")?;

    Ok(())
}
