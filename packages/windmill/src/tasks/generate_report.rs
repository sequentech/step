// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::reports::Report;
use crate::postgres::reports::ReportType;
use crate::services::database::get_hasura_pool;
use crate::services::database::get_keycloak_pool;
use crate::services::reports::num_of_ov_not_yet_pre_enrolled::NumOVNotPreEnrolledReport;
use crate::services::reports::ov_not_pre_enrolled_list::NotPreEnrolledListTemplate;
use crate::services::reports::ov_turnout::OVTurnoutReport;
use crate::services::reports::ov_turnout_with_percentage::OVTurnoutPercentageReport;
use crate::services::reports::template_renderer::{
    GenerateReportMode, ReportOriginatedFrom, ReportOrigins, TemplateRenderer,
};
use crate::services::reports::{
    activity_log::{ActivityLogsTemplate, ReportFormat},
    audit_logs::AuditLogsTemplate,
    ballot_receipt::BallotTemplate,
    electoral_results::ElectoralResults,
    initialization::InitializationTemplate,
    manual_verification::ManualVerificationTemplate,
    ov_users::OVUserTemplate,
    ov_users_who_voted::OVUsersWhoVotedTemplate,
    ov_who_pre_enrolled::PreEnrolledVoterTemplate,
    ovcs_events::OVCSEventsTemplate,
    ovcs_information::OVCSInformationTemplate,
    ovcs_statistics::OVCSStatisticsTemplate,
    overseas_voters::OverseasVotersReport,
    pre_enrolled_ov_but_disapproved::PreEnrolledDisapprovedTemplate,
    pre_enrolled_ov_subject_to_manual_validation::PreEnrolledManualUsersTemplate,
    statistical_report::StatisticalReportTemplate,
    status::StatusTemplate,
    transmission_report::TransmissionReport,
};
use crate::services::tasks_execution::update_fail;
use crate::types::error::Error;
use crate::types::error::Result;
use anyhow::{anyhow, Context};
use celery::error::TaskError;
use deadpool_postgres::Client as DbClient;
use sequent_core::types::hasura::core::TasksExecution;
use std::str::FromStr;
use tracing::info;
use tracing::instrument;

