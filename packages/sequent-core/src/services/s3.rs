// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::util::aws::{
    build_s3_aws_config_for_endpoint, get_fetch_expiration_secs,
    get_from_env_aws_config, get_s3_aws_config, get_upload_expiration_secs,
    AWS_S3_PRIVATE_URI_ENV, AWS_S3_PUBLIC_URI_ENV,
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
use aws_smithy_types::error::metadata::ProvideErrorMetadata;
use core::time::Duration;
use s3::presigning::PresigningConfig;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::{env, error::Error};
use strum_macros::{Display, EnumString};
use tempfile::{NamedTempFile, TempPath};
use tokio::io::{self, AsyncReadExt};
use tracing::{info, instrument, warn};

const MAX_CHUNK_SIZE: u64 = 16 * 1024 * 1024;
const AWS_HOSTED_S3_HOST_DELIMITER: &str = ".s3.";
const AWS_HOSTED_S3_DOMAIN_SUFFIX: &str = "amazonaws.com";
const AWS_S3_SERVICE_HOST_PREFIX: &str = "s3";
const S3_LIST_MAX_KEYS: i32 = 1000;
const S3_ERR_NO_DETAILS: &str = "no additional details available";
const AWS_S3_ENDPOINT_STYLE_ENV: &str = "AWS_S3_ENDPOINT_STYLE";

/// Selects how the configured S3 endpoint addresses buckets. Any
/// S3-compatible provider can be supported without provider-specific code by
/// setting `AWS_S3_ENDPOINT_STYLE` to the style its endpoint uses.
#[derive(Clone, Copy, Debug, Default, Display, EnumString, Eq, PartialEq)]
enum S3EndpointStyle {
    /// The bucket name is part of the URL path, e.g. MinIO
    /// (`http://minio:9000/<bucket>/...`). This is the default: it also
    /// covers AWS's bucket-hosted endpoints, which are recognized
    /// automatically for backwards compatibility with existing zero-config
    /// AWS deployments.
    #[default]
    #[strum(serialize = "path-style-or-auto-detected-aws")]
    PathStyleOrAutoDetectedAws,
    /// The bucket name is embedded in the hostname as its first label, e.g.
    /// `<bucket>.<service-host>`. Any provider that addresses buckets this
    /// way (virtual-hosted-style) can opt in via
    /// `AWS_S3_ENDPOINT_STYLE=virtual-hosted`, regardless of which provider
    /// it is.
    #[strum(serialize = "virtual-hosted")]
    VirtualHosted,
}

/// Reads the optional endpoint style override from the environment, so
/// callers don't need to know which provider is configured.
fn get_s3_endpoint_style() -> Result<S3EndpointStyle> {
    match env::var(AWS_S3_ENDPOINT_STYLE_ENV) {
        Ok(value) => value.parse().with_context(|| {
            format!(
                "Invalid {AWS_S3_ENDPOINT_STYLE_ENV} value `{value}`, the only \
                 accepted value is `virtual-hosted`; omit the variable entirely \
                 for the default path-style/AWS-auto-detect behavior"
            )
        }),
        Err(env::VarError::NotPresent) => Ok(S3EndpointStyle::default()),
        Err(err) => Err(anyhow!(
            "{AWS_S3_ENDPOINT_STYLE_ENV} is set but not valid UTF-8: {err}"
        )),
    }
}

#[derive(Debug, PartialEq, Eq)]
struct ResolvedS3ListTargetParts {
    service_endpoint: Option<String>,
    bucket: String,
    prefix_root: Option<String>,
}

/// Carries the resolved S3 client, real bucket name, and optional logical
/// prefix root for list-style operations that must work on both MinIO and AWS.
struct ResolvedS3ListTarget {
    client: s3::Client,
    bucket: String,
    prefix_root: Option<String>,
}

impl ResolvedS3ListTarget {
    /// Adds the resolved logical prefix root so callers can request the same
    /// effective key space regardless of the underlying endpoint shape.<br>
    /// I.e. AWS prefix_root is "public/" or "election-event-documents/" (both
    /// within the same bucket)  while MinIO prefix_root is None and the
    /// bucket name encodes the scope instead.
    #[instrument(skip_all)]
    fn qualify_prefix(&self, prefix: &str) -> String {
        match &self.prefix_root {
            Some(prefix_root) => join_s3_path(prefix_root, prefix),
            None => prefix.to_string(),
        }
    }
}

