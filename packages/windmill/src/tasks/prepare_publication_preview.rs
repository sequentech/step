// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
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

use crate::postgres::document::{get_document, get_support_material_documents};
use crate::postgres::election::get_elections;
use crate::postgres::election_event::{get_election_event_by_id, update_election_event_status};
use crate::services::ballot_styles::ballot_publication::get_publication_json;
use crate::services::database::get_hasura_pool;
use crate::services::documents::upload_and_return_document;
use crate::services::election_event_status::get_election_status;
use sequent_core::ballot::VotingStatus;
use sequent_core::types::hasura::core::ElectionEvent;
use sequent_core::util::temp_path::write_into_named_temp_file;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PublicationPreview {
    ballot_styles_json: Value,
    election_event_json: Value,
    elections_json: Value,
}

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
    let ballot_styles_json = get_publication_json(
        &hasura_transaction,
        tenant_id.clone(),
        election_event_id.clone(),
        ballot_publication_id.clone(),
        None,
        None,
    )
    .await?;

    let election_event: ElectionEvent =
        get_election_event_by_id(&hasura_transaction, &tenant_id, &election_event_id)
            .await
            .with_context(|| "Can't find election event")?;

    let election_event_json =
        serde_json::to_value(election_event).with_context(|| "Error serializing election event")?;

    let elections_json =
        get_elections_json_with_open_status(&hasura_transaction, &tenant_id, &election_event_id)
            .await
            .with_context(|| "Can't find open elections")?;

    // TODO:
    // Get the support materials filtering by tenant_id, election_event_id and is_hidden false.
    // Get the documents filtering by tenant_id, election_event_id and JOIN with id = support_materials.document_id

    let _doc = get_document(
        hasura_transaction,
        &tenant_id,
        Some(election_event_id.clone()),
        "",
    )
    .await
    .with_context(|| "Can't document")?;

    let _docs = get_support_material_documents(hasura_transaction, &tenant_id, &election_event_id)
        .await
        .with_context(|| "Can't find support materials")?;

    let pub_preview = PublicationPreview {
        ballot_styles_json,
        election_event_json,
        elections_json,
    };

    let pub_preview_data: Vec<u8> = serde_json::to_value(pub_preview)
        .with_context(|| "Error serializing publication preview")?
        .to_string()
        .as_bytes()
        .to_vec();

    let document_id = Uuid::new_v4().to_string();
    let doc_name_s3 = format!("{ballot_publication_id}.json");
    let temp_name = format!("publication-preview-{document_id}-");
    // let file_path = s3::get_public_document_key(&tenant_id, &document_id, name);
    let (_temp_path, temp_path_string, file_size) =
        write_into_named_temp_file(&pub_preview_data, &temp_name, ".json")
            .with_context(|| "Error writing to file")?;

    let _document = upload_and_return_document(
        hasura_transaction,
        &temp_path_string,
        file_size,
        "application/json",
        &tenant_id,
        Some(election_event_id.to_string()),
        &doc_name_s3,
        Some(document_id.clone()),
        true,
    )
    .await
    .map_err(|err| anyhow!("Error uploading document: {err:?}"))?;

    Ok(document_id)
}

/// Get the elections and mutate the status.voting_status to open
pub async fn get_elections_json_with_open_status(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &String,
    election_event_id: &String,
) -> AnyhowResult<Value> {
    let mut elections = get_elections(&hasura_transaction, &tenant_id, &election_event_id, None)
        .await
        .with_context(|| "Can't find open elections")?;

    let open_elections = elections
        .iter_mut()
        .map(|election| {
            let mut status = get_election_status(election.status.clone()).unwrap_or_default();
            status.voting_status = VotingStatus::OPEN;
            election
        })
        .collect::<Vec<_>>();

    let open_elections_json =
        serde_json::to_value(open_elections).with_context(|| "Error serializing open elections")?;

    Ok(open_elections_json)
}
