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
use aws_sdk_s3::operation::create_multipart_upload::CreateMultipartUploadOutput;
use aws_sdk_s3::types::{
    CompletedMultipartUpload, CompletedPart, Delete, ObjectIdentifier,
};
use aws_smithy_types::byte_stream::{ByteStream, Length};
use core::time::Duration;
use s3::presigning::PresigningConfig;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::{env, error::Error};
use tempfile::NamedTempFile;
use tokio::io::AsyncReadExt;
use tracing::{info, instrument};

const MAX_CHUNK_SIZE: u64 = 16 * 1024 * 1024;

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

#[instrument(err, skip_all)]
pub async fn upload_file_to_s3(
    key: String,
    is_public: bool,
    s3_bucket: String,
    media_type: String,
    file_path: String,
    cache_control: Option<String>,
    download_filename: Option<String>,
) -> Result<()> {
    let path = Path::new(&file_path);
    let file_size = tokio::fs::metadata(path)
        .await
        .map_err(|e| anyhow!("Error getting file metadata: {e:?}"))?
        .len();
    info!("Uploading file of size {file_size} bytes to S3");

    if file_size > MAX_CHUNK_SIZE {
        upload_multipart_data_to_s3(
            path,
            key,
            is_public,
            s3_bucket,
            media_type,
            cache_control,
            download_filename,
            file_size,
        )
        .await
    } else {
        let data =
            ByteStream::from_path(&file_path).await.with_context(|| {
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
}

#[instrument(err, skip_all)]
pub async fn upload_multipart_data_to_s3(
    path: &Path,
    key: String,
    is_public: bool,
    s3_bucket: String,
    media_type: String,
    cache_control: Option<String>,
    download_filename: Option<String>,
    file_size: u64,
) -> Result<()> {
    let mut chunk_count = (file_size / MAX_CHUNK_SIZE) + 1;
    let mut size_of_last_chunk = file_size % MAX_CHUNK_SIZE;
    if size_of_last_chunk == 0 {
        size_of_last_chunk = MAX_CHUNK_SIZE;
        chunk_count -= 1;
    }

    let config = get_s3_aws_config(!is_public)
        .await
        .with_context(|| "Error getting s3 aws config")?;
    let client = get_s3_client(config.clone())
        .await
        .with_context(|| "Error getting s3 client")?;

    let mut multipart_builder = client
        .create_multipart_upload()
        .bucket(&s3_bucket)
        .key(&key)
        .content_type(media_type);

    if let Some(filename) = download_filename {
        let disposition = format!("attachment; filename=\"{filename}\"");
        multipart_builder = multipart_builder.content_disposition(disposition);
    }

    let multipart_builder = if let Some(cache_control_value) = cache_control {
        multipart_builder.cache_control(cache_control_value)
    } else {
        multipart_builder
    };

    // First we need to get the id to send it with each part.
    let multipart_upload_res: CreateMultipartUploadOutput = multipart_builder
        .send()
        .await
        .map_err(|e| anyhow!("Error uploading file to S3: {e:?}"))?;

    let upload_id = multipart_upload_res
        .upload_id()
        .ok_or(anyhow!("Missing upload_id after CreateMultipartUpload",))?;

    let mut upload_parts: Vec<aws_sdk_s3::types::CompletedPart> = Vec::new();
    for chunk_index in 0..chunk_count {
        info!("chunk {}", chunk_index);
        let this_chunk = if chunk_index == chunk_count - 1 {
            size_of_last_chunk
        } else {
            MAX_CHUNK_SIZE
        };
        let stream = ByteStream::read_from()
            .path(path)
            .offset(chunk_index * MAX_CHUNK_SIZE)
            .length(Length::Exact(this_chunk))
            .build()
            .await
            .unwrap();

        // Chunk index needs to start at 0, but part numbers start at 1.
        let part_number = (chunk_index as i32) + 1;
        let upload_part_res = client
            .upload_part()
            .key(&key)
            .bucket(&s3_bucket)
            .upload_id(upload_id)
            .body(stream)
            .part_number(part_number)
            .send()
            .await?;

        upload_parts.push(
            CompletedPart::builder()
                .e_tag(upload_part_res.e_tag.unwrap_or_default())
                .part_number(part_number)
                .build(),
        );
    }

    let completed_multipart_upload: CompletedMultipartUpload =
        CompletedMultipartUpload::builder()
            .set_parts(Some(upload_parts))
            .build();

    let _complete_multipart_upload_res = client
        .complete_multipart_upload()
        .bucket(&s3_bucket)
        .key(&key)
        .multipart_upload(completed_multipart_upload)
        .upload_id(upload_id)
        .send()
        .await?;

    Ok(())
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
    info!("Config acquired");
    let client = get_s3_client(config.clone())
        .await
        .with_context(|| "Error getting s3 client")?;
    info!("S3 client acquired");

    // First, collect all keys to delete
    let mut all_keys: Vec<String> = Vec::new();
    let mut token: Option<String> = None;

    loop {
        info!("Listing objects");
        let list_output = match client
            .list_objects_v2()
            .bucket(s3_bucket.clone())
            .prefix(prefix.clone())
            .max_keys(1000)
            .set_continuation_token(token.clone())
            .send()
            .await
        {
            Ok(list) => {
                list
                // Successfully deleted
            }
            Err(err) => {
                // Check if it's a NoSuchKey error
                let err_str = format!("{:?}", err);
                if err_str.contains("NoSuchKey") {
                    info!("Key already absent in S3; continuing. {:?}", err);
                    return Ok(());
                } else {
                    // For other errors, fail the operation
                    return Err(anyhow!("{:?}", err));
                }
            }
        };

        // Collect keys from this page
        for obj in list_output.contents() {
            if let Some(key) = obj.key() {
                all_keys.push(key.to_string());
            }
        }

        if let Some(next_token) = list_output.next_continuation_token() {
            token = Some(next_token.to_string());
        } else {
            break;
        }
    }

    info!(
        "Collected {} objects to delete from S3 bucket '{}' with prefix '{}'",
        all_keys.len(),
        s3_bucket,
        prefix
    );

    // Now delete each key individually, tolerating NoSuchKey errors
    for key in &all_keys {
        match client
            .delete_object()
            .bucket(s3_bucket.clone())
            .key(key.clone())
            .send()
            .await
        {
            Ok(_) => {
                // Successfully deleted
            }
            Err(err) => {
                // Check if it's a NoSuchKey error
                let err_str = format!("{:?}", err);
                if err_str.contains("NoSuchKey") {
                    tracing::warn!(
                        key = %key,
                        "Key already absent in S3; continuing"
                    );
                } else {
                    // For other errors, fail the operation
                    return Err(anyhow::Error::from(err).context(format!(
                        "Failed to delete S3 object: {}",
                        key
                    )));
                }
            }
        }
    }

    info!(
        "Successfully processed deletion of {} objects from S3",
        all_keys.len()
    );

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