/// Joins S3 path fragments while normalizing slashes so generated prefixes stay
/// stable across callers.
#[instrument(skip_all)]
fn join_s3_path(prefix: &str, suffix: &str) -> String {
    let prefix = prefix.trim_matches('/');
    let suffix = suffix.trim_matches('/');

    match (prefix.is_empty(), suffix.is_empty()) {
        (true, true) => String::new(),
        (true, false) => suffix.to_string(),
        (false, true) => prefix.to_string(),
        (false, false) => format!("{prefix}/{suffix}"),
    }
}

/// Detects bucket-hosted (virtual-hosted-style) endpoints — where the bucket
/// name is embedded in the hostname rather than the URL path — and extracts
/// the real service endpoint plus bucket name so list operations can address
/// the endpoint correctly.<br>
/// With `S3EndpointStyle::PathStyleOrAutoDetectedAws`, only AWS's
/// bucket-hosted shape (`<bucket>.s3.[<region>.]amazonaws.com`) is
/// recognized, for backwards compatibility with existing zero-config AWS
/// deployments; anything else is assumed to already be path-style (e.g.
/// MinIO) and is left untouched. With `S3EndpointStyle::VirtualHosted`, any
/// hostname is treated as bucket-hosted with the bucket name as its first
/// label — this covers every S3-compatible provider that addresses buckets
/// this way, without needing to know which provider it is. Bucket names
/// containing dots aren't supported in this mode, since there is no
/// generic way to tell a dotted bucket name apart from a subdomain of the
/// service host without hardcoding a specific provider's shape.
#[instrument(err, skip_all)]
fn parse_bucket_hosted_endpoint(
    endpoint_uri: &str,
    aws_region: Option<&str>,
    endpoint_style: S3EndpointStyle,
) -> Result<Option<(String, String)>> {
    // Parse once so we can reason about the hostname shape without doing any
    // string slicing against the raw env var value.
    let url = reqwest::Url::parse(endpoint_uri)
        .with_context(|| format!("Invalid S3 endpoint URL `{endpoint_uri}`"))?;
    let host = match url.host_str() {
        Some(host) => host,
        None => return Ok(None),
    };

    let (bucket_name, service_host) = match endpoint_style {
        // Virtual-hosted-style addressing is a DNS-hostname convention; a
        // bare IP literal has no bucket label to extract. IPv6 literals are
        // bracketed in `host_str()` (e.g. `[::1]`), so strip that before
        // checking.
        S3EndpointStyle::VirtualHosted
            if host
                .trim_start_matches('[')
                .trim_end_matches(']')
                .parse::<std::net::IpAddr>()
                .is_ok() =>
        {
            return Ok(None);
        }
        S3EndpointStyle::VirtualHosted => match host.split_once('.') {
            Some((bucket_name, service_host)) if !bucket_name.is_empty() => {
                (bucket_name, service_host.to_string())
            }
            _ => return Ok(None),
        },
        S3EndpointStyle::PathStyleOrAutoDetectedAws => {
            // AWS bucket-hosted endpoints look like `<bucket>.s3.amazonaws.com`
            // or `<bucket>.s3.<region>.amazonaws.com`. MinIO and other custom
            // endpoints do not match this shape, so they must be left
            // untouched.
            match host.split_once(AWS_HOSTED_S3_HOST_DELIMITER) {
                Some((bucket_name, suffix)) if !bucket_name.is_empty() => {
                    if !suffix.ends_with(AWS_HOSTED_S3_DOMAIN_SUFFIX) {
                        return Ok(None);
                    }

                    let service_host = if suffix == AWS_HOSTED_S3_DOMAIN_SUFFIX
                    {
                        // The global host form does not encode the bucket
                        // region. Prefer the resolved SDK region so SigV4
                        // targets the correct regional S3 endpoint outside
                        // us-east-1.
                        match aws_region {
                            Some(region) if !region.is_empty() => format!(
                                "{AWS_S3_SERVICE_HOST_PREFIX}.{region}.{AWS_HOSTED_S3_DOMAIN_SUFFIX}"
                            ),
                            _ => format!(
                                "{AWS_S3_SERVICE_HOST_PREFIX}.{AWS_HOSTED_S3_DOMAIN_SUFFIX}"
                            ),
                        }
                    } else {
                        // Regional bucket-hosted endpoints already tell us
                        // which service host to talk to, so we preserve that
                        // region.
                        format!("{AWS_S3_SERVICE_HOST_PREFIX}.{suffix}")
                    };

                    (bucket_name, service_host)
                }
                _ => return Ok(None),
            }
        }
    };

    // Rebuild the endpoint URL without the bucket in the hostname. The caller
    // will use the returned bucket name plus this service endpoint for list and
    // delete operations that require bucket + prefix semantics on the endpoint.
    let mut service_endpoint = format!("{}://{}", url.scheme(), service_host);
    if let Some(port) = url.port() {
        service_endpoint.push_str(&format!(":{port}"));
    }

    Ok(Some((service_endpoint, bucket_name.to_string())))
}

