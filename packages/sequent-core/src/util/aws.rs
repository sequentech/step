// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Result};
use aws_config::{meta::region::RegionProviderChain, Region, SdkConfig};
use tracing::{info, instrument};

pub fn get_region() -> Result<RegionProviderChain> {
    let region = RegionProviderChain::first_try(Region::new(
        std::env::var("AWS_REGION")
            .map_err(|err| anyhow!("AWS_REGION env var missing: {err}"))?,
    ))
    .or_default_provider()
    .or_else(Region::new("us-east-1"));
    Ok(region)
}

#[instrument(err, skip_all)]
pub async fn get_from_env_aws_config() -> Result<SdkConfig> {
    let region = Region::new(
        std::env::var("AWS_REGION")
            .map_err(|err| anyhow!("AWS_REGION env var missing: {err}"))?,
    );
    Ok(aws_config::from_env().region(region).load().await)
}

#[instrument(err)]
pub async fn get_s3_aws_config(is_private: bool) -> Result<aws_sdk_s3::Config> {
    let sdk_config = get_from_env_aws_config().await?;
    let env_var_name = if is_private {
        "AWS_S3_PRIVATE_URI"
    } else {
        "AWS_S3_PUBLIC_URI"
    };
    let access_key_result = std::env::var("AWS_S3_ACCESS_KEY");
    let access_secret_result = std::env::var("AWS_S3_ACCESS_SECRET");
    let endpoint_uri = std::env::var(env_var_name)?;
    info!("env_var_name={env_var_name}, endpoint_uri = {endpoint_uri:?}");

    if let (Ok(access_key), Ok(access_secret)) =
        (access_key_result, access_secret_result)
    {
        if (!access_key.is_empty() && !access_secret.is_empty()) {
            info!("using provided aws access key and secret credentials");

            let credentials_provider = aws_sdk_s3::config::Credentials::new(
                access_key,
                access_secret,
                None,
                None,
                "loaded-from-custom-env",
            );

            return Ok(aws_sdk_s3::config::Builder::from(&sdk_config)
                .endpoint_url(endpoint_uri)
                .credentials_provider(credentials_provider)
                .force_path_style(true) // apply bucketname as path param instead of pre-domain
                .build());
        }
        // Very important: fall-through to auto detecting credentials
        // from the execution environment if the environment variables
        // were present, but empty.
    }

    info!("using default aws sdk config credentials");
    Ok(aws_sdk_s3::config::Builder::from(&sdk_config)
        .endpoint_url(endpoint_uri)
        .force_path_style(true) // apply bucketname as path param instead of pre-domain
        .build())
}

pub fn get_max_upload_size() -> Result<usize> {
    Ok(std::env::var("AWS_S3_MAX_UPLOAD_BYTES")
        .map_err(|err| {
            anyhow!("AWS_S3_MAX_UPLOAD_BYTES env var missing: {err}")
        })?
        .parse()?)
}

pub fn get_upload_expiration_secs() -> Result<u64> {
    Ok(std::env::var("AWS_S3_UPLOAD_EXPIRATION_SECS")
        .map_err(|err| {
            anyhow!("AWS_S3_UPLOAD_EXPIRATION_SECS env var missing: {err}")
        })?
        .parse()?)
}

pub fn get_fetch_expiration_secs() -> Result<u64> {
    Ok(std::env::var("AWS_S3_FETCH_EXPIRATION_SECS")
        .map_err(|err| {
            anyhow!("AWS_S3_FETCH_EXPIRATION_SECS env var missing: {err}")
        })?
        .parse()?)
}