pub async fn generate_report(
    report: Report,
    document_id: String,
    report_mode: GenerateReportMode,
    is_scheduled_task: bool,
    task_execution: Option<TasksExecution>,
) -> Result<(), anyhow::Error> {
    let tenant_id = report.tenant_id.clone();
    let election_event_id = report.election_event_id.clone();
    let report_type_str = report.report_type.clone();
    let report_clone = report.clone();
    // Clone the election id if it exists
    let election_id = report.election_id.clone();
    let template_id = report.template_id.clone();
    let ids = ReportOrigins {
        tenant_id,
        election_event_id,
        election_id,
        template_id,
        voter_id: None,
        report_origin: ReportOriginatedFrom::ReportsTab, // Assuming this is visited only frrom the Reports tab
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

    // Helper macro to reduce duplication in execute_report call
    macro_rules! execute_report {
        ($report:expr) => {
            $report
                .execute_report(
                    &document_id,
                    &report.tenant_id,
                    &report.election_event_id,
                    is_scheduled_task,
                    vec![],
                    report_mode,
                    Some(report_clone),
                    &hasura_transaction,
                    &keycloak_transaction,
                    task_execution,
                )
                .await?;
        };
    }
    match ReportType::from_str(&report_type_str) {
        Ok(ReportType::OVCS_EVENTS) => {
            let report = OVCSEventsTemplate::new(ids);
            execute_report!(report);
        }
        Ok(ReportType::STATUS) => {
            let report = StatusTemplate::new(ids);
            execute_report!(report);
        }
        Ok(ReportType::AUDIT_LOGS) => {
            let report = AuditLogsTemplate::new(ids);
            execute_report!(report);
        }
        Ok(ReportType::OVCS_INFORMATION) => {
            let report = OVCSInformationTemplate::new(ids);
            execute_report!(report);
        }
        Ok(ReportType::LIST_OF_OVERSEAS_VOTERS) => {
            let report = OverseasVotersReport::new(ids);
            execute_report!(report);
        }
        Ok(ReportType::ELECTORAL_RESULTS) => {
            let report = ElectoralResults::new(ids);
            execute_report!(report);
        }
        Ok(ReportType::LIST_OF_OV_WHO_VOTED) => {
            let report = OVUsersWhoVotedTemplate::new(ids);
            execute_report!(report);
        }
        Ok(ReportType::LIST_OF_OVERSEAS_VOTERS_WITH_VOTING_STATUS) => {
            let report = OVUserTemplate::new(ids);
            execute_report!(report);
        }
        Ok(ReportType::OVCS_STATISTICS) => {
            let report = OVCSStatisticsTemplate::new(ids);
            execute_report!(report);
        }
        Ok(ReportType::PRE_ENROLLED_OV_BUT_DISAPPROVED) => {
            let report = PreEnrolledDisapprovedTemplate::new(ids);
            execute_report!(report);
        }
        Ok(ReportType::PRE_ENROLLED_OV_SUBJECT_TO_MANUAL_VALIDATION) => {
            let report = PreEnrolledManualUsersTemplate::new(ids);
            execute_report!(report);
        }
        Ok(ReportType::STATISTICAL_REPORT) => {
            let report = StatisticalReportTemplate::new(ids);
            execute_report!(report);
        }
        Ok(ReportType::ACTIVITY_LOGS) => {
            let report = ActivityLogsTemplate::new(ids, ReportFormat::PDF);
            execute_report!(report);
        }
        Ok(ReportType::MANUAL_VERIFICATION) => {
            if report_mode == GenerateReportMode::REAL {
                return Err(anyhow!(
                    "Can't generate real report for {}",
                    report_type_str
                ));
            }
            let report = ManualVerificationTemplate::new(ids);
            execute_report!(report);
        }
        Ok(ReportType::TRANSMISSION_REPORT) => {
            let report = TransmissionReport::new(ids);
            execute_report!(report);
        }
        Ok(ReportType::BALLOT_RECEIPT) => {
            let report = BallotTemplate::new(ids, None);
            execute_report!(report);
        }
        Ok(ReportType::INITIALIZATION_REPORT) => {
            let report = InitializationTemplate::new(ids);
            execute_report!(report);
        }
        Ok(ReportType::LIST_OF_OV_WHO_PRE_ENROLLED_APPROVED) => {
            let report = PreEnrolledVoterTemplate::new(ids);
            execute_report!(report);
        }
        Ok(ReportType::LIST_OF_OV_WHO_HAVE_NOT_YET_PRE_ENROLLED) => {
            let report = NotPreEnrolledListTemplate::new(ids);
            execute_report!(report);
        }
        Ok(ReportType::OVERSEAS_VOTERS_TURNOUT) => {
            let report = OVTurnoutPercentageReport::new(ids);
            execute_report!(report);
        }
        Ok(ReportType::NUMBER_OF_OV_WHO_HAVE_NOT_YET_PRE_ENROLLED) => {
            let report = NumOVNotPreEnrolledReport::new(ids);
            execute_report!(report);
        }
        Ok(ReportType::OVERSEAS_VOTERS_TURNOUT_PER_ABOARD_STATUS_AND_SEX) => {
            let report = OVTurnoutReport::new(ids);
            execute_report!(report);
        }
        Err(err) => return Err(anyhow!("{:?}", err)),
    }

    hasura_transaction
        .commit()
        .await
        .with_context(|| "Failed to commit Hasura transaction")?;

    Ok(())
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
) -> Result<()> {
    // Spawn the task using an async block
    let handle = tokio::task::spawn_blocking({
        move || {
            tokio::runtime::Handle::current().block_on(async move {
                generate_report(
                    report,
                    document_id,
                    report_mode,
                    is_scheduled_task,
                    task_execution,
                )
                .await
                .map_err(|err| anyhow!("generate_report error: {:?}", err))
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