/// Resolves the bucket and prefix semantics for a list-style S3 call without
/// constructing a client so both runtime code and tests share the same
/// rules.<br> When the endpoint is minIO (development/codespaces) the bucket
/// name is the logical bucket, then sevice_endpoint is set to None (the raw env
/// var must be set by the caller) and prefix_root is empty (is already the
/// bucket name).
#[instrument(err, skip_all)]
fn resolve_s3_list_target_parts(
    endpoint_uri: &str,
    logical_bucket: &str,
    aws_region: Option<&str>,
    endpoint_style: S3EndpointStyle,
) -> Result<ResolvedS3ListTargetParts> {
    if let Some((service_endpoint, bucket_name)) =
        parse_bucket_hosted_endpoint(endpoint_uri, aws_region, endpoint_style)?
    {
        return Ok(ResolvedS3ListTargetParts {
            service_endpoint: Some(service_endpoint),
            bucket: bucket_name,
            prefix_root: Some(logical_bucket.trim_matches('/').to_string()),
        });
    }

    Ok(ResolvedS3ListTargetParts {
        service_endpoint: None,
        bucket: logical_bucket.to_string(),
        prefix_root: None,
    })
}

/// Resolves the client, bucket, and optional logical prefix root for a
/// server-side list operation.<br>
/// When `use_server_endpoint` is `false`, the helper uses the client endpoint
/// instead of the server endpoint.
#[instrument(err)]
async fn get_s3_list_target(
    logical_bucket: &str,
    use_server_endpoint: bool,
) -> Result<ResolvedS3ListTarget> {
    let env_var_name = if use_server_endpoint {
        AWS_S3_PRIVATE_URI_ENV
    } else {
        AWS_S3_PUBLIC_URI_ENV
    };
    let endpoint_uri = env::var(env_var_name)
        .with_context(|| format!("{env_var_name} must be set"))?;
    let sdk_config = get_from_env_aws_config().await?;
    let aws_region = sdk_config.region().map(|region| region.as_ref());
    let endpoint_style = get_s3_endpoint_style()?;
    let target_parts = resolve_s3_list_target_parts(
        &endpoint_uri,
        logical_bucket,
        aws_region,
        endpoint_style,
    )?;
    let resolved_endpoint = target_parts
        .service_endpoint
        .as_deref()
        .unwrap_or(&endpoint_uri);
    let config =
        build_s3_aws_config_for_endpoint(&sdk_config, resolved_endpoint);

    Ok(ResolvedS3ListTarget {
        client: get_s3_client(config).await?,
        bucket: target_parts.bucket,
        prefix_root: target_parts.prefix_root,
    })
}

/// Returns the logical private bucket or root prefix so callers can separate
/// storage scope from endpoint selection.
#[instrument(err, skip_all)]
pub fn get_private_bucket() -> Result<String> {
    let s3_bucket = env::var("AWS_S3_BUCKET")
        .map_err(|err| anyhow!("AWS_S3_BUCKET must be set: {err}"))?;
    Ok(s3_bucket)
}

/// Returns the logical public bucket or root prefix used for public assets and
/// plugin storage.
#[instrument(err, skip_all)]
pub fn get_public_bucket() -> Result<String> {
    let s3_bucket = env::var("AWS_S3_PUBLIC_BUCKET")
        .map_err(|err| anyhow!("AWS_S3_PUBLIC_BUCKET must be set: {err}"))?;
    Ok(s3_bucket)
}

