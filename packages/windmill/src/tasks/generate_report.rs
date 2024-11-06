// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::reports::Report;
use crate::postgres::reports::ReportType;
use crate::services::database::get_hasura_pool;
use crate::services::database::get_keycloak_pool;
use crate::services::reports::audit_logs;
use crate::services::reports::ovcs_events;
use crate::services::reports::template_renderer::GenerateReportMode;
use crate::services::reports::transmission;
use crate::services::reports::{
    activity_log, ballot_receipt, electoral_results, initialization, manual_verification, ov_users,
    ov_users_who_voted, ovcs_information, ovcs_statistics, overseas_voters,
    pre_enrolled_ov_but_disapproved, pre_enrolled_ov_subject_to_manual_validation,
    statistical_report, status,
};
use crate::types::error::Error;
use crate::types::error::Result;
use anyhow::{anyhow, Context};
use celery::error::TaskError;
use deadpool_postgres::Client as DbClient;
use std::str::FromStr;
use tracing::instrument;
use tracing::{event, info, Level};

pub async fn generate_report(
    report: Report,
    document_id: String,
    report_mode: GenerateReportMode,
    is_scheduled_task: bool,
) -> Result<(), anyhow::Error> {
    let tenant_id = report.tenant_id.clone();
    let election_event_id = report.election_event_id.clone();
    let report_type_str = report.report_type.clone();
    // Clone the election id if it exists
    let election_id = report.election_id.as_deref().unwrap_or("");

    let cron_config = report
        .cron_config
        .clone()
        .ok_or_else(|| anyhow!("Cron config not found"))?;

    let mut db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .with_context(|| "Error getting DB pool")?;

    let hasura_transaction = db_client
        .transaction()
        .await
        .with_context(|| "Error starting transaction")?;

    info!("is scheduled eventttttt {:?}", is_scheduled_task);

    let mut keycloak_db_client = get_keycloak_pool()
        .await
        .get()
        .await
        .with_context(|| "Error acquiring Keycloak DB pool")?;

    let keycloak_transaction = keycloak_db_client
        .transaction()
        .await
        .with_context(|| "Error starting Keycloak transaction")?;

    // Create the template renderer based on the report type
    match ReportType::from_str(&report_type_str) {
        Ok(ReportType::OVCS_EVENTS) => {
            return ovcs_events::generate_report(
                &document_id,
                &tenant_id,
                &election_event_id,
                report.election_id.as_deref(),
                report_mode,
                &hasura_transaction,
                &keycloak_transaction,
                is_scheduled_task,
                cron_config.email_recipients    
            )
            .await
            .map_err(|err| anyhow!("error generating report: {err:?}, report_type_str={report_type_str:?}"))
        }
        Ok(ReportType::STATUS) => {
            return status::generate_status_report(
                &document_id,
                &tenant_id,
                &election_event_id,
                report.election_id.as_deref(),
                report_mode,
                &hasura_transaction,
                &keycloak_transaction,
                is_scheduled_task,
                cron_config.email_recipients    
            )
            .await
            .map_err(|err| anyhow!("error generating report: {err:?}, report_type_str={report_type_str:?}"))
        }
        Ok(ReportType::AUDIT_LOGS) => {
            return audit_logs::generate_audit_logs_report(
                &document_id,
                &tenant_id,
                &election_event_id,
                report.election_id.as_deref(),
                report_mode,
                &hasura_transaction,
                &keycloak_transaction,
                is_scheduled_task,
                cron_config.email_recipients    
            )
            .await
            .map_err(|err| anyhow!("error generating report: {err:?}, report_type_str={report_type_str:?}"))
        }
        Ok(ReportType::OVCS_INFORMATION) => {
            return ovcs_information::generate_ovcs_informations_report(
                &document_id,
                &tenant_id,
                &election_event_id,
                report.election_id.as_deref(),
                report_mode,
                &hasura_transaction,
                &keycloak_transaction,
                is_scheduled_task,
                cron_config.email_recipients    
            )
            .await
            .map_err(|err| anyhow!("error generating report: {err:?}, report_type_str={report_type_str:?}"))
        }
        Ok(ReportType::OVERSEAS_VOTERS) => {
            return overseas_voters::generate_overseas_voters_report(
                &document_id,
                &tenant_id,
                &election_event_id,
                report.election_id.as_deref(),
                report_mode,
                &hasura_transaction,
                &keycloak_transaction,
                is_scheduled_task,
                cron_config.email_recipients    
            )
            .await
            .map_err(|err| anyhow!("error generating report: {err:?}, report_type_str={report_type_str:?}"))
        }
        Ok(ReportType::ELECTORAL_RESULTS) => {
            return electoral_results::generate_report(
                &document_id,
                &tenant_id,
                &election_event_id,
                report.election_id.as_deref(),
                report_mode,
                &hasura_transaction,
                &keycloak_transaction
            )
            .await
            .map_err(|err| anyhow!("error generating report: {err:?}, report_type_str={report_type_str:?}"))
        }
        Ok(ReportType::OV_USERS_WHO_VOTED) => {
            return ov_users_who_voted::generate_ov_users_who_voted_report(
                &document_id,
                &tenant_id,
                &election_event_id,
                report.election_id.as_deref(),
                report_mode,
                &hasura_transaction,
                &keycloak_transaction,
                is_scheduled_task,
                cron_config.email_recipients    
            )
            .await
            .map_err(|err| anyhow!("error generating report: {err:?}, report_type_str={report_type_str:?}"))
        }
        Ok(ReportType::OV_USERS) => {
            return ov_users::generate_ov_users_report(
                &document_id,
                &tenant_id,
                &election_event_id,
                report.election_id.as_deref(),
                report_mode,
                &hasura_transaction,
                &keycloak_transaction,
                is_scheduled_task,
                cron_config.email_recipients   
            )
            .await
            .map_err(|err| anyhow!("error generating report: {err:?}, report_type_str={report_type_str:?}"))
        }
        Ok(ReportType::OVCS_STATISTICS) => {
            return ovcs_statistics::generate_ovcs_statistics_report(
                &document_id,
                &tenant_id,
                &election_event_id,
                report.election_id.as_deref(),
                report_mode,
                &hasura_transaction,
                &keycloak_transaction,
                is_scheduled_task,
                cron_config.email_recipients   
            )
            .await
            .map_err(|err| anyhow!("error generating report: {err:?}, report_type_str={report_type_str:?}"))
        }
        Ok(ReportType::PRE_ENROLLED_OV_BUT_DISAPPROVED) => {
            return pre_enrolled_ov_but_disapproved::generate_pre_enrolled_ov_but_disapproved_report(
                &document_id,
                &tenant_id,
                &election_event_id,
                report.election_id.as_deref(),
                report_mode,
                &hasura_transaction,
                &keycloak_transaction,
                is_scheduled_task,
                cron_config.email_recipients   
            )
            .await
            .map_err(|err| anyhow!("error generating report: {err:?}, report_type_str={report_type_str:?}"))
        }
        Ok(ReportType::PRE_ENROLLED_OV_SUBJECT_TO_MANUAL_VALIDATION) => {
            return pre_enrolled_ov_subject_to_manual_validation::generate_pre_enrolled_ov_subject_to_manual_validation_report(
                &document_id,
                &tenant_id,
                &election_event_id,
                report.election_id.as_deref(),
                report_mode,
                &hasura_transaction,
                &keycloak_transaction,
                is_scheduled_task,
                cron_config.email_recipients   
            )
            .await
            .map_err(|err| anyhow!("error generating report: {err:?}, report_type_str={report_type_str:?}"))
        }
        Ok(ReportType::STATISTICAL_REPORT) => {
            return statistical_report::generate_statistical_report(
                &document_id,
                &tenant_id,
                &election_event_id,
                report.election_id.as_deref(),
                report_mode,
                &hasura_transaction,
                &keycloak_transaction,
                is_scheduled_task,
                cron_config.email_recipients         )
            .await
            .map_err(|err| anyhow!("error generating report: {err:?}, report_type_str={report_type_str:?}"))
        }
        Ok(ReportType::ACTIVITY_LOGS) => {
            return activity_log::generate_report(
                &document_id,
                &tenant_id,
                &election_event_id,
                activity_log::ReportFormat::PDF,
                report_mode,
                &hasura_transaction,
                &keycloak_transaction,
                is_scheduled_task,
                cron_config.email_recipients 
            )
            .await
            .map_err(|err| anyhow!("error generating report: {err:?}, report_type_str={report_type_str:?}"))
        }
        Ok(ReportType::MANUAL_VERIFICATION) => {
            return manual_verification::generate_report(
                &document_id,
                &tenant_id,
                &election_event_id,
                match report_mode {
                    GenerateReportMode::PREVIEW => "",
                    GenerateReportMode::REAL => return Err(anyhow!("Can't generate real manual_verification report from here")),
                },
                report_mode,
                &hasura_transaction,
                &keycloak_transaction
            )
            .await
            .map_err(|err| anyhow!("error generating report: {err:?}, report_type_str={report_type_str:?}"))
        }
        Ok(ReportType::TRANSMISSION_REPORTS) => {
            return transmission::generate_transmission_report(
                &document_id,
                &tenant_id,
                &election_event_id,
                report.election_id.as_deref(),
                report_mode,
                &hasura_transaction,
                &keycloak_transaction
            )
            .await
            .map_err(|err| anyhow!("error generating report: {err:?}, report_type_str={report_type_str:?}"))
        }
        Ok(ReportType::BALLOT_RECEIPT) => {
            if report_mode == GenerateReportMode::REAL {
                return Err(anyhow!("Can't generate real ballot_receipt report from here"));
            }
            return ballot_receipt::generate_ballot_receipt_report(
                &document_id,
                &tenant_id,
                &election_event_id,
                report.election_id.as_deref(),
                report_mode,
                &hasura_transaction,
                &keycloak_transaction,
                None,
            )
            .await
            .map_err(|err| anyhow!("error generating report: {err:?}, report_type_str={report_type_str:?}"))
        }
        Ok(ReportType::PRE_ENROLLED_USERS) => {}
        Ok(ReportType::INITIALIZATION) => {
            let _ = initialization::generate_report(
                &document_id,
                &tenant_id,
                &election_event_id,
                Some(&election_id),
                report_mode,
                &hasura_transaction,
                &keycloak_transaction
            )
            .await
            .map_err(|err| anyhow!("error generating report: {err:?}"));
        hasura_transaction.commit().await.with_context(|| "Failed to commit Hasura transaction")?;
        }
        Err(err) => return Err(anyhow!("{err:?}"))
    };

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
) -> Result<()> {
    // Spawn the task using an async block
    let handle = tokio::task::spawn_blocking({
        move || {
            tokio::runtime::Handle::current().block_on(async move {
                generate_report(report, document_id, report_mode, is_scheduled_task)
                    .await
                    .map_err(|err| anyhow!("generate_report error: {err:?}"))
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
