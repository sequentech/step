// SPDX-FileCopyrightText: 2024 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::vote_receipt;
use crate::{services::database::get_hasura_pool, types::error::Result};
use anyhow::{anyhow, Context};
use celery::error::TaskError;
use sequent_core::types::date_time::{DateFormat, TimeZone};
use tracing::instrument;

use deadpool_postgres::{Client as DbClient, Transaction};

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task]
pub async fn create_vote_receipt(
    element_id: String,
    ballot_id: String,
    ballot_tracker_url: String,
    tenant_id: String,
    election_event_id: String,
    election_id: String,
    area_id: String,
    voter_id: String,
    time_zone: Option<TimeZone>,
    date_format: Option<DateFormat>,
) -> Result<()> {
    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|err| anyhow!("{}", err))?;

    let hasura_transaction: Transaction<'_> = hasura_db_client
        .transaction()
        .await
        .map_err(|err| anyhow!("{}", err))?;

    vote_receipt::create_vote_receipt(
        &hasura_transaction,
        &element_id,
        &tenant_id,
        &election_event_id,
        &election_id,
        &area_id,
        &voter_id,
        &ballot_id,
        &ballot_tracker_url,
        time_zone,
        date_format,
    )
    .await
    .map_err(|err| anyhow!("{}", err))?;

    hasura_transaction
        .commit()
        .await
        .with_context(|| "Error committing create_vote_receipt transaction")
        .map_err(|err| anyhow!("{}", err))?;

    Ok(())
}
