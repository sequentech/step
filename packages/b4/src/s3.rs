// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use aws_config::BehaviorVersion;
use aws_sdk_s3::{presigning::PresigningConfig, Client};
use std::time::Duration;

pub async fn init_s3_client() -> Client {
    let config = aws_config::load_defaults(BehaviorVersion::latest()).await;

    // Force path-style URLs for LocalStack compatibility
    let s3_config = aws_sdk_s3::config::Builder::from(&config)
        .force_path_style(true)
        .build();

    Client::from_conf(s3_config)
}

fn expiration_secs(env_var_name: &str) -> Result<u64, Box<dyn std::error::Error>> {
    Ok(std::env::var(env_var_name)?.parse()?)
}

pub async fn generate_upload_url(
    client: &Client,
    bucket: &str,
    key: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let expires_in = expiration_secs("AWS_S3_UPLOAD_EXPIRATION_SECS")?;
    let presigned = client
        .put_object()
        .bucket(bucket)
        .key(key)
        .presigned(PresigningConfig::expires_in(Duration::from_secs(
            expires_in,
        ))?)
        .await?;

    Ok(presigned.uri().to_string())
}

pub async fn generate_download_url(
    client: &Client,
    bucket: &str,
    key: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let expires_in = expiration_secs("AWS_S3_FETCH_EXPIRATION_SECS")?;
    let presigned = client
        .get_object()
        .bucket(bucket)
        .key(key)
        .presigned(PresigningConfig::expires_in(Duration::from_secs(
            expires_in,
        ))?)
        .await?;

    Ok(presigned.uri().to_string())
}
