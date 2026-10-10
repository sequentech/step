// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Result};
use celery::export::Arc;
use config::{Config, ConfigError, Environment};
use deadpool_postgres::{Client, Pool, PoolConfig, PoolError, Runtime, SslMode};
use serde::{Deserialize, Serialize};
use std::env;
use std::time::Duration;
use tracing::instrument;

use super::sql_utils::assert_standard_conforming_strings;

#[cfg(any(feature = "fips_core", feature = "fips_full"))]
use openssl::ssl::{SslConnector, SslMethod};

#[cfg(any(feature = "fips_core", feature = "fips_full"))]
use postgres_openssl::MakeTlsConnector;
use tokio::sync::OnceCell;

#[derive(Debug, Deserialize)]
pub struct PgConfig {
    pub keycloak_db: deadpool_postgres::Config,
    pub hasura_db: deadpool_postgres::Config,
    pub low_sql_limit: i32,
    pub default_sql_limit: i32,
    pub default_sql_batch_size: i32,
}

impl Default for PgConfig {
    fn default() -> Self {
        PgConfig {
            keycloak_db: deadpool_postgres::Config::default(),
            hasura_db: deadpool_postgres::Config::default(),
            low_sql_limit: 1000,
            default_sql_limit: 20,
            default_sql_batch_size: 1000,
        }
    }
}

impl PgConfig {
    pub fn from_env() -> Result<Self> {
        Config::builder()
            .add_source(Environment::default().separator("__"))
            .build()
            .map_err(|err| anyhow!("error building Config from Env: {}", err))?
            .try_deserialize()
            .map_err(|err| anyhow!("error deserializing PgConfig: {}", err))
    }
}

#[instrument(err)]
pub async fn generate_keycloak_pool() -> Result<Arc<Pool>> {
    let config: deadpool_postgres::Config = Config::builder()
        .add_source(Environment::default().separator("__"))
        .build()?
        .get("keycloak_db")?;

    cfg_if::cfg_if! {
        if #[cfg(any(feature = "fips_core", feature = "fips_full"))] {
            if  config.ssl_mode == Some(SslMode::Prefer) ||
                config.ssl_mode == Some(SslMode::Require)
            {
                let mut builder = SslConnector::builder(SslMethod::tls())
                    .map_err(|err|
                        anyhow!("error building SsslConnector: {}", err)
                    )?;
                builder.set_ca_file(
                    env::var("KEYCLOAK_DB_CA_PATH")
                    .map_err(|err|
                        anyhow!("error loading KEYCLOAK_DB_CA_PATH var: {}", err)
                    )?
                )
                .map_err(|err|
                    anyhow!("error in builder.set_ca_file(): {}", err)
                )?;
                let connector_tls = MakeTlsConnector::new(builder.build());

                let pool = config
                    .create_pool(Some(Runtime::Tokio1), connector_tls)
                    .map_err(|err|
                        anyhow!("error creating pool: {}", err)
                    )?;
                Ok(Arc::new(pool))
            } else {
                let pool = config
                    .create_pool(Some(Runtime::Tokio1), tokio_postgres::NoTls)
                    .map_err(|err|
                        anyhow!("error creating pool: {}", err)
                    )?;
                Ok(Arc::new(pool))
            }
        } else {
            let pool = config
                .create_pool(Some(Runtime::Tokio1), tokio_postgres::NoTls)
                .map_err(|err|
                    anyhow!("error creating pool: {}", err)
                )?;
            Ok(Arc::new(pool))
        }
    }
}

#[instrument(err)]
pub async fn generate_hasura_pool() -> Result<Arc<Pool>> {
    let config = PgConfig::from_env()?;

    cfg_if::cfg_if! {
        if #[cfg(any(feature = "fips_core", feature = "fips_full"))] {
            if  config.hasura_db.ssl_mode == Some(SslMode::Prefer) ||
                config.hasura_db.ssl_mode == Some(SslMode::Require)
            {
                let mut builder = SslConnector::builder(SslMethod::tls())
                    .map_err(|err|
                        anyhow!("error building SsslConnector: {}", err)
                    )?;
                builder.set_ca_file(
                    env::var("HASURA_DB_CA_PATH")
                    .map_err(|err|
                        anyhow!("error loading HASURA_DB_CA_PATH var: {}", err)
                    )?
                )
                .map_err(|err|
                    anyhow!("error in builder.set_ca_file(): {}", err)
                )?;
                let connector_tls = MakeTlsConnector::new(builder.build());

                let pool = config
                    .hasura_db
                    .create_pool(Some(Runtime::Tokio1), connector_tls)
                    .map_err(|err|
                        anyhow!("error creating pool: {}", err)
                    )?;
                Ok(Arc::new(pool))
            } else {
                let pool = config
                    .hasura_db
                    .create_pool(Some(Runtime::Tokio1), tokio_postgres::NoTls)
                    .map_err(|err|
                        anyhow!("error creating pool: {}", err)
                    )?;
                Ok(Arc::new(pool))
            }
        } else {
            let pool = config
                .hasura_db
                .create_pool(Some(Runtime::Tokio1), tokio_postgres::NoTls)
                .map_err(|err|
                    anyhow!("error creating pool: {}", err)
                )?;
            Ok(Arc::new(pool))
        }
    }
}

