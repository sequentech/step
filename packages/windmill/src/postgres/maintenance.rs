// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::database::{get_hasura_pool, get_keycloak_pool};
use crate::types::error::Result;
use anyhow::anyhow;
use deadpool_postgres::Client as DbClient;
use tracing::{info, instrument};

#[instrument(err)]
pub async fn vacuum_analyze_direct() -> Result<()> {
    info!("Performing mainteinance running VACUUM ANALYZE");

    let hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|error| anyhow!("Error obtaining db connection to hasura db: {error:?}"))?;
    let keycloak_db_client: DbClient = get_keycloak_pool()
        .await
        .get()
        .await
        .map_err(|error| anyhow!("Error obtaining db connection to keycloak db: {error:?}"))?;

    // Execute database maintenance
    hasura_db_client
        .batch_execute("VACUUM ANALYZE")
        .await
        .map_err(|error| anyhow!("Error running VACUUM ANALYZE on hasura db: {error:?}"))?;
    keycloak_db_client
        .batch_execute("VACUUM ANALYZE")
        .await
        .map_err(|error| anyhow!("Error running VACUUM ANALYZE on keycloak db: {error:?}"))?;

    info!("Mainteinance complete.");
    Ok(())
}
