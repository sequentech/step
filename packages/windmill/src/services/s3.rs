// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::temp_path::generate_temp_file;
use crate::util::aws::{
    get_fetch_expiration_secs, get_max_upload_size, get_s3_aws_config, get_upload_expiration_secs,
};
use anyhow::{anyhow, Context, Result};
use aws_sdk_s3 as s3;
use aws_smithy_types::byte_stream::ByteStream;
use core::time::Duration;
use s3::presigning::PresigningConfig;
use std::fmt;
use std::fs::File;
use std::io::Write;
use std::{env, error::Error};
use tempfile::{tempfile, NamedTempFile};
use tokio::io::AsyncReadExt;
use tracing::{info, instrument};

//This can be enhanced to more kinds of cache-policies
#[derive(Debug)]
pub enum CacheControlOptions {
    MaxAge(u32),
}

impl fmt::Display for CacheControlOptions {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CacheControlOptions::MaxAge(seconds) => write!(f, "max-age={}", seconds),
        }
    }
}

#[instrument(err, ret)]
pub fn get_private_bucket() -> Result<String> {
    let s3_bucket =
        env::var("AWS_S3_BUCKET").map_err(|err| anyhow!("AWS_S3_BUCKET must be set: {err}"))?;
    Ok(s3_bucket)
}

pub fn get_public_bucket() -> Result<String> {
    let s3_bucket = env::var("AWS_S3_PUBLIC_BUCKET")
        .map_err(|err| anyhow!("AWS_S3_PUBLIC_BUCKET must be set: {err}"))?;
    Ok(s3_bucket)
}

#[instrument(skip(client, config))]
async fn create_bucket_if_not_exists(
    client: &s3::Client,
    config: &s3::Config,
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
            .await
            .with_context(|| format!("Error creating bucket with name={bucket_name}"))?;
        println!("Bucket {} created", bucket_name);
    }
    Ok(())
}

pub async fn get_s3_client(config: s3::Config) -> Result<s3::Client> {
    let client = s3::Client::from_conf(config);
    Ok(client)
}

#[instrument]
pub fn get_document_key(
    tenant_id: &str,
    election_event_id: &str,
    document_id: &str,
    name: &str,
) -> String {
    format!("tenant-{tenant_id}/event-{election_event_id}/document-{document_id}/{name}")
}

#[instrument]
pub fn get_public_document_key(tenant_id: &str, document_id: &str, name: &str) -> String {
    format!("tenant-{}/document-{}/{}", tenant_id, document_id, name)
}

#[instrument(err)]
pub async fn get_document_url(key: String, s3_bucket: String) -> Result<String> {
    let config = get_s3_aws_config(/* private = */ false).await?;
    let client = get_s3_client(config).await?;

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

#[instrument(err, ret)]
pub async fn get_upload_url(key: String, is_public: bool) -> Result<String> {
    let s3_bucket = match is_public {
        true => get_public_bucket()?,
        false => get_private_bucket()?,
    };
    // We always use the public aws config since we are generating a client-side
    // upload url. is_public is only used to define the upload bucket
    let config = get_s3_aws_config(/* private = */ false).await?;
    let client = get_s3_client(config.clone()).await?;

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

#[instrument(err, ret)]
pub async fn get_object_into_temp_file(
    s3_bucket: &str,
    key: &str,
    prefix: &str,
    suffix: &str,
) -> anyhow::Result<NamedTempFile> {
    let config = get_s3_aws_config(/* private = */ true)
        .await
        .with_context(|| "Error obtaining aws config")?;
    let client = get_s3_client(config.clone()).await?;

    let response = client
        .get_object()
        .bucket(s3_bucket)
        .key(key)
        .send()
        .await
        .map_err(|err| anyhow!("Error getting the object from S3: {:?}", err.source()))?;

    // Stream the data into a temporary file
    let mut temp_file =
        generate_temp_file(prefix, suffix).with_context(|| "Error creating temp file")?;
    let mut stream = response.body.into_async_read();
    let mut buffer = [0u8; 1024]; // Adjust buffer size as needed

    while let Ok(size) = stream.read(&mut buffer).await {
        if size == 0 {
            break; // End of file
        }
        temp_file
            .write_all(&buffer[..size])
            .with_context(|| "Error writting to the text file")?;
    }

    // The file is now downloaded to a temporary file
    Ok(temp_file)
}

#[instrument(err, ret)]
pub async fn upload_file_to_s3(
    key: String,
    is_public: bool,
    s3_bucket: String,
    media_type: String,
    file_path: String,
    cache_control: Option<CacheControlOptions>,
) -> Result<()> {
    let body = ByteStream::from_path(&file_path)
        .await
        .with_context(|| anyhow!("Error creating bytestream from file path={file_path}"))?;
    let config = get_s3_aws_config(!is_public)
        .await
        .with_context(|| "Error getting s3 aws config")?;
    let client = get_s3_client(config.clone())
        .await
        .with_context(|| "Error getting s3 client")?;

    let request = client
        .put_object()
        .bucket(s3_bucket)
        .key(key)
        .content_type(media_type)
        .body(body);

    let request = if let Some(cache_control_value) = cache_control {
        request.cache_control(cache_control_value.to_string())
    } else {
        request
    };

    request.send().await.context("Error uploading file to S3")?;

    Ok(())
}

pub fn get_minio_url() -> Result<String> {
    let minio_private_uri =
        env::var("AWS_S3_PRIVATE_URI").map_err(|err| anyhow!("AWS_S3_PRIVATE_URI must be set"))?;
    let bucket = get_public_bucket()?;

    Ok(format!("{}/{}", minio_private_uri, bucket))
}
