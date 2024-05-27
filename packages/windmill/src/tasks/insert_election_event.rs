// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::import_election_event::process_insert_election_event;
use crate::types::error::Result;
use celery::error::TaskError;
use sequent_core::types::hasura::core::ElectionEvent;
use tracing::instrument;

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task]
pub async fn insert_election_event_t(election_event: ElectionEvent, id: String) -> Result<()> {
    process_insert_election_event(election_event, id).await?;

    Ok(())
}
