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
use crate::services::external::utils::remove_datafix_annotations;
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
use tempfile::TempPath;
use tracing::{info, instrument};

/// Field holding the annotations of a serialized `election_event` row.
const ANNOTATIONS_FIELD: &str = "annotations";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PublicationPreview {
    pub ballot_styles: Value,
    election_event: Value,
    elections: Value,
    support_materials: Value,
    documents: Value,
}

impl PublicationPreview {
    /// Serializes the preview into a temporary file ready to upload. Every
    /// preview upload goes through here, so the Datafix annotations are
    /// dropped here too: they hold the VoterView credentials and the preview
    /// is uploaded to the public bucket. Ballot styles are already sanitized
    /// when they are stored, the event is not. Elections never carry Datafix
    /// annotations: they only live on the election event.
    pub fn into_temp_file(mut self, prefix: &str) -> AnyhowResult<(TempPath, String, u64)> {
        remove_datafix_annotations(self.election_event.get_mut(ANNOTATIONS_FIELD));

        let preview_data =
            serde_json::to_vec(&self).with_context(|| "Error serializing publication preview")?;
        write_into_named_temp_file(&preview_data, prefix, ".json")
            .with_context(|| "Error writing publication preview to file")
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
    let pub_preview = PublicationPreview {
        ballot_styles: ballot_styles_json,
        election_event: election_event_json,
        elections: elections_json,
        support_materials: support_materials_json,
        documents: documents_json,
    };

    let doc_name_s3 = format!("{ballot_publication_id}.json");
    let temp_name = format!("publication-preview-{document_id}-");
    let (_temp_path, temp_path_string, file_size) = pub_preview.into_temp_file(&temp_name)?;

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

    fn preview_with_annotations(election_event: Value) -> PublicationPreview {
        PublicationPreview {
            ballot_styles: json!([{"id": "style", "area_id": "area"}]),
            election_event,
            elections: json!([{"id": "election"}]),
            support_materials: json!([]),
            documents: json!([]),
        }
    }

    fn written_preview(preview: PublicationPreview) -> Value {
        let (_temp_path, path, _size) = preview
            .into_temp_file("publication-preview-test-")
            .expect("preview file");
        serde_json::from_str(&std::fs::read_to_string(&path).expect("read written preview"))
            .expect("parse written preview")
    }

    #[test]
    fn the_written_preview_drops_datafix_annotations_from_the_event() {
        let preview = preview_with_annotations(json!({
            "id": "event",
            "annotations": {
                DATAFIX_ID_KEY: "external-event",
                DATAFIX_VOTERVIEW_REQ_KEY: r#"{"url":"https://example.invalid","usr":"user","psw":"secret"}"#,
                "miru:election-event-id": "miru-event",
            },
        }));

        let written = written_preview(preview);

        assert_eq!(
            written["election_event"]["annotations"],
            json!({"miru:election-event-id": "miru-event"})
        );
        assert_eq!(written["election_event"]["id"], "event");
        assert_eq!(written["elections"][0]["id"], "election");
        assert_eq!(written["ballot_styles"][0]["id"], "style");
    }

    #[test]
    fn the_written_preview_tolerates_annotations_that_are_absent_or_not_an_object() {
        for election_event in [
            json!({"id": "event"}),
            json!({"id": "event", "annotations": null}),
            json!({"id": "event", "annotations": [DATAFIX_ID_KEY]}),
        ] {
            let written = written_preview(preview_with_annotations(election_event.clone()));

            assert_eq!(written["election_event"], election_event);
        }
    }
}
