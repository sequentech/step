use anyhow::{anyhow, Result};
use async_once::AsyncOnce;
use celery::export::Arc;
use config::{Config, ConfigError, Environment};
use deadpool_postgres::{Client, Pool, PoolError, Runtime, SslMode};
use serde::{Deserialize, Serialize};
use std::env;

#[cfg(any(feature = "fips_core", feature = "fips_full"))]
use openssl::ssl::{SslConnector, SslMethod};

#[cfg(any(feature = "fips_core", feature = "fips_full"))]
use postgres_openssl::MakeTlsConnector;

#[derive(Debug, Deserialize)]
pub struct PgConfig {
    pub keycloak_db: deadpool_postgres::Config,
    pub low_sql_limit: i32,
    pub default_sql_limit: i32,
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

pub async fn generate_database_pool() -> Result<Arc<Pool>> {
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

lazy_static! {
    static ref DATABASE_POOL: AsyncOnce<Arc<Pool>> =
        AsyncOnce::new(async { generate_database_pool().await.unwrap() });
}

pub async fn get_database_pool() -> Arc<Pool> {
    DATABASE_POOL.get().await.clone()
}
