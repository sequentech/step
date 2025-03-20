// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::util::aws::{
    get_fetch_expiration_secs, get_s3_aws_config, get_upload_expiration_secs,
};
use crate::util::temp_path::{
    generate_temp_file, get_public_assets_path_env_var,
};
use anyhow::{anyhow, Context, Result};
use aws_sdk_s3 as s3;
use aws_smithy_types::byte_stream;
use aws_smithy_types::byte_stream::ByteStream;
use core::time::Duration;
use s3::presigning::PresigningConfig;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::{env, error::Error};
use tempfile::NamedTempFile;
use tokio::io::AsyncReadExt;
use tracing::{info, instrument};

// constant defining chunk size to 16 mb
// this is the max supported by minio:
// https://github.com/minio/minio/blob/42d4ab2a0ab56bf4953e6fb77a8268d478d2df32/cmd/streaming-signature-v4.go#L260
const CHUNK_SIZE_16MB: usize = 15_784;//16 << 20;

// variant of https://github.com/smithy-lang/smithy-rs/blob/0774950eabaccec6a48fb93495ac0fc1e2054116/rust-runtime/aws-smithy-types/src/byte_stream.rs#L408
// that allows configuring the chunk size
pub async fn bytestream_from_path(
    path: impl AsRef<std::path::Path>,
    chunk_size: usize,
) -> Result<ByteStream, byte_stream::error::Error> {
    byte_stream::FsBuilder::new()
        .buffer_size(chunk_size)
        .path(path)
        .build()
        .await
}

#[instrument(err, skip_all)]
pub fn get_private_bucket() -> Result<String> {
    let s3_bucket = env::var("AWS_S3_BUCKET")
        .map_err(|err| anyhow!("AWS_S3_BUCKET must be set: {err}"))?;
    Ok(s3_bucket)
}

#[instrument(err, skip_all)]
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
                    .location_constraint(
                        s3::types::BucketLocationConstraint::from(
                            region.as_str(),
                        ),
                    )
                    .build(),
            )
            .bucket(bucket_name)
            .send()
            .await
            .with_context(|| {
                format!("Error creating bucket with name={bucket_name}")
            })?;
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
    election_event_id: Option<&str>,
    document_id: &str,
    name: &str,
) -> String {
    match election_event_id {
        Some(event_id) => {
            format!("tenant-{tenant_id}/event-{event_id}/document-{document_id}/{name}")
        }
        None => {
            format!("tenant-{tenant_id}/document-{document_id}/{name}")
        }
    }
}

#[instrument(skip_all)]
pub fn get_public_document_key(
    tenant_id: &str,
    document_id: &str,
    name: &str,
) -> String {
    format!("tenant-{}/document-{}/{}", tenant_id, document_id, name)
}

#[instrument(err)]
pub async fn get_document_url(
    key: String,
    s3_bucket: String,
) -> Result<String> {
    let config = get_s3_aws_config(/* private = */ false).await?;
    let client = get_s3_client(config).await?;

    let presigning_config = PresigningConfig::expires_in(Duration::from_secs(
        get_fetch_expiration_secs()?,
    ))?;

    let presigned_request = client
        .get_object()
        .bucket(&s3_bucket)
        .key(&key)
        .presigned(presigning_config)
        .await?;

    Ok(presigned_request.uri().to_string())
}

#[instrument(err, ret)]
pub async fn get_upload_url(
    key: String,
    is_public: bool,
    is_local: bool,
) -> Result<String> {
    let s3_bucket = match is_public {
        true => get_public_bucket()?,
        false => get_private_bucket()?,
    };
    // We always use the public aws config since we are generating a client-side
    // upload url. is_public is only used to define the upload bucket
    let config = get_s3_aws_config(/* private = */ is_local).await?;
    let client = get_s3_client(config.clone()).await?;

    let presigning_config = PresigningConfig::expires_in(Duration::from_secs(
        get_upload_expiration_secs()?,
    ))?;

    let presigned_request = client
        .put_object()
        .bucket(&s3_bucket)
        .key(&key)
        .presigned(presigning_config)
        .await?;
    Ok(presigned_request.uri().to_string())
}

#[instrument(err, skip_all)]
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
        .map_err(|err| {
            anyhow!("Error getting the object from S3: {:?}", err.source())
        })?;

    // Stream the data into a temporary file
    let mut temp_file = generate_temp_file(prefix, suffix)
        .with_context(|| "Error creating temp file")?;
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

#[instrument(err)]
pub async fn upload_file_to_s3(
    key: String,
    is_public: bool,
    s3_bucket: String,
    media_type: String,
    file_path: String,
    cache_control: Option<String>,
    download_filename: Option<String>,
) -> Result<()> {
    let data = bytestream_from_path(&file_path, CHUNK_SIZE_16MB)
        .await
        .with_context(|| {
            anyhow!("Error creating bytestream from file path={file_path}")
        })?;
    upload_data_to_s3(
        data,
        key,
        is_public,
        s3_bucket,
        media_type,
        cache_control,
        download_filename,
    )
    .await
}

