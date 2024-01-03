// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Result};
use aws_config::{meta::region::RegionProviderChain, Region, SdkConfig};
use tracing::instrument;

pub fn get_region() -> Result<RegionProviderChain> {
    let region = RegionProviderChain::first_try(Region::new(
        std::env::var("AWS_REGION").map_err(|err| anyhow!("AWS_REGION env var missing"))?,
    ))
    .or_default_provider()
    .or_else(Region::new("us-east-1"));
    Ok(region)
}

#[instrument(err)]
pub async fn get_aws_config() -> Result<SdkConfig> {
    let region = get_region()?;
    Ok(aws_config::from_env().region(region).load().await)
}

pub fn get_max_upload_size() -> Result<usize> {
    Ok(std::env::var("AWS_S3_MAX_UPLOAD_BYTES")
        .map_err(|err| anyhow!("AWS_S3_MAX_UPLOAD_BYTES env var missing"))?
        .parse()?)
}

pub fn get_upload_expiration_secs() -> Result<u64> {
    Ok(std::env::var("AWS_S3_UPLOAD_EXPIRATION_SECS")
        .map_err(|err| anyhow!("AWS_S3_UPLOAD_EXPIRATION_SECS env var missing"))?
        .parse()?)
}

pub fn get_fetch_expiration_secs() -> Result<u64> {
    Ok(std::env::var("AWS_S3_FETCH_EXPIRATION_SECS")
        .map_err(|err| anyhow!("AWS_S3_FETCH_EXPIRATION_SECS env var missing"))?
        .parse()?)
}
