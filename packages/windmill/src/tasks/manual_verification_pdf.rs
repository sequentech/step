// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::manual_verification;
use crate::{services::database::get_hasura_pool, types::error::Result};
use anyhow::{anyhow, Context};
use celery::error::TaskError;
use tracing::instrument;

use deadpool_postgres::{Client as DbClient, Transaction};

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task]
pub async fn get_manual_verification_pdf(
    document_id: String,
    tenant_id: String,
    election_event_id: String,
    voter_id: String,
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

    manual_verification::get_manual_verification_pdf(
        &hasura_transaction,
        &document_id,
        &tenant_id,
        &election_event_id,
        &voter_id,
    )
    .await
    .map_err(|err| anyhow!("{}", err))?;

    Ok(())
}
