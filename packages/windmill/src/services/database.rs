use async_once::AsyncOnce;
use celery::export::Arc;
use config::{Config, Environment, ConfigError};
use deadpool_postgres::{Client, Pool, PoolError, Runtime, SslMode};
use serde::{Deserialize, Serialize};
use std::env;

#[cfg(any(feature = "fips_core", feature = "fips_full"))]
use openssl::ssl::{SslConnector, SslMethod};

#[cfg(any(feature = "fips_core", feature = "fips_full"))]
use postgres_openssl::MakeTlsConnector;

#[derive(Debug, Deserialize)]
struct PgConfig {
    keycloak_db: deadpool_postgres::Config,
}

impl PgConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        Config::builder()
            .add_source(Environment::default().separator("_"))
            .build()
            .unwrap()
            .try_deserialize()
    }
}

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("An internal error occurred. Please try again later.")]
    PoolError(#[from] PoolError),
}

pub async fn generate_database_pool() -> Arc<Pool> {
    let config = PgConfig::from_env().unwrap();

    cfg_if::cfg_if! {
        if #[cfg(any(feature = "fips_core", feature = "fips_full"))] {
            if  config.keycloak_db.ssl_mode == Some(SslMode::Prefer) ||
                config.keycloak_db.ssl_mode == Some(SslMode::Require)
            {
                let mut builder = SslConnector::builder(SslMethod::tls()).unwrap();
                builder.set_ca_file(
                    env::var("KEYCLOAK_DB_CA_PATH").unwrap()
                ).unwrap();
                let connector_tls = MakeTlsConnector::new(builder.build());

                let pool = config
                    .keycloak_db
                    .create_pool(Some(Runtime::Tokio1), connector_tls)
                    .unwrap();
                Arc::new(pool)
            } else {
                let pool = config
                    .keycloak_db
                    .create_pool(Some(Runtime::Tokio1), tokio_postgres::NoTls)
                    .unwrap();
                Arc::new(pool)
            }
        } else {
            let pool = config
                .keycloak_db
                .create_pool(Some(Runtime::Tokio1), tokio_postgres::NoTls)
                .unwrap();
            Arc::new(pool)
        }
    }
}

lazy_static! {
    static ref DATABASE_POOL: AsyncOnce<Arc<Pool>> =
        AsyncOnce::new(async { generate_database_pool().await });
}

pub async fn get_database_pool() -> Arc<Pool> {
    DATABASE_POOL.get().await.clone()
}
