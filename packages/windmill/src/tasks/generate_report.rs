// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::reports::Report;
use crate::services::celery_app::get_celery_app;
use crate::services::database::get_hasura_pool;
use crate::services::pg_lock::PgLock;
use crate::services::reports::template_renderer::TemplateRenderer;
use crate::types::error::Error;
use crate::types::error::Result;
use anyhow::{anyhow, Context};
use celery::error::TaskError;
use chrono::Duration;
use deadpool_postgres::Client as DbClient;
use deadpool_postgres::Transaction;
use sequent_core::services::date::ISO8601;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use tracing::{event, info, Level};
use uuid::Uuid;

// fn create_template_renderer(
//     report: &Report,
//     document_id: &str,
// ) -> Result<Box<dyn TemplateRenderer>> {
//     Handle each report type and generate a template renderer implimentation
//     match report.report_type {
//         ReportType::MANUAL_VERIFICATION => {
//             let voter_id = additional_params
//                 .get("voter_id")
//                 .ok_or_else(|| anyhow!("voter_id required for MANUAL_VERIFICATION"))?;
//             Ok(Box::new(ManualVerificationTemplate {
//                 tenant_id: tenant_id.to_string(),
//                 election_event_id: election_event_id.to_string(),
//                 voter_id: voter_id.clone(),
//             }))
//         }
//         ReportType::BALLOT_RECEIPT => {
//             let voter_id = additional_params
//                 .get("voter_id")
//                 .ok_or_else(|| anyhow!("voter_id required for BALLOT_RECEIPT"))?;
//             let election_id = additional_params
//                 .get("election_id")
//                 .ok_or_else(|| anyhow!("election_id required for BALLOT_RECEIPT"))?;
//             Ok(Box::new(BallotReceiptTemplate {
//                 tenant_id: tenant_id.to_string(),
//                 election_event_id: election_event_id.to_string(),
//                 voter_id: voter_id.clone(),
//                 election_id: election_id.clone(),
//             }))
//         }
//         // Handle other report types...
//     }
// }

pub async fn generate_report(report: Report, document_id: String) -> Result<()> {
    return Ok(());
    // // Create the template renderer based on the report type
    // let template_renderer = create_template_renderer(
    //     &report,
    //     &document_id,
    // )?;

    // // Execute the report using the template renderer
    // template_renderer
    //     .execute_report(&document_id, &report.tenant_id, &report.election_event_id, false, None)
    //     .await
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task]
pub async fn generate_report(report: Report, document_id: String) -> Result<()> {
    // Spawn the task using an async block
    let handle = tokio::task::spawn_blocking({
        move || {
            tokio::runtime::Handle::current().block_on(async move {
                generate_report(report, document_id)
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
