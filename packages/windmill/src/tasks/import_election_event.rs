// SPDX-FileCopyrightText: 2024 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::fs::File;

use crate::hasura::election_event::get_election_event;
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
use serde::{Deserialize, Serialize};
use tracing::instrument;

#[derive(Deserialize, Debug, Clone, Serialize)]
pub struct ImportElectionEventBody {
    pub tenant_id: String,
    pub document_id: String,
    pub check_only: Option<bool>,
}

#[instrument(err)]
pub async fn get_document(object: ImportElectionEventBody) -> Result<ImportElectionEventSchema> {
    let document = documents::get_document(&object.tenant_id, None, &object.document_id)
        .await?
        .ok_or(anyhow!(
            "Error trying to get document id {}: not found",
            &object.document_id
        ))?;

    let temp_file_path = documents::get_document_as_temp_file(&object.tenant_id, &document)
        .await
        .map_err(|err| anyhow!("Error trying to get document as temporary file {err}"))?;

    let file = File::open(temp_file_path)?;

    let data: ImportElectionEventSchema = serde_json::from_reader(file)?;

    let auth_headers = keycloak::get_client_credentials().await?;
    let tenant_id = data.tenant_id.to_string();
    let election_event_id = data.election_event_data.id.to_string();

    let events = get_election_event(auth_headers, tenant_id, election_event_id.clone())
        .await?
        .data
        .ok_or(anyhow!(
            "Error fetching election event: {}",
            election_event_id
        ))?
        .sequent_backend_election_event;

    if events.len() > 0 {
        return Err(anyhow!("Election event already exists {}", election_event_id).into());
    }

    Ok(data)
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task]
pub async fn import_election_event(object: ImportElectionEventBody) -> Result<()> {
    let data: ImportElectionEventSchema = get_document(object).await?;

    import_election_event_service::process(&data).await?;

    Ok(())
}
