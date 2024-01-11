use crate::util::aws::get_from_env_aws_config;
use anyhow::Result;
use aws_sdk_secretsmanager::Client;

pub async fn save_secret(key: String, value: String) -> Result<()> {
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

pub async fn read_secret(key: String) -> Result<Option<String>> {
    let shared_config = get_from_env_aws_config().await?;
    let client = Client::new(&shared_config);

    let resp = client.get_secret_value().secret_id(key).send().await?;

    Ok(resp.secret_string().map(|s| s.to_string()))
}
