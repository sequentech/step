// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use deadpool_postgres::{Config as PgConfig, Pool, Runtime};
use std::env;
use tokio_postgres::NoTls;

pub async fn get_hasura_pool() -> Result<Pool, Box<dyn std::error::Error>> {
    let mut cfg = PgConfig::default();
    cfg.host = Some(env::var("HASURA_PG_HOST")?);
    cfg.port = Some(env::var("HASURA_PG_PORT")?.parse::<u16>()?);
    cfg.user = Some(env::var("HASURA_PG_USER")?);
    cfg.password = Some(env::var("HASURA_PG_PASSWORD")?);
    cfg.dbname = Some(env::var("HASURA_PG_DBNAME")?);
    Ok(cfg.create_pool(Some(Runtime::Tokio1), NoTls)?)
}