/// Creates a bucket when running against environments that manage buckets
/// directly instead of pre-provisioning them.
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

/// Wraps S3 client construction so callers rely on one place for config to
/// client conversion.
pub async fn get_s3_client(config: s3::Config) -> Result<s3::Client> {
    let client = s3::Client::from_conf(config);
    Ok(client)
}

/// Builds the private document key layout so uploads and downloads use a
/// stable tenant and event-specific hierarchy.
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

/// Builds the public document key layout so public assets share the same naming
/// convention as private documents.
#[instrument(skip_all)]
pub fn get_public_document_key(
    tenant_id: &str,
    document_id: &str,
    name: &str,
) -> String {
    format!("tenant-{}/document-{}/{}", tenant_id, document_id, name)
}

#[instrument(skip_all)]
/// Builds the public document key for an election event.
/// Used for when the UI does not have access to the document ID.
pub fn get_public_election_event_document_name_key(
    tenant_id: &str,
    election_event_id: &str,
    name: &str,
) -> String {
    format!("tenant-{}/event-{}/{}", tenant_id, election_event_id, name)
}

/// Creates a presigned download URL for a document so clients can fetch files
/// without proxying the bytes through the backend.
#[instrument(err)]
pub async fn get_document_url(
    key: String,
    s3_bucket: String,
) -> Result<String> {
    let config = get_s3_aws_config(/* use_server_endpoint = */ false).await?;
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

/// Creates a presigned upload URL and selects the endpoint that the caller can
/// actually reach.
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
    // Select the AWS endpoint that the caller can reach: when `is_local` is
    // true we use the server-only endpoint; `is_public` only determines the
    // upload bucket.
    let config =
        get_s3_aws_config(/* use_server_endpoint = */ is_local).await?;
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

/// Downloads one object into a temporary file so downstream code can work with
/// a filesystem path instead of holding the full payload in memory.
#[instrument(err, skip_all)]
pub async fn get_object_into_temp_file(
    s3_bucket: &str,
    key: &str,
    prefix: &str,
    suffix: &str,
) -> anyhow::Result<NamedTempFile> {
    let config = get_s3_aws_config(/* use_server_endpoint = */ true)
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

/// Uploads a file path to S3 and switches to multipart uploads only when the
/// payload is large enough to need chunking.
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

/// Streams a large file through S3 multipart upload so oversized reports and
/// exports do not need to be buffered at once.
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

/// Uploads a single in-memory body to S3 for smaller files where multipart
/// upload would add unnecessary overhead.
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

/// Returns the server-side MinIO URL used by backend services when they need a
/// direct path to the public bucket.
pub fn get_minio_url() -> Result<String> {
    let minio_private_uri = env::var(AWS_S3_PRIVATE_URI_ENV)
        .map_err(|_err| anyhow!("AWS_S3_PRIVATE_URI must be set"))?;
    let bucket = get_public_bucket()?;

    Ok(format!("{}/{}", minio_private_uri, bucket))
}

/// Returns the client-facing MinIO URL used when generated links must be
/// reachable from outside the backend network.
pub fn get_minio_public_url() -> Result<String> {
    let minio_public_uri = env::var(AWS_S3_PUBLIC_URI_ENV)
        .map_err(|_err| anyhow!("AWS_S3_PUBLIC_URI must be set"))?;
    let bucket = get_public_bucket()?;

    Ok(format!("{}/{}", minio_public_uri, bucket))
}

/// Builds the URL for a public asset stored in S3 or MinIO so templates can
/// reference it directly.
pub fn get_public_asset_file_path(filename: &str) -> Result<String> {
    let minio_endpoint_base =
        get_minio_url().with_context(|| "Error fetching get_minio_url")?;
    let public_asset_path = get_public_assets_path_env_var()?;

    Ok(format!(
        "{}/{}/{}",
        minio_endpoint_base, public_asset_path, filename
    ))
}

/// Downloads a file via HTTP into a string for flows that consume public text
/// assets rather than raw S3 SDK responses.
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

/// Deletes every object under a prefix and resolves AWS bucket-hosted endpoints
/// into the real bucket plus key prefix before listing.
#[instrument(err, ret)]
pub async fn delete_files_from_s3(
    s3_bucket: String,
    prefix: String,
    is_public: bool,
) -> Result<()> {
    let resolved_target = get_s3_list_target(&s3_bucket, !is_public)
        .await
        .with_context(|| "Error getting s3 list target")?;
    info!("S3 list target acquired");
    let list_prefix = resolved_target.qualify_prefix(&prefix);
    let client = resolved_target.client;
    let bucket_name = resolved_target.bucket;

    // First, collect all keys to delete
    let mut all_keys: Vec<String> = Vec::new();
    let mut token: Option<String> = None;

    loop {
        info!("Listing objects");
        let list_output = match client
            .list_objects_v2()
            .bucket(bucket_name.clone())
            .prefix(list_prefix.clone())
            .max_keys(S3_LIST_MAX_KEYS)
            .set_continuation_token(token.clone())
            .send()
            .await
        {
            Ok(list) => list,
            Err(err) => {
                let code = err.code();
                if let Some(c) = code {
                    warn!(code = c, "S3 list_objects_v2 returned error code");
                }
                return Err(anyhow!(
                    "Error \"{}\" when listing objects for deletion in bucket '{bucket_name}' with prefix '{list_prefix}': {}",
                    code.unwrap_or(""),
                    err.message().unwrap_or(S3_ERR_NO_DETAILS)
                ));
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
        bucket_name,
        list_prefix
    );

    // Now delete each key individually, tolerating NoSuchKey errors
    for key in &all_keys {
        match client
            .delete_object()
            .bucket(bucket_name.clone())
            .key(key.clone())
            .send()
            .await
        {
            Ok(_) => {
                // Successfully deleted
            }
            Err(err) => {
                let code = err.code();
                if let Some(c) = code {
                    warn!(code = c, "S3 delete_object returned error code");
                }
                return Err(anyhow!(
                    "Error '{}' when deleting object key '{key}' in bucket '{bucket_name}' with prefix '{list_prefix}': {}",
                    code.unwrap_or(""),
                    err.message().unwrap_or(S3_ERR_NO_DETAILS)
                ));
            }
        }
    }

    info!(
        "Successfully processed deletion of {} objects from S3",
        all_keys.len()
    );

    Ok(())
}

/// Downloads one object into memory when callers need its bytes immediately.
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

/// Lists a prefix and streams each matching file into a temporary path so
/// export code can package files without buffering them all in memory.
#[instrument(err)]
pub async fn get_files_from_s3(
    s3_bucket: String,
    prefix: String,
) -> Result<Vec<TempPath>> {
    let resolved_target = get_s3_list_target(&s3_bucket, true)
        .await
        .with_context(|| "Error getting s3 list target")?;
    let list_prefix = resolved_target.qualify_prefix(&prefix);
    let client = resolved_target.client;
    let bucket_name = resolved_target.bucket;

    let mut file_paths = Vec::new();

    let result = client
        .list_objects_v2()
        .bucket(&bucket_name)
        .prefix(&list_prefix)
        .send()
        .await?;

    for object in result.contents().iter() {
        let key = object.key().ok_or(anyhow!("s3 object key is missing"))?;

        if !key.contains("export") {
            // Extract file name and document ID
            let parts: Vec<&str> = key.split('/').collect();
            let s3_file_name = parts
                .last()
                .ok_or(anyhow!("Can't find file name in path"))?;
            let document_id = parts.iter().find_map(|part| {
                if part.starts_with("document-") {
                    Some(part.trim_start_matches("document-").to_string())
                } else {
                    None
                }
            });

            // Get object from S3
            let s3_object = client
                .get_object()
                .bucket(&bucket_name)
                .key(key)
                .send()
                .await?;

            let s3_body_stream = s3_object.body;

            let file_name = document_id
                .clone()
                .map(|id| format!("document_{}_{}", id, s3_file_name))
                .unwrap_or_else(|| s3_file_name.to_string());

            let temp_file = generate_temp_file("", &file_name)
                .context("generating temp file")?;

            let std_file = temp_file
                .reopen()
                .context("reopening temp file for async I/O")?;
            let mut async_file = tokio::fs::File::from_std(std_file);

            // Stream from S3 → disk without buffering into memory
            let mut reader = s3_body_stream.into_async_read();
            io::copy(&mut reader, &mut async_file)
                .await
                .context("stream-copy from S3 to temp file")?;

            file_paths.push(temp_file.into_temp_path());
        }
    }

    Ok(file_paths)
}

#[instrument(err)]
/// Lists a prefix and returns each file as name plus bytes for startup paths,
/// such as plugin loading, that need the content in memory.
pub async fn get_files_names_bytes_from_s3(
    s3_bucket: String,
    prefix: String,
) -> Result<Vec<(String, Vec<u8>)>> {
    let resolved_target = get_s3_list_target(&s3_bucket, true)
        .await
        .with_context(|| "Error getting S3 list target")?;
    let list_prefix = resolved_target.qualify_prefix(&prefix);
    let client = resolved_target.client;
    let bucket_name = resolved_target.bucket;

    let mut files_data: Vec<(String, Vec<u8>)> = Vec::new();

    // List objects under the given prefix
    let list_output = client
        .list_objects_v2()
        .bucket(&bucket_name)
        .prefix(&list_prefix)
        .send()
        .await
        .with_context(|| {
            format!(
                "Error listing objects in bucket `{}` with prefix `{}`",
                bucket_name, list_prefix
            )
        })?;

    // For each object, fetch and collect its bytes
    if let Some(contents) = list_output.contents {
        for object in contents {
            if let Some(key) = object.key {
                let file_name = key.split('/').last().unwrap();

                let get_obj_output = client
                    .get_object()
                    .bucket(&bucket_name)
                    .key(&key)
                    .send()
                    .await
                    .with_context(|| {
                        format!("Error getting object `{}`", key)
                    })?;

                // ByteStream -> Bytes -> Vec<u8>
                let bytes = ByteStream::collect(get_obj_output.body)
                    .await
                    .with_context(|| {
                        format!("Error streaming object `{}` body", key)
                    })?
                    .into_bytes()
                    .to_vec();

                files_data.push((file_name.to_string(), bytes));
            }
        }
    }

    Ok(files_data)
}

#[cfg(test)]
mod tests {
    use super::{
        get_s3_endpoint_style, join_s3_path, parse_bucket_hosted_endpoint,
        resolve_s3_list_target_parts, ResolvedS3ListTargetParts,
        S3EndpointStyle, AWS_S3_ENDPOINT_STYLE_ENV,
    };
    use std::env;

    #[test]
    fn s3_endpoint_style_defaults_to_path_style_or_auto_detected_aws() {
        assert_eq!(
            S3EndpointStyle::default(),
            S3EndpointStyle::PathStyleOrAutoDetectedAws
        );
    }

    #[test]
    fn s3_endpoint_style_parses_virtual_hosted() {
        assert_eq!(
            "virtual-hosted".parse::<S3EndpointStyle>().unwrap(),
            S3EndpointStyle::VirtualHosted
        );
    }

    #[test]
    fn s3_endpoint_style_rejects_unknown_value() {
        assert!("bogus".parse::<S3EndpointStyle>().is_err());
    }

    #[test]
    fn get_s3_endpoint_style_reads_env_var() {
        // Combined into one test (rather than three) because these cases all
        // mutate the same process-global env var, which isn't safe to do
        // from separately/concurrently run #[test] functions.
        env::remove_var(AWS_S3_ENDPOINT_STYLE_ENV);
        assert_eq!(
            get_s3_endpoint_style().unwrap(),
            S3EndpointStyle::PathStyleOrAutoDetectedAws
        );

        env::set_var(AWS_S3_ENDPOINT_STYLE_ENV, "virtual-hosted");
        assert_eq!(
            get_s3_endpoint_style().unwrap(),
            S3EndpointStyle::VirtualHosted
        );

        env::set_var(AWS_S3_ENDPOINT_STYLE_ENV, "bogus");
        let err = get_s3_endpoint_style().unwrap_err();
        assert!(err.to_string().contains(AWS_S3_ENDPOINT_STYLE_ENV));
        assert!(err.to_string().contains("virtual-hosted"));

        env::remove_var(AWS_S3_ENDPOINT_STYLE_ENV);
    }

    #[test]
    fn join_s3_path_handles_empty_segments() {
        assert_eq!(join_s3_path("public", "plugins/"), "public/plugins");
        assert_eq!(join_s3_path("public/", "/plugins/"), "public/plugins");
        assert_eq!(join_s3_path("", "plugins/"), "plugins");
        assert_eq!(join_s3_path("public", ""), "public");
    }

    #[test]
    fn parse_region_aware_aws_bucket_endpoint() {
        let parsed = parse_bucket_hosted_endpoint(
            "https://sequent-dev-bucket-eu-west-1-123.s3.eu-west-1.amazonaws.com",
            Some("eu-west-1"),
            S3EndpointStyle::PathStyleOrAutoDetectedAws,
        )
        .unwrap();

        assert_eq!(
            parsed,
            Some((
                "https://s3.eu-west-1.amazonaws.com".to_string(),
                "sequent-dev-bucket-eu-west-1-123".to_string(),
            ))
        );
    }

    #[test]
    fn parse_global_aws_bucket_endpoint() {
        let parsed = parse_bucket_hosted_endpoint(
            "https://sequent-dev-bucket-eu-west-1-123.s3.amazonaws.com",
            Some("eu-west-1"),
            S3EndpointStyle::PathStyleOrAutoDetectedAws,
        )
        .unwrap();

        assert_eq!(
            parsed,
            Some((
                "https://s3.eu-west-1.amazonaws.com".to_string(),
                "sequent-dev-bucket-eu-west-1-123".to_string(),
            ))
        );
    }

    #[test]
    fn parse_global_aws_bucket_endpoint_without_region_keeps_global_host() {
        let parsed = parse_bucket_hosted_endpoint(
            "https://sequent-dev-bucket-eu-west-1-123.s3.amazonaws.com",
            None,
            S3EndpointStyle::PathStyleOrAutoDetectedAws,
        )
        .unwrap();

        assert_eq!(
            parsed,
            Some((
                "https://s3.amazonaws.com".to_string(),
                "sequent-dev-bucket-eu-west-1-123".to_string(),
            ))
        );
    }

    #[test]
    fn parse_aws_bucket_endpoint_with_dotted_bucket_name() {
        // AWS bucket names may contain dots; the AWS host delimiter (`.s3.`)
        // must anchor the split rather than the first `.` in the host.
        let parsed = parse_bucket_hosted_endpoint(
            "https://my.dotted.bucket.s3.eu-west-1.amazonaws.com",
            Some("eu-west-1"),
            S3EndpointStyle::PathStyleOrAutoDetectedAws,
        )
        .unwrap();

        assert_eq!(
            parsed,
            Some((
                "https://s3.eu-west-1.amazonaws.com".to_string(),
                "my.dotted.bucket".to_string(),
            ))
        );
    }

    #[test]
    fn ignores_non_aws_endpoints() {
        let parsed = parse_bucket_hosted_endpoint(
            "http://minio:9000",
            Some("eu-west-1"),
            S3EndpointStyle::PathStyleOrAutoDetectedAws,
        )
        .unwrap();

        assert_eq!(parsed, None);
    }

    #[test]
    fn ignores_localhost_non_aws_endpoints() {
        let parsed = parse_bucket_hosted_endpoint(
            "http://127.0.0.1:9000",
            Some("eu-west-1"),
            S3EndpointStyle::PathStyleOrAutoDetectedAws,
        )
        .unwrap();

        assert_eq!(parsed, None);
    }

    #[test]
    fn parse_virtual_hosted_bucket_endpoint_for_any_provider() {
        // With the style explicitly configured as virtual-hosted, any
        // provider's bucket-hosted endpoint is recognized generically, using
        // the first hostname label as the bucket name. No provider needs to
        // be known by name.
        let parsed = parse_bucket_hosted_endpoint(
            "https://real-bucket-name.storage.example-provider.test",
            Some("eu-de"),
            S3EndpointStyle::VirtualHosted,
        )
        .unwrap();

        assert_eq!(
            parsed,
            Some((
                "https://storage.example-provider.test".to_string(),
                "real-bucket-name".to_string(),
            ))
        );
    }

    #[test]
    fn virtual_hosted_style_ignores_host_without_a_bucket_label() {
        // A bare, single-label host has no room for a bucket label before a
        // service host, so it can't be bucket-hosted.
        let parsed = parse_bucket_hosted_endpoint(
            "http://storage-provider:9000",
            Some("eu-de"),
            S3EndpointStyle::VirtualHosted,
        )
        .unwrap();

        assert_eq!(parsed, None);
    }

    #[test]
    fn virtual_hosted_style_ignores_ipv4_literal_hosts() {
        // Virtual-hosted-style addressing is a DNS-hostname convention; an
        // IP literal has no bucket label to extract, even though it happens
        // to contain dots.
        let parsed = parse_bucket_hosted_endpoint(
            "http://10.0.0.1:9000",
            Some("eu-de"),
            S3EndpointStyle::VirtualHosted,
        )
        .unwrap();

        assert_eq!(parsed, None);
    }

    #[test]
    fn virtual_hosted_style_ignores_ipv6_literal_hosts() {
        // `Url::host_str()` keeps the brackets around IPv6 literals (e.g.
        // `[::1]`); the bracket-stripping must still recognize them as IPs.
        let parsed = parse_bucket_hosted_endpoint(
            "http://[2001:db8::1]:9000",
            Some("eu-de"),
            S3EndpointStyle::VirtualHosted,
        )
        .unwrap();

        assert_eq!(parsed, None);
    }

    #[test]
    fn resolves_local_private_bucket_without_rewriting() {
        let resolved = resolve_s3_list_target_parts(
            "http://minio:9000",
            "election-event-documents",
            Some("us-east-1"),
            S3EndpointStyle::PathStyleOrAutoDetectedAws,
        )
        .unwrap();

        assert_eq!(
            resolved,
            ResolvedS3ListTargetParts {
                service_endpoint: None,
                bucket: "election-event-documents".to_string(),
                prefix_root: None,
            }
        );
    }

    #[test]
    fn resolves_local_public_bucket_without_rewriting() {
        let resolved = resolve_s3_list_target_parts(
            "http://127.0.0.1:9000",
            "public",
            Some("us-east-1"),
            S3EndpointStyle::PathStyleOrAutoDetectedAws,
        )
        .unwrap();

        assert_eq!(
            resolved,
            ResolvedS3ListTargetParts {
                service_endpoint: None,
                bucket: "public".to_string(),
                prefix_root: None,
            }
        );
    }

    #[test]
    fn resolves_production_public_bucket_to_real_bucket_and_prefix() {
        let resolved = resolve_s3_list_target_parts(
            "https://sequent-dev-bucket-eu-west-1-133529410358.s3.amazonaws.com",
            "public",
            Some("eu-west-1"),
            S3EndpointStyle::PathStyleOrAutoDetectedAws,
        )
        .unwrap();

        assert_eq!(
            resolved,
            ResolvedS3ListTargetParts {
                service_endpoint: Some(
                    "https://s3.eu-west-1.amazonaws.com".to_string(),
                ),
                bucket: "sequent-dev-bucket-eu-west-1-133529410358".to_string(),
                prefix_root: Some("public".to_string()),
            }
        );
    }

    #[test]
    fn resolves_production_private_bucket_to_real_bucket_and_prefix() {
        let resolved = resolve_s3_list_target_parts(
            "https://sequent-dev-bucket-eu-west-1-133529410358.s3.amazonaws.com",
            "election-event-documents",
            Some("eu-west-1"),
            S3EndpointStyle::PathStyleOrAutoDetectedAws,
        )
        .unwrap();

        assert_eq!(
            resolved,
            ResolvedS3ListTargetParts {
                service_endpoint: Some(
                    "https://s3.eu-west-1.amazonaws.com".to_string(),
                ),
                bucket: "sequent-dev-bucket-eu-west-1-133529410358".to_string(),
                prefix_root: Some("election-event-documents".to_string(),),
            }
        );
    }

    #[test]
    fn resolves_virtual_hosted_provider_bucket_to_real_bucket_and_prefix() {
        let resolved = resolve_s3_list_target_parts(
            "https://real-bucket-name.storage.example-provider.test",
            "election-event-documents",
            Some("eu-de"),
            S3EndpointStyle::VirtualHosted,
        )
        .unwrap();

        assert_eq!(
            resolved,
            ResolvedS3ListTargetParts {
                service_endpoint: Some(
                    "https://storage.example-provider.test".to_string(),
                ),
                bucket: "real-bucket-name".to_string(),
                prefix_root: Some("election-event-documents".to_string()),
            }
        );
    }
}
