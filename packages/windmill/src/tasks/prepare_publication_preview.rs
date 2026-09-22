// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::document::get_support_material_documents;
use crate::postgres::election::get_elections;
use crate::postgres::election_event::get_election_event_by_id;
use crate::services::ballot_styles::ballot_publication::get_publication_json;
use crate::services::database::get_hasura_pool;
use crate::services::documents::upload_and_return_document;
use crate::services::election_event_status::get_election_status;
use crate::services::external::utils::remove_datafix_annotations_json;
use crate::{
    services::tasks_execution::{update_complete, update_fail},
    types::error::Result,
};
use anyhow::{anyhow, Context, Result as AnyhowResult};
use celery::error::TaskError;
use deadpool_postgres::Transaction;
use sequent_core::ballot::VotingStatus;
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

impl PublicationPreview {
    /// The preview payload is uploaded to the public bucket, so it must carry
    /// no Datafix annotations: they hold the VoterView credentials. Ballot
    /// styles are already sanitized when they are stored, the event and its
    /// elections are not.
    pub fn remove_datafix_annotations(&mut self) {
        remove_datafix_annotations_json(&mut self.election_event);
        if let Some(elections) = self.elections.as_array_mut() {
            for election in elections {
                remove_datafix_annotations_json(election);
            }
        }
    }
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
        document_id,
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
    document_id: String,
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
            .await?;
    let (support_materials_json, documents_json) =
        get_support_material_documents_json(&hasura_transaction, &tenant_id, &election_event_id)
            .await?;
    let mut pub_preview = PublicationPreview {
        ballot_styles: ballot_styles_json,
        election_event: election_event_json,
        elections: elections_json,
        support_materials: support_materials_json,
        documents: documents_json,
    };
    pub_preview.remove_datafix_annotations();

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
    let mut elections = get_elections(&hasura_transaction, tenant_id, election_event_id)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::external::utils::{DATAFIX_ID_KEY, DATAFIX_VOTERVIEW_REQ_KEY};
    use serde_json::json;

    fn preview_with_annotations(
        event_annotations: Value,
        election_annotations: Value,
    ) -> PublicationPreview {
        PublicationPreview {
            ballot_styles: json!([{"id": "style", "area_id": "area"}]),
            election_event: json!({"id": "event", "annotations": event_annotations}),
            elections: json!([{"id": "election", "annotations": election_annotations}]),
            support_materials: json!([]),
            documents: json!([]),
        }
    }

    #[test]
    fn preview_payload_drops_datafix_annotations_from_event_and_elections() {
        let mut preview = preview_with_annotations(
            json!({
                DATAFIX_ID_KEY: "external-event",
                DATAFIX_VOTERVIEW_REQ_KEY: r#"{"url":"https://example.invalid","usr":"user","psw":"secret"}"#,
                "miru:election-event-id": "miru-event",
            }),
            json!({DATAFIX_ID_KEY: "external-event", "miru:election-id": "miru-election"}),
        );

        preview.remove_datafix_annotations();

        assert_eq!(
            preview.election_event["annotations"],
            json!({"miru:election-event-id": "miru-event"})
        );
        assert_eq!(
            preview.elections[0]["annotations"],
            json!({"miru:election-id": "miru-election"})
        );
        assert_eq!(preview.election_event["id"], "event");
        assert_eq!(preview.ballot_styles[0]["id"], "style");
    }

    #[test]
    fn preview_payload_sanitization_tolerates_missing_annotations() {
        let mut preview = PublicationPreview {
            ballot_styles: json!([]),
            election_event: json!({"id": "event"}),
            elections: Value::Null,
            support_materials: json!([]),
            documents: json!([]),
        };

        preview.remove_datafix_annotations();

        assert_eq!(preview.election_event, json!({"id": "event"}));
        assert_eq!(preview.elections, Value::Null);
    }
}
