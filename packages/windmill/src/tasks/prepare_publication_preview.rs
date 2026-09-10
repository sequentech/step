// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::ballot_publication::get_ballot_publication_by_id;
use crate::postgres::document::{get_document, get_support_material_documents};
use crate::postgres::election::get_elections;
use crate::postgres::election_event::get_election_event_by_id;
use crate::services::ballot_styles::ballot_publication::get_publication_json;
use crate::services::ballot_styles::publication_files::preview_snapshot;
use crate::services::documents::upload_and_return_document;
use crate::services::providers::transactions_provider::provide_hasura_transaction;
use crate::{
    services::tasks_execution::{update_complete, update_fail},
    types::error::Result,
};
use anyhow::{anyhow, Context, Result as AnyhowResult};
use celery::error::TaskError;
use deadpool_postgres::Transaction;
use sequent_core::types::hasura::core::ElectionEvent;
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::types::hasura::core::{Document, SupportMaterial};
use sequent_core::util::temp_path::write_into_named_temp_file;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{info, instrument};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PublicationPreview {
    pub ballot_styles: Value,
    election_event: Value,
    elections: Value,
    support_materials: Value,
    documents: Value,
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 2)]
pub async fn prepare_publication_preview(
    tenant_id: String,
    election_event_id: String,
    ballot_publication_id: String,
    task_execution: TasksExecution,
    document_id: String,
) -> Result<()> {
    let output_document_id = document_id.clone();
    let result = provide_hasura_transaction(move |tx| {
        Box::pin(async move {
            prepare_publication_preview_task(
                tx,
                tenant_id,
                election_event_id,
                ballot_publication_id,
                document_id,
            )
            .await?;
            Ok(())
        })
    })
    .await;

    match result {
        Ok(()) => {
            update_complete(&task_execution, Some(output_document_id))
                .await
                .map_err(|err| format!("Error completing publication preview task: {err:?}"))?;
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
    document_id: String,
) -> AnyhowResult<String> {
    let publication = get_ballot_publication_by_id(
        hasura_transaction,
        &tenant_id,
        &election_event_id,
        &ballot_publication_id,
    )
    .await?
    .context("Publication not found")?;
    anyhow::ensure!(
        publication.is_generated == Some(true),
        "Publication is not generated"
    );

    // Completion bookkeeping can be retried after the document transaction committed.
    if let Some(document) = get_document(
        hasura_transaction,
        &tenant_id,
        Some(election_event_id.clone()),
        &document_id,
    )
    .await?
    {
        anyhow::ensure!(
            document.name.as_deref() == Some(format!("{ballot_publication_id}.json").as_str())
                && document.is_public == Some(true),
            "Preview document scope mismatch"
        );
        return Ok(document_id);
    }

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

    let mut election_event_json =
        serde_json::to_value(election_event).with_context(|| "Error serializing election event")?;

    let mut elections_json =
        get_elections_json_with_open_status(&hasura_transaction, &tenant_id, &election_event_id)
            .await?;
    if let Some((event, elections)) = preview_snapshot(
        hasura_transaction,
        &tenant_id,
        &election_event_id,
        &ballot_publication_id,
    )
    .await?
    {
        for (key, value) in event.as_object().context("Invalid event snapshot")? {
            election_event_json[key] = value.clone();
        }
        elections_json = Value::Array(elections);
    }
    open_preview_election(&mut election_event_json);
    if let Some(elections) = elections_json.as_array_mut() {
        elections.retain(|election| {
            election["id"].as_str().is_some_and(|id| {
                publication
                    .election_ids
                    .as_ref()
                    .is_some_and(|ids| ids.iter().any(|election_id| election_id == id))
            })
        });
        for election in elections {
            open_preview_election(election);
        }
    }
    let (support_materials_json, documents_json) =
        get_support_material_documents_json(&hasura_transaction, &tenant_id, &election_event_id)
            .await?;
    let pub_preview = PublicationPreview {
        ballot_styles: ballot_styles_json,
        election_event: election_event_json,
        elections: elections_json,
        support_materials: support_materials_json,
        documents: documents_json,
    };

    let pub_preview_data: Vec<u8> = serde_json::to_value(pub_preview)
        .with_context(|| "Error serializing publication preview")?
        .to_string()
        .as_bytes()
        .to_vec();

    let doc_name_s3 = format!("{ballot_publication_id}.json");
    let temp_name = format!("publication-preview-{document_id}-");
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

/// Get the support materials and document vectors in json.
pub async fn get_support_material_documents_json(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> AnyhowResult<(Value, Value)> {
    let support_material_docs: Vec<(SupportMaterial, Document)> =
        get_support_material_documents(hasura_transaction, tenant_id, election_event_id)
            .await
            .with_context(|| "Can't find support materials")?
            .unwrap_or_default();

    let (sm, d): (Vec<SupportMaterial>, Vec<Document>) = support_material_docs.into_iter().unzip();
    let support_materials =
        serde_json::to_value(sm).with_context(|| "Error serializing support materials")?;
    let documents = serde_json::to_value(d).with_context(|| "Error serializing documents")?;
    Ok((support_materials, documents))
}

/// Get the elections and mutate the status.voting_status to open
pub async fn get_elections_json_with_open_status(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> AnyhowResult<Value> {
    let elections = get_elections(&hasura_transaction, tenant_id, election_event_id)
        .await
        .with_context(|| "Can't find open elections")?;
    let mut elections_json =
        serde_json::to_value(elections).with_context(|| "Error serializing open elections")?;
    if let Some(elections) = elections_json.as_array_mut() {
        for election in elections {
            open_preview_election(election);
        }
    }
    Ok(elections_json)
}

fn open_preview_election(election: &mut Value) {
    if !election["status"].is_object() {
        election["status"] = serde_json::json!({});
    }
    election["status"]["voting_status"] = Value::String("OPEN".to_owned());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publication_preview_opens_closed_and_missing_status_without_changing_metadata() {
        for status in [
            Value::Null,
            serde_json::json!({"voting_status":"CLOSED", "kiosk_voting_status":"PAUSED"}),
        ] {
            let mut election =
                serde_json::json!({"id":"election", "description":"frozen", "status":status});
            open_preview_election(&mut election);
            assert_eq!(election["status"]["voting_status"], "OPEN");
            assert_eq!(election["description"], "frozen");
            if status.is_object() {
                assert_eq!(election["status"]["kiosk_voting_status"], "PAUSED");
            }
        }
    }
}
