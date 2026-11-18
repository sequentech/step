// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Result};
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

    /// Bounds a requested page size to `low_sql_limit`, rejecting values that
    /// are not positive, as well as a `low_sql_limit` that is not.
    pub fn page_limit(&self, requested: i64) -> Result<usize> {
        if requested <= 0 {
            return Err(anyhow!(
                "Invalid page limit {requested}: it must be positive"
            ));
        }
        let limit = std::cmp::min(requested, self.count_limit()?);
        usize::try_from(limit).map_err(|err| anyhow!("Invalid page limit {limit}: {err}"))
    }

    /// The most matches a list counts, `low_sql_limit`: past it, the list only reports that
    /// there are more. Rejects a `low_sql_limit` that is not positive.
    pub fn count_limit(&self) -> Result<i64> {
        if self.low_sql_limit <= 0 {
            return Err(anyhow!(
                "Invalid low_sql_limit {}: it must be positive",
                self.low_sql_limit
            ));
        }
        Ok(i64::from(self.low_sql_limit))
    }
}

#[cfg(test)]
mod count_limit_tests {
    use super::*;

    #[test]
    fn lists_count_up_to_the_low_sql_limit() {
        let pg_config = PgConfig {
            low_sql_limit: 50000,
            ..PgConfig::default()
        };
        assert_eq!(pg_config.count_limit().unwrap(), 50000);
    }

    #[test]
    fn a_non_positive_low_sql_limit_is_rejected() {
        for low_sql_limit in [0, -1] {
            let pg_config = PgConfig {
                low_sql_limit,
                ..PgConfig::default()
            };
            assert!(pg_config.count_limit().is_err());
        }
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

static KEYCLOAK_POOL: OnceCell<Arc<Pool>> = OnceCell::const_new();

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