/// Connection and pool limits of the task-queue database unless `QUEUE_DB__*` sets them, so
/// a request that enqueues fails fast instead of waiting on an unreachable database.
const QUEUE_DB_TIMEOUT: Duration = Duration::from_secs(5);

/// The environment's task-queue database (`QUEUE_DB__*`), where PGMQ keeps the queues.
#[instrument(err)]
pub async fn generate_queue_pool() -> Result<Arc<Pool>> {
    let mut config: deadpool_postgres::Config = Config::builder()
        .add_source(Environment::default().separator("__"))
        .build()?
        .get("queue_db")?;
    config.connect_timeout.get_or_insert(QUEUE_DB_TIMEOUT);
    let timeouts = &mut config.pool.get_or_insert_with(PoolConfig::default).timeouts;
    timeouts.wait.get_or_insert(QUEUE_DB_TIMEOUT);
    timeouts.create.get_or_insert(QUEUE_DB_TIMEOUT);
    timeouts.recycle.get_or_insert(QUEUE_DB_TIMEOUT);

    cfg_if::cfg_if! {
        if #[cfg(any(feature = "fips_core", feature = "fips_full"))] {
            if  config.ssl_mode == Some(SslMode::Prefer) ||
                config.ssl_mode == Some(SslMode::Require)
            {
                let mut builder = SslConnector::builder(SslMethod::tls())
                    .map_err(|err|
                        anyhow!("error building SslConnector: {}", err)
                    )?;
                builder.set_ca_file(
                    env::var("QUEUE_DB_CA_PATH")
                    .map_err(|err|
                        anyhow!("error loading QUEUE_DB_CA_PATH var: {}", err)
                    )?
                )
                .map_err(|err|
                    anyhow!("error in builder.set_ca_file(): {}", err)
                )?;
                let connector_tls = MakeTlsConnector::new(builder.build());

                let pool = config
                    .create_pool(Some(Runtime::Tokio1), connector_tls)
                    .map_err(|err|
                        anyhow!("error creating pool: {}", err)
                    )?;
                Ok(Arc::new(pool))
            } else {
                let pool = config
                    .create_pool(Some(Runtime::Tokio1), tokio_postgres::NoTls)
                    .map_err(|err|
                        anyhow!("error creating pool: {}", err)
                    )?;
                Ok(Arc::new(pool))
            }
        } else {
            let pool = config
                .create_pool(Some(Runtime::Tokio1), tokio_postgres::NoTls)
                .map_err(|err|
                    anyhow!("error creating pool: {}", err)
                )?;
            Ok(Arc::new(pool))
        }
    }
}

static KEYCLOAK_POOL: OnceCell<Arc<Pool>> = OnceCell::const_new();

static QUEUE_POOL: OnceCell<Arc<Pool>> = OnceCell::const_new();

static HASURA_POOL: OnceCell<Arc<Pool>> = OnceCell::const_new();

pub async fn get_keycloak_pool() -> Arc<Pool> {
    KEYCLOAK_POOL
        .get_or_init(|| async {
            let pool = generate_keycloak_pool().await.unwrap();
            assert_standard_conforming_strings(&pool)
                .await
                .expect("Keycloak DB: standard_conforming_strings check failed");
            pool
        })
        .await
        .clone()
}

pub async fn get_queue_pool() -> Arc<Pool> {
    try_get_queue_pool()
        .await
        .expect("Task-queue DB: could not create the pool")
}

/// The task-queue pool, or the error that kept it from being created; a later call tries again.
pub async fn try_get_queue_pool() -> Result<Arc<Pool>> {
    QUEUE_POOL
        .get_or_try_init(|| async {
            let pool = generate_queue_pool().await?;
            assert_standard_conforming_strings(&pool)
                .await
                .map_err(|error| {
                    anyhow!("Task-queue DB: standard_conforming_strings check failed: {error}")
                })?;
            Ok::<_, anyhow::Error>(pool)
        })
        .await
        .cloned()
}

pub async fn get_hasura_pool() -> Arc<Pool> {
    HASURA_POOL
        .get_or_init(|| async {
            let pool = generate_hasura_pool().await.unwrap();
            assert_standard_conforming_strings(&pool)
                .await
                .expect("Hasura DB: standard_conforming_strings check failed");
            pool
        })
        .await
        .clone()
}
