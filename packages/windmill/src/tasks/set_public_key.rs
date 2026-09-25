// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::adapters::keys_ceremony::{B4KeysBoard, PgKeysCeremonyStore};
use crate::adapters::system::SystemClock;
use crate::services::ceremonies::keys_ceremony::{record_public_key, KeysCeremonyUpdate};
use crate::services::database::get_hasura_pool;
use crate::types::error::Result;
use anyhow::{Context, Result as AnyhowResult};
use celery::error::TaskError;
use deadpool_postgres::Client as DbClient;
use tracing::instrument;

pub async fn set_public_key_impl(
    tenant_id: String,
    election_event_id: String,
    keys_ceremony_id: String,
) -> AnyhowResult<()> {
    let mut hasura_db_client: DbClient = get_hasura_pool().await.get().await?;

    let hasura_transaction = hasura_db_client.transaction().await?;
    let update = record_public_key(
        &PgKeysCeremonyStore {
            transaction: &hasura_transaction,
        },
        &B4KeysBoard {
            transaction: &hasura_transaction,
        },
        &SystemClock,
        &tenant_id,
        &election_event_id,
        &keys_ceremony_id,
    )
    .await?;
    if update == KeysCeremonyUpdate::Skipped {
        return Ok(());
    }

    hasura_transaction
        .commit()
        .await
        .with_context(|| "error comitting transaction")?;

    Ok(())
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn set_public_key(
    tenant_id: String,
    election_event_id: String,
    keys_ceremony_id: String,
) -> Result<()> {
    set_public_key_impl(tenant_id, election_event_id, keys_ceremony_id).await?;

    Ok(())
}
