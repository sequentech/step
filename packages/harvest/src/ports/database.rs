// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use deadpool_postgres::Pool;
use std::sync::Arc;

/// The PostgreSQL pools route handlers open their transactions on.
#[rocket::async_trait]
pub trait DatabasePools: Send + Sync {
    async fn hasura(&self) -> Arc<Pool>;
    async fn keycloak(&self) -> Arc<Pool>;
}
