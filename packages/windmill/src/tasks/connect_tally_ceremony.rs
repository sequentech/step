// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use tracing::{event, instrument, Level};
use celery::prelude::TaskError;
use crate::types::error::{Error, Result};

#[instrument]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(time_limit = 120000)]
pub async fn connect_tally_ceremony(
    tenant_id: String,
    election_event_id: String,
    tally_session_id: String,
) -> Result<()> {
    Ok(())
}