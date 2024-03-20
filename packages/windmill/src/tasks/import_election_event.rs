// SPDX-FileCopyrightText: 2024 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::fs::File;

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
use serde::{Deserialize, Serialize};
use serde_json::value::Value;
use tracing::instrument;

#[derive(Deserialize, Debug, Clone, Serialize)]
pub struct ImportElectionEventBody {
    pub tenant_id: String,
    pub document_id: String,
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task]
pub async fn import_election_event(object: ImportElectionEventBody) -> Result<()> {
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

    let data: ImportElectionEventSchema = serde_json::from_reader(file)?;

    import_election_event_service::process(&data).await;

    Ok(())
}
