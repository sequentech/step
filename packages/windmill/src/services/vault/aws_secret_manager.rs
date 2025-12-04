// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::{Vault, VaultManagerType};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use aws_sdk_secretsmanager::Client;
use sequent_core::util::aws::get_from_env_aws_config;
use std::env;
use tracing::{info, instrument};

#[derive(Debug)]
pub struct AwsSecretManager;

impl AwsSecretManager {
    fn get_prefixed_key(&self, key: String) -> Result<String> {
        let key_prefix = env::var("AWS_SM_KEY_PREFIX").context("AWS_SM_KEY_PREFIX must be set")?;
        Ok(key_prefix + key.as_str())
    }
}

#[async_trait]
impl Vault for AwsSecretManager {
    // TODO: add back skip(value)
    #[instrument(err)]
    async fn save_secret(&self, key: String, value: String) -> Result<()> {
        let shared_config = get_from_env_aws_config()
            .await
            .map_err(|err| anyhow!("Error getting env aws config: {err:?}"))?;
        let client = Client::new(&shared_config);

        client
            .create_secret()
            .name(self.get_prefixed_key(key)?)
            .secret_string(value)
            .send()
            .await
            .map_err(|err| anyhow!("Error creating secret: {err:?}"))?;

        Ok(())
    }

    #[instrument(err)]
    async fn read_secret(&self, key: String) -> Result<Option<String>> {
        let shared_config = get_from_env_aws_config()
            .await
            .map_err(|err| anyhow!("Error getting env aws config: {err:?}"))?;
        let client = Client::new(&shared_config);

        let final_key = self.get_prefixed_key(key)?;
        info!("reading secret key: {:?}", final_key);

        let resp = client.get_secret_value().secret_id(final_key).send().await;

        match resp {
            Ok(data) => Ok(data.secret_string().map(|s| s.to_string())),
            Err(_) => Ok(None),
        }
    }

    #[instrument]
    fn vault_type(&self) -> VaultManagerType {
        VaultManagerType::AwsSecretManager
    }
}
