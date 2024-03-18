// SPDX-FileCopyrightText: 2024 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::{
    services::{database::get_hasura_pool, documents::fetch_document},
    types::error::Result,
};
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
    let doc = fetch_document(object.tenant_id, "".to_string(), object.document_id).await?;

    dbg!(&doc);

    Ok(())
}
