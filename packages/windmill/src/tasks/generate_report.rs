// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::reports::Report;
use crate::postgres::reports::ReportType;
use crate::services::database::get_hasura_pool;
use crate::services::database::get_keycloak_pool;
use crate::services::reports::template_renderer::{
    GenerateReportMode, ReportOriginatedFrom, ReportOrigins, ReportOutcome, TemplateRenderer,
};
use crate::services::reports::{
    activity_log::ActivityLogsTemplate, ballot_images::BallotImagesTemplate,
    ballot_receipt::BallotTemplate, electoral_results::ElectoralResults,
    initialization::InitializationTemplate, manual_verification::ManualVerificationTemplate,
    participation::ParticipationReportTemplate,
    voter_information_letter::VoterInformationLetterTemplate,
};
use crate::services::serialize_tasks_logs::append_general_log;
use crate::services::signing::actions::reports::{
    held_by_the_tally, start_report_signing, system_requester, ReportToSign,
};
use crate::services::signing::SigningCaller;
use crate::services::tasks_execution::update_fail;
use crate::services::tasks_execution::update_with_annotations;
use crate::services::tasks_semaphore::acquire_semaphore;
use crate::types::error::Error;
use crate::types::error::Result;
use anyhow::{anyhow, Context};
use celery::error::TaskError;
use deadpool_postgres::Client as DbClient;
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::types::hasura::extra::TasksExecutionStatus;
use serde_json::json;
use std::str::FromStr;
use tracing::info;
use tracing::instrument;
use uuid::Uuid;

/// The event's activity logs report as the Reports tab generates it: in
/// the formats its report asks for. Its copies are the report's too.
pub fn configured_activity_logs(
    ids: ReportOrigins,
    report: &Report,
) -> Result<ActivityLogsTemplate, anyhow::Error> {
    let formats = ReportType::ACTIVITY_LOGS
        .generation_formats(report.output_formats.as_deref())
        .map_err(|refusal| anyhow!(refusal))?;
    Ok(ActivityLogsTemplate::in_formats(ids, formats))
}