#[instrument(err, skip_all)]
pub async fn upload_data_to_s3(
    data: ByteStream,
    key: String,
    is_public: bool,
    s3_bucket: String,
    media_type: String,
    cache_control: Option<String>,
    download_filename: Option<String>,
) -> Result<()> {
    let config = get_s3_aws_config(!is_public)
        .await
        .with_context(|| "Error getting s3 aws config")?;
    let client = get_s3_client(config.clone())
        .await
        .with_context(|| "Error getting s3 client")?;

    let mut request = client
        .put_object()
        .bucket(s3_bucket)
        .key(key)
        .content_type(media_type)
        .body(data);

    if let Some(filename) = download_filename {
        // e.g. "attachment; filename=\"myfile.ezip\""
        let disposition = format!("attachment; filename=\"{filename}\"");
        request = request.content_disposition(disposition);
    }

    let request = if let Some(cache_control_value) = cache_control {
        request.cache_control(cache_control_value)
    } else {
        request
    };

    request.send().await.context("Error uploading file to S3")?;

    Ok(())
}

pub fn get_minio_url() -> Result<String> {
    let minio_private_uri = env::var("AWS_S3_PRIVATE_URI")
        .map_err(|err| anyhow!("AWS_S3_PRIVATE_URI must be set"))?;
    let bucket = get_public_bucket()?;

    Ok(format!("{}/{}", minio_private_uri, bucket))
}

pub fn get_minio_public_url() -> Result<String> {
    let minio_public_uri = env::var("AWS_S3_PUBLIC_URI")
        .map_err(|err| anyhow!("AWS_S3_PUBLIC_URI must be set"))?;
    let bucket = get_public_bucket()?;

    Ok(format!("{}/{}", minio_public_uri, bucket))
}

pub fn get_public_asset_file_path(filename: &str) -> Result<String> {
    let minio_endpoint_base =
        get_minio_url().with_context(|| "Error fetching get_minio_url")?;
    let public_asset_path = get_public_assets_path_env_var()?;

    Ok(format!(
        "{}/{}/{}",
        minio_endpoint_base, public_asset_path, filename
    ))
}

#[instrument(err)]
pub async fn download_s3_file_to_string(file_url: &str) -> Result<String> {
    let client = reqwest::Client::new();

    info!("Requesting HTTP GET {:?}", file_url);
    let response = client.get(file_url).send().await?;

    let unwrapped_response = if response.status() != reqwest::StatusCode::OK {
        return Err(anyhow!(
            "Error during download_s3_file_to_string: {:?}",
            response
        ));
    } else {
        response
    };
    let bytes = unwrapped_response.bytes().await?;
    Ok(String::from_utf8(bytes.to_vec())?)
}

#[instrument(err, ret)]
pub async fn delete_files_from_s3(
    s3_bucket: String,
    prefix: String,
    is_public: bool,
) -> Result<()> {
    let config = get_s3_aws_config(!is_public)
        .await
        .with_context(|| "Error getting s3 aws config")?;
    let client = get_s3_client(config.clone())
        .await
        .with_context(|| "Error getting s3 client")?;

    let mut token: Option<String> = None;
    loop {
        let result = client
            .list_objects_v2()
            .bucket(s3_bucket.clone())
            .prefix(prefix.clone())
            .max_keys(20)
            .set_continuation_token(token.clone())
            .send()
            .await?;

        for object in result.contents().iter() {
            let key = object.key().unwrap();

            client
                .delete_object()
                .bucket(s3_bucket.clone())
                .key(key)
                .send()
                .await?;
        }

        if let Some(next_token) = result.next_continuation_token() {
            token = Some(next_token.to_string());
        } else {
            break;
        }
    }

    Ok(())
}

#[instrument(err)]
pub async fn get_file_from_s3(
    s3_bucket: String,
    path: String,
) -> Result<Vec<u8>> {
    let config = get_s3_aws_config(true)
        .await
        .with_context(|| "Error getting s3 aws config")?;
    let client = get_s3_client(config.clone())
        .await
        .with_context(|| "Error getting s3 client")?;

    let mut object = client
        .get_object()
        .bucket(s3_bucket.clone())
        .key(path)
        .send()
        .await?;

    let mut result: Vec<u8> = Vec::new();
    while let Some(bytes) = object.body.try_next().await.map_err(|err| {
        anyhow!("Failed to read from S3 download stream: {err:?}")
    })? {
        result.extend(&bytes);
    }

    Ok(result)
}

#[instrument(err)]
pub async fn get_files_from_s3(
    s3_bucket: String,
    prefix: String,
) -> Result<Vec<PathBuf>> {
    let config = get_s3_aws_config(true)
        .await
        .with_context(|| "Error getting s3 aws config")?;
    let client = get_s3_client(config.clone())
        .await
        .with_context(|| "Error getting s3 client")?;

    let mut file_paths = Vec::new();

    let result = client
        .list_objects_v2()
        .bucket(s3_bucket.clone())
        .prefix(prefix.clone())
        .send()
        .await?;

    for object in result.contents().iter() {
        let key = object.key().unwrap();

        if !key.contains("export") {
            // Get the object from S3
            let s3_object = client
                .get_object()
                .bucket(s3_bucket.clone())
                .key(key)
                .send()
                .await?;

            let stream = s3_object.body;
            let file_data = ByteStream::collect(stream).await?.into_bytes();

            // Create a temporary file to store the downloaded S3 file
            let file_name = key.split('/').last().unwrap();
            let file_path = Path::new(&env::temp_dir()).join(file_name);
            let mut temp_file = File::create(&file_path)?;

            temp_file.write_all(&file_data)?;
            file_paths.push(file_path);
        }
    }

    Ok(file_paths)
}
