// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Result};
use async_once::AsyncOnce;
use celery::export::Arc;
use config::{Config, ConfigError, Environment};
use deadpool_postgres::{Client, Pool, PoolError, Runtime, SslMode};
use serde::{Deserialize, Serialize};
use std::env;
use tracing::instrument;

#[cfg(any(feature = "fips_core", feature = "fips_full"))]
use openssl::ssl::{SslConnector, SslMethod};

#[cfg(any(feature = "fips_core", feature = "fips_full"))]
use postgres_openssl::MakeTlsConnector;

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
    let config = PgConfig::from_env()?;

    cfg_if::cfg_if! {
        if #[cfg(any(feature = "fips_core", feature = "fips_full"))] {
            if  config.keycloak_db.ssl_mode == Some(SslMode::Prefer) ||
                config.keycloak_db.ssl_mode == Some(SslMode::Require)
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
                    .keycloak_db
                    .create_pool(Some(Runtime::Tokio1), connector_tls)
                    .map_err(|err|
                        anyhow!("error creating pool: {}", err)
                    )?;
                Ok(Arc::new(pool))
            } else {
                let pool = config
                    .keycloak_db
                    .create_pool(Some(Runtime::Tokio1), tokio_postgres::NoTls)
                    .map_err(|err|
                        anyhow!("error creating pool: {}", err)
                    )?;
                Ok(Arc::new(pool))
            }
        } else {
            let pool = config
                .keycloak_db
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

lazy_static! {
    static ref KEYCLOAK_POOL: AsyncOnce<Arc<Pool>> =
        AsyncOnce::new(async { generate_keycloak_pool().await.unwrap() });
    static ref HASURA_POOL: AsyncOnce<Arc<Pool>> =
        AsyncOnce::new(async { generate_hasura_pool().await.unwrap() });
}

pub async fn get_keycloak_pool() -> Arc<Pool> {
    KEYCLOAK_POOL.get().await.clone()
}

pub async fn get_hasura_pool() -> Arc<Pool> {
    HASURA_POOL.get().await.clone()
}
