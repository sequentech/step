// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::application::insert_applications;
use crate::postgres::area::get_event_areas;
use crate::postgres::election_event::get_election_event_by_id;
use crate::services::providers::transactions_provider::provide_hasura_transaction;
use crate::{
    postgres::document::get_document,
    services::documents::get_document_as_temp_file,
    services::tasks_execution::{update_complete, update_fail},
    types::error::Result,
};
use anyhow::{anyhow, Context, Result as AnyhowResult};
use celery::error::TaskError;
use deadpool_postgres::Transaction;
use sequent_core::serialization::deserialize_with_path::deserialize_str;
use sequent_core::types::hasura::core::Application;
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::util::integrity_check::{integrity_check, HashFileVerifyError};
use std::collections::HashSet;
use std::io::{Read, Seek};
use tracing::{info, instrument};
use uuid::Uuid;

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 2)]
pub async fn import_applications(
    tenant_id: String,
    election_event_id: String,
    document_id: String,
    sha256: Option<String>,
    task_execution: TasksExecution,
) -> Result<()> {
    let result = provide_hasura_transaction(|hasura_transaction| {
        let document_copy = document_id.clone();
        Box::pin(async move {
            import_applications_task(
                hasura_transaction,
                tenant_id,
                election_event_id,
                document_copy.clone(),
                sha256,
            )
            .await
        })
    })
    .await;

    match result {
        Ok(_) => {
            let _res = update_complete(&task_execution, Some(document_id.clone())).await;
            Ok(())
        }
        Err(err) => {
            let err_str = format!("Error importing applications: {err:?}");
            let _res = update_fail(&task_execution, &err.to_string()).await;
            Err(err_str.into())
        }
    }
}

fn parse_applications_csv<R: Read>(
    reader: R,
    tenant_id: &str,
    election_event_id: &str,
    event_area_ids: &HashSet<Uuid>,
) -> AnyhowResult<Vec<Application>> {
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b',')
        .has_headers(true)
        .from_reader(reader);

    let mut applications: Vec<Application> = vec![];

    for result in rdr.records() {
        let record = result.with_context(|| "Error reading CSV record")?;

        let created_at = record.get(1).unwrap_or("");
        let updated_at = record.get(2).unwrap_or("");
        let area_id = record.get(5).unwrap_or("");
        let applicant_id = record.get(6).unwrap_or("");
        let applicant_data = record.get(7).unwrap_or("");
        let labels = record.get(8).unwrap_or("");
        let annotations = record.get(9).unwrap_or("");
        let verification_type = record.get(10).unwrap_or("");
        let status = record.get(11).unwrap_or("");

        let area_uuid = Uuid::parse_str(area_id)
            .ok()
            .filter(|uuid| event_area_ids.contains(uuid))
            .ok_or_else(|| {
                anyhow!("Area {area_id} does not belong to election event {election_event_id}")
            })?;

        applications.push(Application {
            id: Uuid::new_v4().to_string(),
            created_at: Some(created_at.parse().unwrap_or_default()),
            updated_at: Some(updated_at.parse().unwrap_or_default()),
            tenant_id: tenant_id.to_string(),
            election_event_id: election_event_id.to_string(),
            area_id: Some(area_uuid.to_string()),
            applicant_id: applicant_id.to_string(),
            applicant_data: deserialize_str(applicant_data).unwrap_or_default(),
            labels: Some(serde_json::Value::String(labels.to_string())),
            annotations: Some(serde_json::Value::String(annotations.to_string())),
            verification_type: verification_type.to_string(),
            status: status.to_string(),
        });
    }

    Ok(applications)
}

