// SPDX-FileCopyrightText: 2024 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::hasura::election_event::get_election_event;
use crate::{
    services::{
        database::get_hasura_pool,
        documents,
        import_election_event::{self as import_election_event_service, ImportElectionEventSchema},
    },
    types::error::Result,
};
use crate::services::import_election_event::upsert_keycloak_realm;
use crate::services::import_election_event::upsert_immu_board;
use crate::services::import_election_event::insert_election_event_db;
use anyhow::{anyhow, Context};
use celery::error::TaskError;
use sequent_core::services::keycloak;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use tracing::instrument;
use uuid::Uuid;

#[derive(Deserialize, Debug, Clone, Serialize)]
pub struct ImportElectionEventBody {
    pub tenant_id: String,
    pub document_id: String,
    pub check_only: Option<bool>,
}

#[instrument(err, skip_all)]
pub fn replace_ids(
    data_str: &str,
    original_data: &ImportElectionEventSchema,
    replace_event_id: bool,
    id_opt: Option<String>,
) -> Result<ImportElectionEventSchema> {
    let mut ids_to_replace: Vec<String> = vec![];
    if replace_event_id && id_opt.is_none() {
        ids_to_replace.push(original_data.election_event_data.id.clone());
    }

    let mut election_ids = original_data
        .elections
        .iter()
        .map(|element| element.id.to_string())
        .collect();
    ids_to_replace.append(&mut election_ids);

    let mut contest_ids = original_data
        .contests
        .iter()
        .map(|element| element.id.to_string())
        .collect();
    ids_to_replace.append(&mut contest_ids);

    let mut candidate_ids = original_data
        .candidates
        .iter()
        .map(|element| element.id.to_string())
        .collect();
    ids_to_replace.append(&mut candidate_ids);

    let mut area_ids = original_data
        .areas
        .iter()
        .map(|element| element.id.to_string())
        .collect();
    ids_to_replace.append(&mut area_ids);

    let mut area_contest_ids = original_data
        .area_contest_list
        .iter()
        .map(|element| element.id.to_string())
        .collect();
    ids_to_replace.append(&mut area_contest_ids);

    let mut new_data = String::from(data_str);

    if let Some(id) = id_opt {
        new_data = new_data.replace(&original_data.election_event_data.id, &id);
    }
    for id in ids_to_replace {
        let uuid = Uuid::new_v4().to_string();
        new_data = new_data.replace(&id, &uuid);
    }

    let data: ImportElectionEventSchema = serde_json::from_str(&new_data)?;
    Ok(data.clone())
}

#[instrument(err)]
pub async fn get_document(
    object: ImportElectionEventBody,
    id: Option<String>,
) -> Result<ImportElectionEventSchema> {
    let document = documents::get_document(&object.tenant_id, None, &object.document_id)
        .await?
        .ok_or(anyhow!(
            "Error trying to get document id {}: not found",
            &object.document_id
        ))?;

    let temp_file_path = documents::get_document_as_temp_file(&object.tenant_id, &document)
        .await
        .map_err(|err| anyhow!("Error trying to get document as temporary file {err}"))?;

    let mut file = File::open(temp_file_path)?;

    let mut data_str = String::new();
    file.read_to_string(&mut data_str)?;

    let original_data: ImportElectionEventSchema = serde_json::from_str(&data_str)?;

    let auth_headers = keycloak::get_client_credentials().await?;
    let tenant_id = original_data.tenant_id.to_string();
    let election_event_id = original_data.election_event_data.id.to_string();

    let events = get_election_event(auth_headers, tenant_id, election_event_id.clone())
        .await?
        .data
        .ok_or(anyhow!(
            "Error fetching election event: {}",
            election_event_id
        ))?
        .sequent_backend_election_event;

    let replace_event_id = events.len() > 0;

    let data = replace_ids(&data_str, &original_data, replace_event_id, id)?;

    Ok(data)
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task]
pub async fn import_election_event(object: ImportElectionEventBody, id: String) -> Result<()> {
    let data: ImportElectionEventSchema = get_document(object, Some(id)).await?;

    import_election_event_service::process(&data).await?;

    Ok(())
}
