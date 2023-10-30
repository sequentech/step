// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use celery::prelude::*;
use tracing::instrument;

#[instrument]
#[celery::task]
pub async fn tally_election_event(
    tenant_id: String,
    election_event_id: String,
) -> TaskResult<()> {
    Ok(())
}