// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::util::aws::{
    get_aws_config, get_fetch_expiration_secs, get_max_upload_size, get_upload_expiration_secs,
};

use anyhow::{anyhow, Result};
use aws_config::SdkConfig;
use aws_sdk_s3 as s3;
use aws_smithy_types::byte_stream::ByteStream;
use core::time::Duration;
use s3::presigning::PresigningConfig;
use std::env;
use tracing::{info, instrument};

pub fn get_private_bucket() -> Result<String> {
    let s3_bucket =
        env::var("AWS_S3_BUCKET").map_err(|err| anyhow!("AWS_S3_BUCKET must be set"))?;
    Ok(s3_bucket)
}

pub fn get_public_bucket() -> Result<String> {
    let s3_bucket = env::var("AWS_S3_PUBLIC_BUCKET")
        .map_err(|err| anyhow!("AWS_S3_PUBLIC_BUCKET must be set"))?;
    Ok(s3_bucket)
}

#[instrument(skip(client))]
async fn create_bucket_if_not_exists(
    client: &s3::Client,
    config: &SdkConfig,
    bucket_name: &str,
) -> Result<()> {
    let region = config
        .region()
        .ok_or(anyhow!("Error getting region"))?
        .to_string();
    // Check if the bucket exists
    if client
        .head_bucket()
        .bucket(bucket_name)
        .send()
        .await
        .is_err()
    {
        info!("Bucket {bucket_name} doesn't exist - creating it");
        client
            .create_bucket()
            .create_bucket_configuration(
                s3::types::CreateBucketConfiguration::builder()
                    .location_constraint(s3::types::BucketLocationConstraint::from(region.as_str()))
                    .build(),
            )
            .bucket(bucket_name)
            .send()
            .await?;
        println!("Bucket {} created", bucket_name);
    }
    Ok(())
}

pub async fn get_s3_client(config: &SdkConfig) -> Result<s3::Client> {
    let client = s3::Client::new(config);
    Ok(client)
}

#[instrument(skip(data), err)]
pub async fn upload_to_s3(
    data: &Vec<u8>,
    key: String,
    media_type: String,
    s3_bucket: String,
) -> Result<()> {
    if data.len() > get_max_upload_size()? {
        return Err(anyhow!(
            "File is too big: data.len() [{}] > get_max_upload_size() [{}]",
            data.len(),
            get_max_upload_size()?
        ));
    }

    let config = get_aws_config().await?;
    let client = get_s3_client(&config).await?;
    create_bucket_if_not_exists(&client, &config, s3_bucket.as_str()).await?;
    client
        .put_object()
        .bucket(s3_bucket)
        .key(key)
        .content_type(media_type)
        .body(ByteStream::from(data.to_vec()))
        .send()
        .await?;

    Ok(())
}

#[instrument]
pub fn get_document_key(
    tenant_id: String,
    election_event_id: String,
    document_id: String,
) -> String {
    format!(
        "tenant-{}/event-{}/document-{}",
        tenant_id, election_event_id, document_id
    )
}

#[instrument]
pub fn get_public_document_key(tenant_id: String, document_id: String, name: String) -> String {
    format!("tenant-{}/document-{}/{}", tenant_id, document_id, name)
}

#[instrument(err)]
pub async fn get_document_url(key: String, s3_bucket: String) -> Result<String> {
    let config = get_aws_config().await?;
    let client = get_s3_client(&config).await?;

    let presigning_config =
        PresigningConfig::expires_in(Duration::from_secs(get_fetch_expiration_secs()?))?;

    let presigned_request = client
        .get_object()
        .bucket(&s3_bucket)
        .key(&key)
        .presigned(presigning_config)
        .await?;

    Ok(presigned_request.uri().to_string())
}

#[instrument(err)]
pub async fn get_upload_url(key: String) -> Result<String> {
    let s3_bucket = get_public_bucket()?;
    let config = get_aws_config().await?;
    let client = get_s3_client(&config).await?;
    create_bucket_if_not_exists(&client, &config, s3_bucket.as_str()).await?;

    let presigning_config =
        PresigningConfig::expires_in(Duration::from_secs(get_upload_expiration_secs()?))?;

    let presigned_request = client
        .put_object()
        .bucket(&s3_bucket)
        .key(&key)
        .presigned(presigning_config)
        .await?;
    Ok(presigned_request.uri().to_string())
}
