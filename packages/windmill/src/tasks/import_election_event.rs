// SPDX-FileCopyrightText: 2024 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::{
    services::{database::get_hasura_pool, documents},
    types::error::Result,
};
use anyhow::{anyhow, Context};
use celery::error::TaskError;
use serde::{Deserialize, Serialize};
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

    dbg!(&document);

    let temp_file = documents::get_document_as_temp_file(&object.tenant_id, &document)
        .await
        .map_err(|err| anyhow!("Error trying to get document as temporary file {err}"))?;

    dbg!(&temp_file);

    Ok(())
}