#[instrument(err)]
pub async fn import_applications_task(
    hasura_transaction: &Transaction<'_>,
    tenant_id: String,
    election_event_id: String,
    document_id: String,
    sha256: Option<String>,
) -> AnyhowResult<()> {
    get_election_event_by_id(hasura_transaction, &tenant_id, &election_event_id).await?;
    let event_area_ids = get_event_areas(hasura_transaction, &tenant_id, &election_event_id)
        .await?
        .iter()
        .map(|area| Uuid::parse_str(&area.id))
        .collect::<std::result::Result<HashSet<Uuid>, uuid::Error>>()?;

    let document = get_document(hasura_transaction, &tenant_id, None, &document_id)
        .await
        .with_context(|| "Error obtaining the document")?
        .ok_or(anyhow!("document not found"))?;

    let mut temp_file = get_document_as_temp_file(&tenant_id, &document).await?;
    temp_file.rewind()?;

    match sha256 {
        Some(hash) if !hash.is_empty() => match integrity_check(&temp_file, hash) {
            Ok(_) => {
                info!("Hash verified !");
            }
            Err(HashFileVerifyError::HashMismatch(input_hash, gen_hash)) => {
                let err_str = format!("Failed to verify the integrity: Hash of voters file: {gen_hash} does not match with the input hash: {input_hash}");
                return Err(anyhow!(err_str));
            }
            Err(err) => {
                let err_str = format!("Failed to verify the integrity: {err:?}");
                return Err(anyhow!(err_str));
            }
        },
        _ => {
            info!("No hash provided, skipping integrity check");
        }
    }

    let applications =
        parse_applications_csv(temp_file, &tenant_id, &election_event_id, &event_area_ids)?;

    insert_applications(hasura_transaction, &applications).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const TASK_TENANT_ID: &str = "4f1c6b38-9f5e-4a59-8d3b-2a7f0c1e5d61";
    const TASK_EVENT_ID: &str = "0d6f8c2e-5b1a-4c3d-9e7f-1a2b3c4d5e6f";
    const EVENT_AREA_ID: &str = "6a7b8c9d-0e1f-4a2b-8c3d-4e5f6a7b8c9d";
    const ROW_TENANT_ID: &str = "b7e2d915-3c4a-4e8f-9a61-5d0f2c8b7e43";
    const ROW_EVENT_ID: &str = "c3d4e5f6-a7b8-4c9d-8e0f-1a2b3c4d5e6f";
    const OTHER_AREA_ID: &str = "e1f2a3b4-c5d6-4e7f-8a9b-0c1d2e3f4a5b";

    fn csv_row(tenant_id: &str, election_event_id: &str, area_id: &str) -> String {
        format!(
            "id,created_at,updated_at,tenant_id,election_event_id,area_id,applicant_id,applicant_data,labels,annotations,verification_type,status\n\
             application,,,{tenant_id},{election_event_id},{area_id},applicant,{{}},,,MANUAL,PENDING\n"
        )
    }

    fn event_area_ids() -> HashSet<Uuid> {
        HashSet::from([Uuid::parse_str(EVENT_AREA_ID).unwrap()])
    }

    #[test]
    fn imported_applications_belong_to_task_tenant_and_event() {
        let csv = csv_row(ROW_TENANT_ID, ROW_EVENT_ID, EVENT_AREA_ID);
        let applications = parse_applications_csv(
            csv.as_bytes(),
            TASK_TENANT_ID,
            TASK_EVENT_ID,
            &event_area_ids(),
        )
        .unwrap();
        assert_eq!(applications.len(), 1);
        assert_eq!(applications[0].tenant_id, TASK_TENANT_ID);
        assert_eq!(applications[0].election_event_id, TASK_EVENT_ID);
        assert_eq!(applications[0].area_id.as_deref(), Some(EVENT_AREA_ID));
    }

    #[test]
    fn rows_with_a_blank_or_malformed_tenant_are_stored_under_the_task_tenant() {
        for row_tenant_id in ["", "not-a-uuid"] {
            let csv = csv_row(row_tenant_id, ROW_EVENT_ID, EVENT_AREA_ID);
            let applications = parse_applications_csv(
                csv.as_bytes(),
                TASK_TENANT_ID,
                TASK_EVENT_ID,
                &event_area_ids(),
            )
            .unwrap();
            assert_eq!(applications.len(), 1, "{row_tenant_id:?}");
            assert_eq!(applications[0].tenant_id, TASK_TENANT_ID);
        }
    }

    #[test]
    fn rejects_area_outside_task_event() {
        let csv = csv_row(TASK_TENANT_ID, TASK_EVENT_ID, OTHER_AREA_ID);
        let result = parse_applications_csv(
            csv.as_bytes(),
            TASK_TENANT_ID,
            TASK_EVENT_ID,
            &event_area_ids(),
        );
        assert!(result.is_err());
    }
}
