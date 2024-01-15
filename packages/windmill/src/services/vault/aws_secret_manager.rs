use super::Vault;
use crate::util::aws::get_from_env_aws_config;
use anyhow::Result;
use aws_sdk_secretsmanager::Client;
use tracing::instrument;

pub struct AwsSecretManager;

impl AwsSecretManager {
    pub fn new() -> Self {
        Self {}
    }
}

impl Vault for AwsSecretManager {
    #[instrument(skip(value), err)]
    async fn save_secret(key: String, value: String) -> Result<()> {
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
    async fn read_secret(key: String) -> Result<Option<String>> {
        let shared_config = get_from_env_aws_config().await?;
        let client = Client::new(&shared_config);

        let resp = client.get_secret_value().secret_id(key).send().await?;

        Ok(resp.secret_string().map(|s| s.to_string()))
    }
}
