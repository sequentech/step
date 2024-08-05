// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use config::{Config, Environment};
use serde::Deserialize;
use anyhow::{anyhow, Result};
use tracing::info;

#[derive(Debug, Deserialize)]
pub struct DatabaseSettings {
    pub keycloak_db: deadpool_postgres::Config,
    pub hasura_db: deadpool_postgres::Config,
    pub low_sql_limit: i32,
    pub default_sql_limit: i32,
    pub default_sql_batch_size: i32,
    pub immudb_server_url: String,
    pub immudb_user: String,
    pub immudb_password: String,
    pub immudb_index_db: String,
}

impl Default for DatabaseSettings {
    fn default() -> Self {
        DatabaseSettings {
            low_sql_limit: 1000,
            default_sql_limit: 20,
            default_sql_batch_size: 1000,
            ..Default::default()
        }
    }
}

#[derive(Debug, Deserialize, Default)]
pub struct AwsS3 {
    pub bucket: String,
    pub public_bucket: String,
    pub private_uri: String,
    pub public_uri: String,
    pub access_key: String,
    pub access_secret: String,
    pub jwks_cache_policy: String,
    pub upload_expiration_secs: String,
    pub fetch_expiration_secs: String,
    pub max_upload_bytes: String,
}

#[derive(Debug, Deserialize, Default)]
pub struct Settings {
    db: DatabaseSettings,
    aws_s3: AwsS3,
}

impl Settings {
    pub fn from_env() -> Result<Self> {
        Config::builder()
            .add_source(Environment::default().separator("__"))
            .build()
            .map_err(|err| anyhow!("error building Settings from Env: {}", err))?
            .try_deserialize()
            .map_err(|err| anyhow!("error deserializing Settings: {}", err))
    }
}

lazy_static! {
    pub static ref SETTINGS: Settings = {
        info!("loading settings..");
        Settings::from_env().unwrap()
    };
}
