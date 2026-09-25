// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::ports::database::DatabasePools;
use deadpool_postgres::Pool;
use std::sync::Arc;
use windmill::services::database::{get_hasura_pool, get_keycloak_pool};

/// Windmill's process-wide pools, configured from `HASURA_DB__*` and
/// `KEYCLOAK_DB__*` on first use.
pub struct WindmillDatabasePools;

#[rocket::async_trait]
impl DatabasePools for WindmillDatabasePools {
    async fn hasura(&self) -> Arc<Pool> {
        get_hasura_pool().await
    }

    async fn keycloak(&self) -> Arc<Pool> {
        get_keycloak_pool().await
    }
}
