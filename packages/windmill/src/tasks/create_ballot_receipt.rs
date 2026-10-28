// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::database::{get_hasura_pool, get_keycloak_pool};
use crate::services::reports::ballot_receipt::{BallotData, BallotTemplate};
use crate::services::reports::template_renderer::{
    GenerateReportMode, ReportOriginatedFrom, ReportOrigins, TemplateRenderer,
};
use crate::services::tasks_semaphore::acquire_semaphore;
use crate::types::error::Error;
use crate::types::error::Result;
use anyhow::{anyhow, Context};
use celery::error::TaskError;
use deadpool_postgres::Client as DbClient;
use sequent_core::types::date_time::{DateFormat, TimeZone};
use sequent_core::types::hasura::core::TasksExecution;
use tracing::instrument;

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task]
pub async fn create_ballot_receipt(
    document_id: String,
    ballot_id: String,
    ballot_tracker_url: String,
    tenant_id: String,
    election_event_id: String,
    election_id: String,
    area_id: String,
    voter_id: String,
    time_zone: Option<TimeZone>,
    date_format: Option<DateFormat>,
    task_execution: TasksExecution,
    is_kiosk: Option<bool>,
) -> Result<()> {
    // Kept so requests and queued tasks stay compatible; the tracker link is
    // built from portal configuration. Tasks queued without `is_kiosk` get a
    // normal portal link.
    let _ = ballot_tracker_url;
    let _permit = acquire_semaphore().await?;
    // Spawn the task using an async block
    let handle = tokio::task::spawn_blocking({
        move || {
            tokio::runtime::Handle::current().block_on(async move {
                let mut db_client: DbClient = get_hasura_pool()
                    .await
                    .get()
                    .await
                    .map_err(|err| format!("Error getting DB pool: {err:?}"))?;

                let hasura_transaction = match db_client.transaction().await {
                    Ok(transaction) => transaction,
                    Err(err) => {
                        return Err(Error::String(format!(
                            "Error starting Hasura transaction: {err}"
                        )));
                    }
                };

                let mut keycloak_db_client = get_keycloak_pool()
                    .await
                    .get()
                    .await
                    .map_err(|err| format!("Error acquiring Keycloak DB pool: {err:?}"))?;

                let keycloak_transaction = keycloak_db_client
                    .transaction()
                    .await
                    .map_err(|err| format!("Error starting Keycloak transaction: {err:?}"))?;

                let report = BallotTemplate::new(
                    ReportOrigins {
                        tenant_id: tenant_id.clone(),
                        election_event_id: election_event_id.clone(),
                        election_id: Some(election_id.clone()),
                        template_alias: None,
                        voter_id: None,
                        report_origin: ReportOriginatedFrom::VotingPortal,
                        executer_username: None,
                        tally_session_id: None,
                    },
                    Some(BallotData {
                        area_id,
                        voter_id,
                        ballot_id,
                        is_kiosk: is_kiosk.unwrap_or_default(),
                        time_zone,
                        date_format,
                    }),
                );

                report
                    .execute_report(
                        &document_id,
                        &tenant_id,
                        &election_event_id,
                        /* is_scheduled_task */ false,
                        /* recipients */ vec![],
                        GenerateReportMode::REAL,
                        None,
                        &hasura_transaction,
                        &keycloak_transaction,
                        Some(task_execution),
                    )
                    .await
                    .map_err(|err| anyhow!("Error generating ballot receipt report: {err:?}"))?;

                hasura_transaction
                    .commit()
                    .await
                    .with_context(|| "Failed to commit Hasura transaction")?;

                Ok(())
            })
        }
    });

    // Await the result and handle JoinError explicitly
    match handle.await {
        Ok(inner_result) => Ok(inner_result.map_err(|err| format!("Task failed: {err:?}"))?),
        Err(join_error) => Err(format!("Join error. Task panicked: {:?}", join_error)),
    }?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use celery::task::Task;

    type Params = <create_ballot_receipt as Task>::Params;

    #[test]
    fn tasks_queued_without_the_kiosk_flag_still_deserialize() {
        let mut kwargs = serde_json::json!({
            "document_id": "document-id",
            "ballot_id": "ballot-id",
            "ballot_tracker_url": "https://voting.example/legacy-path",
            "tenant_id": "tenant-id",
            "election_event_id": "event-id",
            "election_id": "election-id",
            "area_id": "area-id",
            "voter_id": "voter-id",
            "time_zone": null,
            "date_format": null,
            "task_execution": {
                "id": "task-id",
                "tenant_id": "tenant-id",
                "election_event_id": "event-id",
                "name": "ballot receipt",
                "task_type": "CREATE_BALLOT_RECEIPT",
                "execution_status": "IN_PROGRESS",
                "created_at": "2026-10-16T00:00:00Z",
                "start_at": null,
                "end_at": null,
                "annotations": null,
                "labels": null,
                "logs": null,
                "executed_by_user": "voter-id"
            }
        });
        let queued: Params = serde_json::from_value(kwargs.clone()).unwrap();
        assert_eq!(queued.is_kiosk, None);

        kwargs["is_kiosk"] = serde_json::Value::Bool(true);
        let current: Params = serde_json::from_value(kwargs).unwrap();
        assert_eq!(current.is_kiosk, Some(true));
    }
}
