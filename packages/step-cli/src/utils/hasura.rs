// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use deadpool_postgres::{Config as PgConfig, Pool, Runtime};
use std::env;
use tokio_postgres::NoTls;

/// Get hasura pool
pub fn get_hasura_pool() -> Result<Pool, Box<dyn std::error::Error>> {
    let cfg = PgConfig {
        host: Some(env::var("HASURA_PG_HOST")?),
        port: Some(env::var("HASURA_PG_PORT")?.parse::<u16>()?),
        user: Some(env::var("HASURA_PG_USER")?),
        password: Some(env::var("HASURA_PG_PASSWORD")?),
        dbname: Some(env::var("HASURA_PG_DBNAME")?),
        ..Default::default()
    };

    Ok(cfg.create_pool(Some(Runtime::Tokio1), NoTls)?)
}
