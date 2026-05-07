// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Postgres connection pools for Hasura-tracked data and for the Keycloak database.

use anyhow::{anyhow, Result};
use async_once::AsyncOnce;
use celery::export::Arc;
use config::{Config, ConfigError, Environment};
use deadpool_postgres::{Client, Pool, PoolError, Runtime, SslMode};
use serde::{Deserialize, Serialize};
use std::env;
use tracing::instrument;

use super::sql_utils::assert_standard_conforming_strings;

#[cfg(any(feature = "fips_core", feature = "fips_full"))]
use openssl::ssl::{SslConnector, SslMethod};

#[cfg(any(feature = "fips_core", feature = "fips_full"))]
use postgres_openssl::MakeTlsConnector;

#[derive(Debug, Deserialize)]
/// Postgres connectivity and query limit configuration loaded from the environment.
pub struct PgConfig {
    /// Deadpool configuration for the Keycloak database.
    pub keycloak_db: deadpool_postgres::Config,
    /// Deadpool configuration for the Hasura database.
    pub hasura_db: deadpool_postgres::Config,
    /// Low limit used for queries that may return many rows.
    pub low_sql_limit: i32,
    /// Default limit used for paginated queries.
    pub default_sql_limit: i32,
    /// Default batch size used for bulk operations.
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
    /// Load Postgres configuration from environment variables.
    ///
    /// # Errors
    ///
    /// Returns an error if configuration cannot be built from the environment or deserialization fails.
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
/// Generate a pool for the Keycloak database.
///
/// # Errors
///
/// Returns an error if configuration is missing/invalid or the pool cannot be created.
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
/// Generate a pool for the Hasura database.
///
/// # Errors
///
/// Returns an error if configuration is missing/invalid or the pool cannot be created.
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
    static ref KEYCLOAK_POOL: AsyncOnce<Arc<Pool>> = AsyncOnce::new(async {
        let pool = generate_keycloak_pool().await.unwrap();
        assert_standard_conforming_strings(&pool)
            .await
            .expect("Keycloak DB: standard_conforming_strings check failed");
        pool
    });
    static ref HASURA_POOL: AsyncOnce<Arc<Pool>> = AsyncOnce::new(async {
        let pool = generate_hasura_pool().await.unwrap();
        assert_standard_conforming_strings(&pool)
            .await
            .expect("Hasura DB: standard_conforming_strings check failed");
        pool
    });
}

/// Return the process-wide Keycloak Postgres pool.
pub async fn get_keycloak_pool() -> Arc<Pool> {
    KEYCLOAK_POOL.get().await.clone()
}

/// Return the process-wide Hasura Postgres pool.
pub async fn get_hasura_pool() -> Arc<Pool> {
    HASURA_POOL.get().await.clone()
}
