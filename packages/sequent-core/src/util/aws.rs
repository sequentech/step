// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Result};
use aws_config::{meta::region::RegionProviderChain, Region, SdkConfig};
use tracing::{info, instrument};

pub const AWS_S3_PRIVATE_URI_ENV: &str = "AWS_S3_PRIVATE_URI";
pub const AWS_S3_PUBLIC_URI_ENV: &str = "AWS_S3_PUBLIC_URI";

/// Resolves the AWS region from the environment and keeps the default chain
/// as a fallback so local and deployed runtimes share the same lookup flow.
#[instrument(err, skip_all)]
pub fn get_region() -> Result<RegionProviderChain> {
    let region = RegionProviderChain::first_try(Region::new(
        std::env::var("AWS_REGION")
            .map_err(|err| anyhow!("AWS_REGION env var missing: {err}"))?,
    ))
    .or_default_provider()
    .or_else(Region::new("us-east-1"));
    Ok(region)
}

/// Loads the shared AWS SDK configuration from the process environment so S3,
/// SES, SNS, and STS all use the same credentials and region resolution.
#[instrument(err, skip_all)]
pub async fn get_from_env_aws_config() -> Result<SdkConfig> {
    let region = Region::new(
        std::env::var("AWS_REGION")
            .map_err(|err| anyhow!("AWS_REGION env var missing: {err}"))?,
    );
    Ok(aws_config::from_env().region(region).load().await)
}

/// Builds an S3 client configuration for an explicit endpoint URL while
/// preserving this module's credential-loading rules.
///
/// Use this helper when the caller already resolved the final endpoint URI.
/// Use [`get_s3_aws_config`] when endpoint selection should be derived from
/// `AWS_S3_PRIVATE_URI`/`AWS_S3_PUBLIC_URI`.
///
/// - `sdk_config`: shared AWS SDK config loaded from environment/default chain.
/// - `endpoint_uri`: absolute S3-compatible endpoint URL to target.
///
/// Returns the final S3 client config with endpoint, path-style behavior, and
/// optional explicit credentials from
/// `AWS_S3_ACCESS_KEY`/`AWS_S3_ACCESS_SECRET`.
#[instrument(skip_all)]
pub(crate) fn build_s3_aws_config_for_endpoint(
    sdk_config: &SdkConfig,
    endpoint_uri: &str,
) -> aws_sdk_s3::Config {
    let access_key_result = std::env::var("AWS_S3_ACCESS_KEY");
    let access_secret_result = std::env::var("AWS_S3_ACCESS_SECRET");
    let mut builder = aws_sdk_s3::config::Builder::from(sdk_config)
        .endpoint_url(endpoint_uri)
        .force_path_style(true); // apply bucketname as path param instead of pre-domain
    let mut using_custom_credentials = false;

    if let (Ok(access_key), Ok(access_secret)) =
        (access_key_result, access_secret_result)
    {
        if !access_key.is_empty() && !access_secret.is_empty() {
            info!("using provided aws access key and secret credentials");
            using_custom_credentials = true;

            let credentials_provider = aws_sdk_s3::config::Credentials::new(
                access_key,
                access_secret,
                None,
                None,
                "loaded-from-custom-env",
            );
            builder = builder.credentials_provider(credentials_provider);
        }
        // Very important: fall-through to auto detecting credentials
        // from the execution environment if the environment variables
        // were present, but empty.
    }

    if !using_custom_credentials {
        info!("using default aws sdk config credentials");
    }

    builder.build()
}

/// Builds an S3 client configuration for the selected endpoint. <br>
/// When `use_server_endpoint` is `false`, the client-facing endpoint is used
/// instead of the server-side endpoint.
#[instrument(err, skip_all)]
pub async fn get_s3_aws_config(
    use_server_endpoint: bool,
) -> Result<aws_sdk_s3::Config> {
    let sdk_config = get_from_env_aws_config().await?;
    let env_var_name = if use_server_endpoint {
        AWS_S3_PRIVATE_URI_ENV
    } else {
        AWS_S3_PUBLIC_URI_ENV
    };
    let endpoint_uri = std::env::var(env_var_name)?;
    info!("env_var_name={env_var_name}, endpoint_uri = {endpoint_uri:?}");

    Ok(build_s3_aws_config_for_endpoint(&sdk_config, &endpoint_uri))
}

/// Returns the maximum upload size so callers can reject oversized payloads
/// before opening a long-running upload flow.
#[instrument(err, skip_all)]
pub fn get_max_upload_size() -> Result<usize> {
    Ok(std::env::var("AWS_S3_MAX_UPLOAD_BYTES")
        .map_err(|err| {
            anyhow!("AWS_S3_MAX_UPLOAD_BYTES env var missing: {err}")
        })?
        .parse()?)
}

/// Returns the upload URL lifetime so presigned uploads expire predictably.
#[instrument(err, skip_all)]
pub fn get_upload_expiration_secs() -> Result<u64> {
    Ok(std::env::var("AWS_S3_UPLOAD_EXPIRATION_SECS")
        .map_err(|err| {
            anyhow!("AWS_S3_UPLOAD_EXPIRATION_SECS env var missing: {err}")
        })?
        .parse()?)
}

/// Returns the download URL lifetime so generated fetch URLs match the
/// deployment's cache and access expectations.
#[instrument(err, skip_all)]
pub fn get_fetch_expiration_secs() -> Result<u64> {
    Ok(std::env::var("AWS_S3_FETCH_EXPIRATION_SECS")
        .map_err(|err| {
            anyhow!("AWS_S3_FETCH_EXPIRATION_SECS env var missing: {err}")
        })?
        .parse()?)
}
