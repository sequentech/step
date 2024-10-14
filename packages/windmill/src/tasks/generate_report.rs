// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::reports::Report;
use crate::postgres::reports::ReportType;
use crate::services::celery_app::get_celery_app;
use crate::services::database::get_hasura_pool;
use crate::services::date::ISO8601;
use crate::services::pg_lock::PgLock;
use crate::services::reports::audit_logs;
use crate::services::reports::manual_verification::ManualVerificationTemplate;
use crate::services::reports::ovcs_events;
use crate::services::reports::ovcs_events::OVCSEventsTemplate;
use crate::services::reports::template_renderer::GenerateReportMode;
use crate::services::reports::template_renderer::TemplateRenderer;
use crate::services::reports::utils::ToMap;
use crate::types::error::Error;
use crate::types::error::Result;
use anyhow::{anyhow, Context};
use celery::error::TaskError;
use chrono::Duration;
use deadpool_postgres::Client as DbClient;
use deadpool_postgres::Transaction;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tracing::instrument;
use tracing::{event, info, Level};
use uuid::Uuid;

pub async fn generate_report(
    report: Report,
    document_id: String,
    report_mode: GenerateReportMode,
) -> Result<(), anyhow::Error> {
    let tenant_id = report.tenant_id.clone();
    let election_event_id = report.election_event_id.clone();
    let report_type_str = report.report_type.clone();
    // Clone the election id if it exists
    let election_id = report.election_id.as_deref().unwrap_or("");
    // Create the template renderer based on the report type
    match ReportType::from_str(&report_type_str) {
        Ok(ReportType::OVCS_EVENTS) => {
            return ovcs_events::generate_ovcs_report(
                &document_id,
                &tenant_id,
                &election_event_id,
                report_mode,
            )
            .await
            .map_err(|err| anyhow!("{}", err))
        }
        Ok(ReportType::AUDIT_LOGS) => {
            return audit_logs::generate_audit_logs_report(
                &document_id,
                &tenant_id,
                &election_event_id,
                &election_id,
                report_mode,
            )
            .await
            .map_err(|err| anyhow!("{}", err))
        }
        _ => {
            panic!("Invalid report type");
        }
    }
    Ok(())
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task]
pub async fn generate_report(
    report: Report,
    document_id: String,
    report_mode: GenerateReportMode,
) -> Result<()> {
    // Spawn the task using an async block
    let handle = tokio::task::spawn_blocking({
        move || {
            tokio::runtime::Handle::current().block_on(async move {
                generate_report(report, document_id, report_mode)
                    .await
                    .map_err(|err| anyhow!("{}", err))
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
