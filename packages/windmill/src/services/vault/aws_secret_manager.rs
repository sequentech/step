use super::Vault;
use crate::util::aws::get_from_env_aws_config;
use anyhow::Result;
use async_trait::async_trait;
use aws_sdk_secretsmanager::Client;
use tracing::instrument;

#[derive(Debug)]
pub struct AwsSecretManager;

#[async_trait]
impl Vault for AwsSecretManager {
    #[instrument(skip(value), err)]
    async fn save_secret(&self, key: String, value: String) -> Result<()> {
        let shared_config = get_from_env_aws_config().await?;
        let client = Client::new(&shared_config);

        client
            .create_secret()
            .name(key)
            .secret_string(value)
            .send()
            .await?;

        Ok(())
    }

    #[instrument(err)]
    async fn read_secret(&self, key: String) -> Result<Option<String>> {
        let shared_config = get_from_env_aws_config().await?;
        let client = Client::new(&shared_config);

        let resp = client.get_secret_value().secret_id(key).send().await?;

        Ok(resp.secret_string().map(|s| s.to_string()))
    }
}
