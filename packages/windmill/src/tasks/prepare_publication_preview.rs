// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::document;
use crate::{
    services::tasks_execution::{update_complete, update_fail},
    types::error::Result,
};
use anyhow::{anyhow, Context, Result as AnyhowResult};
use celery::error::TaskError;
use deadpool_postgres::Transaction;
use sequent_core::types::hasura::core::TasksExecution;
use tracing::{info, instrument};
use uuid::Uuid;

use crate::postgres::ballot_publication::get_ballot_publication_by_id;
use crate::postgres::ballot_style::get_publication_ballot_styles;
use crate::postgres::election::{get_elections_ids, update_election_status};
use crate::postgres::election_event::{get_election_event_by_id, update_election_event_status};
use crate::services::ballot_styles::ballot_publication::get_publication_json;
use crate::services::database::get_hasura_pool;
use rocket::http::Status;
#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 2)]
pub async fn prepare_publication_preview(
    tenant_id: String,
    election_event_id: String,
    ballot_publication_id: String,
    task_execution: TasksExecution,
) -> Result<()> {
    let mut hasura_db_client = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|e| format!("Failed to get db connection: {e:?}"))?;

    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|e| format!("Failed to get db transaction: {e:?}"))?;

    let result = prepare_publication_preview_task(
        &hasura_transaction,
        tenant_id,
        election_event_id,
        ballot_publication_id,
    )
    .await;

    match result {
        Ok(document_id) => {
            let _res = update_complete(&task_execution, Some(document_id.clone())).await;
            Ok(())
        }
        Err(err) => {
            let err_str = format!("Error preparing publication preview: {err:?}");
            let _res = update_fail(&task_execution, &err.to_string()).await;
            Err(err_str.into())
        }
    }
}

#[instrument(err)]
pub async fn prepare_publication_preview_task(
    hasura_transaction: &Transaction<'_>,
    tenant_id: String,
    election_event_id: String,
    ballot_publication_id: String,
) -> AnyhowResult<String> {
    let ballot_publication = get_ballot_publication_by_id(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        &ballot_publication_id,
    )
    .await?
    .with_context(|| "Can't find ballot publication")?;

    let ballot_styles = get_publication_json(
        &hasura_transaction,
        tenant_id.clone(),
        election_event_id.clone(),
        ballot_publication.id.clone(),
        ballot_publication.election_id.clone(),
        None,
    )
    .await?;

    // TODO: Upload file to S3...
    let document_id = Uuid::new_v4().to_string();
    // ballot_publication_id: ballot_publication_id.clone(),
    // ballot_styles

    // {
    //     ballot_styles: ballotData?.current?.ballot_styles,
    //     election_event: electionEvent,
    //     elections: openElections,
    //     support_materials: supportMaterials, ??
    //     documents: documents, ??
    // }

    Ok(document_id)
}
