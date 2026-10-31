// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::ports::database::DatabasePools;
use deadpool_postgres::Pool;
use std::sync::Arc;

/// Pools a test opened itself, usually on its migrated test database.
pub struct FixedDatabasePools {
    pub hasura: Arc<Pool>,
    pub keycloak: Arc<Pool>,
}

#[rocket::async_trait]
impl DatabasePools for FixedDatabasePools {
    async fn hasura(&self) -> Arc<Pool> {
        self.hasura.clone()
    }

    async fn keycloak(&self) -> Arc<Pool> {
        self.keycloak.clone()
    }
}
