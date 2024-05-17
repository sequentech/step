// SPDX-FileCopyrightText: 2024 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::hasura::election_event::get_election_event;
use crate::services::import_election_event::insert_election_event_db;
use crate::services::import_election_event::upsert_immu_board;
use crate::services::import_election_event::upsert_keycloak_realm;
use crate::{
    services::{
        database::get_hasura_pool,
        documents,
        import_election_event::{self as import_election_event_service, ImportElectionEventSchema},
    },
    types::error::Result,
};
use anyhow::{anyhow, Context};
use celery::error::TaskError;
use sequent_core::services::keycloak;
use sequent_core::services::replace_uuids::replace_uuids;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use tracing::{event, instrument, Level};
use uuid::Uuid;

#[derive(Deserialize, Debug, Clone, Serialize)]
pub struct ImportElectionEventBody {
    pub tenant_id: String,
    pub document_id: String,
    pub check_only: Option<bool>,
}

#[instrument(err, skip(data_str, original_data))]
pub fn replace_ids(
    data_str: &str,
    original_data: &ImportElectionEventSchema,
    id_opt: Option<String>,
    tenant_id: String,
) -> Result<ImportElectionEventSchema> {
    let keep: Vec<String> = if id_opt.is_some() {
        vec![
            original_data.election_event.id.clone(),
            original_data.tenant_id.clone().to_string(),
        ]
    } else {
        vec![original_data.tenant_id.clone().to_string()]
    };
    let mut new_data = replace_uuids(data_str, keep);
    let before: String = data_str.chars().take(7000).collect();
    let after: String = new_data.as_str().chars().take(7000).collect();
    event!(Level::INFO, "before: {:?}", before);
    event!(Level::INFO, "after: {:?}", after);

    if let Some(id) = id_opt {
        new_data = new_data.replace(&original_data.election_event.id, &id);
    }
    if original_data.tenant_id.to_string() != tenant_id {
        new_data = new_data.replace(&original_data.tenant_id.to_string(), &tenant_id);
    }

    let data: ImportElectionEventSchema = serde_json::from_str(&new_data)?;
    Ok(data.clone())
}

#[instrument(err)]
pub async fn get_document(
    object: ImportElectionEventBody,
    id: Option<String>,
    tenant_id: String,
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
    let election_event_id = original_data.election_event.id.to_string();

    let events = get_election_event(auth_headers, tenant_id.clone(), election_event_id.clone())
        .await?
        .data
        .ok_or(anyhow!(
            "Error fetching election event: {}",
            election_event_id
        ))?
        .sequent_backend_election_event;

    let replace_id = if let Some(id_val) = id {
        if events.len() > 0 && election_event_id != id_val {
            Some(id_val)
        } else {
            None
        }
    } else {
        None
    };

    let data = replace_ids(&data_str, &original_data, replace_id, tenant_id)?;

    Ok(data)
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task]
pub async fn import_election_event(
    object: ImportElectionEventBody,
    id: String,
    tenant_id: String,
) -> Result<()> {
    let data: ImportElectionEventSchema = get_document(object, Some(id), tenant_id).await?;

    import_election_event_service::process(&data).await?;

    Ok(())
}
