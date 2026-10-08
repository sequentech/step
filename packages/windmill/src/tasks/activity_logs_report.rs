// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::tasks_semaphore::acquire_semaphore;
use crate::{
    postgres::reports::Report,
    services::{
        database::{get_hasura_pool, get_keycloak_pool},
        reports::{
            activity_log::{ActivityLogExportOptions, ActivityLogsTemplate, ReportFormat},
            template_renderer::{
                GenerateReportMode, ReportOriginatedFrom, ReportOrigins, TemplateRenderer,
            },
        },
        tasks_execution::*,
    },
    types::error::Result,
};
use anyhow::{anyhow, Context};
use celery::error::TaskError;
use deadpool_postgres::Client as DbClient;
use sequent_core::types::hasura::core::TasksExecution;
use tracing::instrument;

/// An export from the Logs tab. It is no configured report: it is written
/// once, as one file in the format the administrator asked for, whatever
/// copies and formats the event's activity logs report is set to. That
/// report is generated from the Reports tab.
pub fn logs_tab_export(
    tenant_id: &str,
    election_event_id: &str,
    format: ReportFormat,
    options: ActivityLogExportOptions,
    requested_by: Option<String>,
) -> ActivityLogsTemplate {
    ActivityLogsTemplate::new(
        ReportOrigins {
            tenant_id: tenant_id.to_string(),
            election_event_id: election_event_id.to_string(),
            election_id: None,
            template_alias: None,
            voter_id: None,
            report_origin: ReportOriginatedFrom::ExportFunction,
            executer_username: requested_by,
            tally_session_id: None,
        },
        format,
    )
    .with_options(options)
}

async fn generate_activity_logs_report_impl(
    tenant_id: String,
    election_event_id: String,
    document_id: String,
    format: ReportFormat,
    options: ActivityLogExportOptions,
    requested_by: Option<String>,
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

    let report = logs_tab_export(
        &tenant_id,
        &election_event_id,
        format,
        options,
        requested_by,
    );

    report
        .execute_report(
            &document_id,
            &tenant_id,
            &election_event_id,
            /* is_scheduled_task */ false,
            /* recipients */ vec![],
            GenerateReportMode::REAL,
            /* report */ None,
            &hasura_transaction,
            &keycloak_transaction,
            /* task_execution */ None,
            /* may_read_secret_attributes */ false,
        )
        .await
        .map_err(|err| anyhow!("error generating report: {err:?}"))?;

    hasura_transaction
        .commit()
        .await
        .with_context(|| "Failed to commit Hasura transaction")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::postgres::reports::ReportFormat as OutputFormat;
    use crate::postgres::reports::ReportType;
    use crate::services::reports::template_renderer::{generation_copies, EReportEncryption};

    fn configured() -> Report {
        Report {
            id: "report".to_string(),
            election_event_id: "event".to_string(),
            tenant_id: "tenant".to_string(),
            election_id: None,
            report_type: ReportType::ACTIVITY_LOGS.to_string(),
            template_alias: None,
            encryption_policy: EReportEncryption::Unencrypted,
            cron_config: None,
            created_at: chrono::Utc::now(),
            permission_label: None,
            copies: Some(7),
            output_formats: Some(vec![
                OutputFormat::Pdf,
                OutputFormat::Csv,
                OutputFormat::Sql,
            ]),
        }
    }

    #[test]
    fn an_export_from_the_logs_tab_is_one_file_in_the_format_asked_for() {
        for (format, written) in [
            (ReportFormat::PDF, OutputFormat::Pdf),
            (ReportFormat::CSV, OutputFormat::Csv),
        ] {
            let export = logs_tab_export(
                "tenant",
                "event",
                format,
                ActivityLogExportOptions::default(),
                Some("admin".to_string()),
            );
            assert_eq!(export.output_formats(None).unwrap(), vec![written]);
            // Whatever the event's activity logs report is set to.
            assert_eq!(
                export.output_formats(Some(&configured())).unwrap(),
                vec![written]
            );
            assert_eq!(export.requested_by(), Some("admin".to_string()));
            assert!(matches!(
                export.get_report_origin(),
                ReportOriginatedFrom::ExportFunction
            ));
        }
        // It is generated without a report, so it is written once.
        assert_eq!(generation_copies(None), 1);
    }
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn generate_activity_logs_report(
    tenant_id: String,
    election_event_id: String,
    document_id: String,
    format: ReportFormat,
    // Never read: a queued task may still carry a report.
    _report: Option<Report>,
    task_execution: TasksExecution,
    // The Logs tab's range and zone; absent in tasks queued before it existed.
    options: Option<ActivityLogExportOptions>,
) -> Result<()> {
    match generate_activity_logs_report_impl(
        tenant_id,
        election_event_id,
        document_id.clone(),
        format,
        options.unwrap_or_default(),
        Some(task_execution.executed_by_user.clone()),
    )
    .await
    {
        Ok(()) => {
            update_complete(&task_execution, Some(document_id))
                .await
                .context("Failed to update task execution status to COMPLETED")?;
            Ok(())
        }
        Err(err) => {
            if let Err(update_err) = update_fail(&task_execution, &format!("{err:?}")).await {
                tracing::error!(
                    "Failed to update task execution status to FAILED: {:?}",
                    update_err
                );
            }
            Err(err)
        }
    }
}
