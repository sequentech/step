// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Explicit bounded migration before public-only proof reads are enabled.
use crate::services::public_ballot_keys::{backfill, BackfillSource};
use crate::types::error::Result;
use anyhow::Context;
use celery::error::TaskError;

#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn backfill_ballot_verification_keys(
    tenant_id: String,
    election_event_id: String,
    source: BackfillSource,
) -> Result<usize> {
    let mut client = crate::services::database::get_hasura_pool()
        .await
        .get()
        .await
        .context("getting database client for verification key backfill")?;
    let tx = client.transaction().await?;
    let event = crate::postgres::election_event::get_election_event_by_id(
        &tx,
        &tenant_id,
        &election_event_id,
    )
    .await?;
    let count = backfill(&tx, &event, source).await?;
    tx.commit().await?;
    Ok(count)
}
