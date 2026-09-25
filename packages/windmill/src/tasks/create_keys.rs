// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::adapters::keys_ceremony::{B4KeysBoard, PgKeysCeremonyStore};
use crate::services::ceremonies::keys_ceremony::{generate_keys, KeysCeremonyUpdate};
use crate::services::database::get_hasura_pool;
use crate::types::error::{Error, Result};
use anyhow::{Context, Result as AnyhowResult};
use celery::error::TaskError;
use deadpool_postgres::Client as DbClient;
use serde::{Deserialize, Serialize};
use tracing::instrument;

#[derive(Deserialize, Debug, Serialize, Clone)]
pub struct CreateKeysBody {
    pub trustee_pks: Vec<String>,
    pub threshold: usize,
}

pub async fn create_keys_impl(
    tenant_id: String,
    election_event_id: String,
    keys_ceremony_id: String,
) -> AnyhowResult<()> {
    let mut hasura_db_client: DbClient = get_hasura_pool().await.get().await?;

    let hasura_transaction = hasura_db_client.transaction().await?;

    let update = generate_keys(
        &PgKeysCeremonyStore {
            transaction: &hasura_transaction,
        },
        &B4KeysBoard {
            transaction: &hasura_transaction,
        },
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
#[celery::task]
pub async fn create_keys(
    tenant_id: String,
    election_event_id: String,
    keys_ceremony_id: String,
) -> Result<()> {
    create_keys_impl(tenant_id, election_event_id, keys_ceremony_id)
        .await
        .map_err(|err| Error::from(err.context("Task failed")))
}
