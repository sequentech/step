// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::{
    services::import_election_event::{self as import_election_event_service},
    types::error::Result,
};
use celery::error::TaskError;
use serde::{Deserialize, Serialize};
use tracing::{event, instrument, Level};

#[derive(Deserialize, Debug, Clone, Serialize)]
pub struct ImportElectionEventBody {
    pub tenant_id: String,
    pub document_id: String,
    pub check_only: Option<bool>,
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task]
pub async fn import_election_event(
    object: ImportElectionEventBody,
    election_event_id: String,
    tenant_id: String,
) -> Result<()> {
    import_election_event_service::process(object, election_event_id, tenant_id).await?;

    Ok(())
}