pub async fn generate_report(
    report: Report,
    document_id: String,
    report_mode: GenerateReportMode,
    is_scheduled_task: bool,
    task_execution: Option<TasksExecution>,
    executer_username: Option<String>,
    tally_session_id: Option<String>,
    may_read_secret_attributes: bool,
    requester: Option<SigningCaller>,
) -> Result<(), anyhow::Error> {
    let tenant_id = report.tenant_id.clone();
    let election_event_id = report.election_event_id.clone();
    let report_type_str = report.report_type.clone();
    let report_clone = report.clone();
    // Clone the election id if it exists
    let election_id = report.election_id.clone();
    let template_alias = report.template_alias.clone();
    let ids = ReportOrigins {
        tenant_id,
        election_event_id,
        election_id,
        template_alias,
        voter_id: None,
        report_origin: ReportOriginatedFrom::ReportsTab, // Assuming this is visited only frrom the Reports tab
        executer_username: executer_username.clone(),
        tally_session_id,
    };

    let mut db_client: DbClient = match get_hasura_pool().await.get().await {
        Ok(client) => client,
        Err(err) => {
            if let Some(ref task_exec) = task_execution {
                let _ = update_fail(task_exec, "Failed to get Hasura DB pool").await;
            }
            return Err(anyhow!("Error getting Hasura DB pool: {}", err));
        }
    };

    let hasura_transaction = match db_client.transaction().await {
        Ok(transaction) => transaction,
        Err(err) => {
            if let Some(ref task_exec) = task_execution {
                let _ = update_fail(task_exec, "Failed to get Hasura DB pool").await;
            };
            return Err(anyhow!("Error starting Hasura transaction: {err}"));
        }
    };
    let mut keycloak_db_client = match get_keycloak_pool().await.get().await {
        Ok(client) => client,
        Err(err) => {
            if let Some(ref task_exec) = task_execution {
                let _ = update_fail(task_exec, "Failed to get Hasura DB pool").await;
            }
            return Err(anyhow!("Error getting Keycloak DB pool: {}", err));
        }
    };

    let keycloak_transaction = match keycloak_db_client.transaction().await {
        Ok(transaction) => transaction,
        Err(err) => {
            if let Some(ref task_exec) = task_execution {
                let _ = update_fail(task_exec, "Failed to get Hasura DB pool").await;
            }
            return Err(anyhow!("Error starting Keycloak transaction: {err}"));
        }
    };

    // The election returns and the initialization report are produced and
    // held by the tally while their rule needs signatures: never generated
    // (released, mailed) here.
    if report_mode == GenerateReportMode::REAL {
        if let Ok(report_type) = ReportType::from_str(&report_type_str) {
            let parse = |value: &str| Uuid::parse_str(value).context("Invalid report ids");
            if held_by_the_tally(
                &hasura_transaction,
                parse(&report.tenant_id)?,
                parse(&report.election_event_id)?,
                &report_type,
            )
            .await?
            {
                return Err(anyhow!(
                    "This report needs signatures: the tally produces it for each Post, to be signed there"
                ));
            }
        }
    }

    let email_recipients = report.cron_config.unwrap_or_default().email_recipients;

    // Helper macro to reduce duplication in execute_report call. It says
    // whether the report was released or awaits signatures.
    macro_rules! execute_report {
        ($report:expr) => {
            $report
                .execute_report_outcome(
                    &document_id,
                    &report.tenant_id,
                    &report.election_event_id,
                    is_scheduled_task,
                    email_recipients,
                    report_mode,
                    Some(report_clone),
                    &hasura_transaction,
                    &keycloak_transaction,
                    task_execution.clone(),
                    may_read_secret_attributes,
                )
                .await?
        };
    }
    let outcome = match ReportType::from_str(&report_type_str) {
        Ok(ReportType::INITIALIZATION_REPORT) => {
            let report = InitializationTemplate::new(ids);
            execute_report!(report)
        }
        Ok(ReportType::ELECTORAL_RESULTS) => {
            let report = ElectoralResults::new(ids);
            execute_report!(report)
        }
        Ok(ReportType::BALLOT_IMAGES) => {
            let report = BallotImagesTemplate::new(ids);
            execute_report!(report)
        }
        Ok(ReportType::BALLOT_RECEIPT) => {
            let report = BallotTemplate::new(ids, None);
            execute_report!(report)
        }
        Ok(ReportType::ACTIVITY_LOGS) => {
            let report = configured_activity_logs(ids, &report_clone)?;
            execute_report!(report)
        }
        Ok(ReportType::MANUAL_VERIFICATION) => {
            let report = ManualVerificationTemplate::new(ids);
            execute_report!(report)
        }
        Ok(ReportType::PARTICIPATION_REPORT) => {
            let report = ParticipationReportTemplate::new(ids);
            execute_report!(report)
        }
        Ok(ReportType::CREDENTIALS) => {
            if report_mode != GenerateReportMode::PREVIEW {
                return Err(anyhow!(
                    "Voter Information Letters must be generated from the voter action"
                ));
            }
            let report = VoterInformationLetterTemplate::new_preview(ids);
            execute_report!(report)
        }
        Err(err) => return Err(anyhow!("{:?}", err)),
    };

    // A report held for its signatures waits for its request: the task
    // succeeds once the request started, and says which one.
    let mut signing_request = None;
    if let ReportOutcome::AwaitingSignatures(base, release) = outcome {
        let parse = |value: &str, what: &str| {
            Uuid::parse_str(value).with_context(|| format!("Error parsing the {what}"))
        };
        let to_sign = ReportToSign {
            tenant_id: parse(&report.tenant_id, "tenant id")?,
            election_event_id: parse(&report.election_event_id, "election event id")?,
            election_id: report
                .election_id
                .as_deref()
                .map(|id| parse(id, "election id"))
                .transpose()?,
            area_id: None,
            report_type: report.report_type.clone(),
            template_id: report.template_alias.clone(),
            subject_key: report.id.clone(),
            base,
            release,
        };
        // A scheduled report's request is its cron's executer's.
        let requester = requester
            .unwrap_or_else(|| system_requester(executer_username.as_deref().unwrap_or("system")));
        signing_request = Some(
            start_report_signing(&hasura_transaction, &requester, &to_sign)
                .await
                .map_err(|error| anyhow!("Error starting the report's signing request: {error}"))?,
        );
    }

    hasura_transaction
        .commit()
        .await
        .with_context(|| "Failed to commit Hasura transaction")?;

    if let (Some(summary), Some(task)) = (signing_request, &task_execution) {
        let logs = serde_json::to_value(append_general_log(
            &task.logs,
            "Generated; awaiting signatures before its release",
        ))?;
        update_with_annotations(
            &task.tenant_id,
            &task.id,
            TasksExecutionStatus::SUCCESS,
            logs,
            json!({ "signing_request": summary }),
        )
        .await
        .context("Failed to update the task execution")?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::postgres::reports::ReportFormat as OutputFormat;
    use crate::services::reports::template_renderer::{generation_copies, EReportEncryption};

    fn ids() -> ReportOrigins {
        ReportOrigins {
            tenant_id: "tenant".to_string(),
            election_event_id: "event".to_string(),
            election_id: None,
            template_alias: None,
            voter_id: None,
            report_origin: ReportOriginatedFrom::ReportsTab,
            executer_username: Some("admin".to_string()),
            tally_session_id: None,
        }
    }

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
    fn the_configured_activity_logs_report_honours_its_copies_and_formats() {
        let report = configured();
        let generated = configured_activity_logs(ids(), &report).unwrap();
        assert_eq!(
            generated.output_formats(Some(&report)).unwrap(),
            vec![OutputFormat::Pdf, OutputFormat::Csv, OutputFormat::Sql]
        );
        assert_eq!(generation_copies(Some(&report)), 7);
        assert_eq!(generated.requested_by(), Some("admin".to_string()));

        let unset = Report {
            copies: None,
            output_formats: None,
            ..configured()
        };
        let generated = configured_activity_logs(ids(), &unset).unwrap();
        assert_eq!(
            generated.output_formats(Some(&unset)).unwrap(),
            vec![OutputFormat::Pdf]
        );
        assert_eq!(generation_copies(Some(&unset)), 1);
    }

    #[test]
    fn a_format_the_activity_logs_cannot_be_written_in_is_refused() {
        let xml = Report {
            output_formats: Some(vec![OutputFormat::Xml]),
            ..configured()
        };
        assert!(configured_activity_logs(ids(), &xml).is_err());
    }
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task]
pub async fn generate_report(
    report: Report,
    document_id: String,
    report_mode: GenerateReportMode,
    is_scheduled_task: bool,
    task_execution: Option<TasksExecution>,
    executer_username: Option<String>,
    tally_session_id: Option<String>,
    may_read_secret_attributes: bool,
    // Who generates it: the requester of its signing request when its
    // action needs signatures; a scheduled report has none. Absent from
    // tasks queued before signing existed.
    requester: Option<SigningCaller>,
) -> Result<()> {
    let _permit = acquire_semaphore().await?;
    // Spawn the task using an async block
    let task_execution_clone = task_execution.clone();
    let handle = tokio::task::spawn_blocking({
        move || {
            tokio::runtime::Handle::current().block_on(async move {
                generate_report(
                    report,
                    document_id,
                    report_mode,
                    is_scheduled_task,
                    task_execution_clone,
                    executer_username,
                    tally_session_id,
                    may_read_secret_attributes,
                    requester,
                )
                .await
                .map_err(|err| anyhow!("generate_report error: {:?}", err))
            })
        }
    });

    // Await the result and handle JoinError explicitly
    match handle.await {
        Ok(inner_result) => {
            if let Err(ref err) = inner_result {
                if let Some(ref task_exec) = task_execution {
                    let _ = update_fail(task_exec, &format!("Task failed: {:?}", err)).await;
                }
            }
            inner_result.map_err(|err| Error::from(err.context("Task failed")))?;
        }
        Err(join_error) => {
            if let Some(ref task_exec) = task_execution {
                let _ = update_fail(task_exec, &format!("Task panicked: {}", join_error)).await;
            }
            return Err(Error::from(anyhow::anyhow!(
                "Task panicked: {}",
                join_error
            )));
        }
    }

    Ok(())
}
