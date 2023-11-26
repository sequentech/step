// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::Result;
use s3::bucket::Bucket;
use s3::creds::Credentials;
use s3::error::S3Error;
use s3::region::Region;
use s3::BucketConfiguration;
use std::env;
use tracing::instrument;

pub fn get_private_bucket() -> String {
    let minio_bucket = env::var("MINIO_BUCKET").expect(&format!("MINIO_BUCKET must be set"));
    minio_bucket
}

pub fn get_public_bucket() -> String {
    let minio_bucket =
        env::var("MINIO_PUBLIC_BUCKET").expect(&format!("MINIO_PUBLIC_BUCKET must be set"));
    minio_bucket
}

#[instrument(skip(data))]
pub async fn upload_to_s3(
    data: &Vec<u8>,
    key: String,
    media_type: String,
    minio_bucket: String,
) -> Result<()> {
    let key_id = env::var("MINIO_ACCESS_KEY").expect(&format!("MINIO_ACCESS_KEY must be set"));
    let key_secret =
        env::var("MINIO_ACCESS_SECRET").expect(&format!("MINIO_ACCESS_SECRET must be set"));
    let minio_private_uri =
        env::var("MINIO_PRIVATE_URI").expect(&format!("MINIO_PRIVATE_URI must be set"));
    //let minio_public_uri = env::var("MINIO_PUBLIC_URI")
    //    .expect(&format!("MINIO_PUBLIC_URI must be set"));
    let minio_region = env::var("MINIO_REGION").expect(&format!("MINIO_REGION must be set"));

    // 1) Instantiate the bucket client
    println!("=== Bucket instantiation");

    let private_region = Region::Custom {
        region: minio_region.to_owned(),
        endpoint: minio_private_uri.to_owned(),
    };
    let credentials = Credentials {
        access_key: Some(key_id.to_owned()),
        secret_key: Some(key_secret.to_owned()),
        security_token: None,
        session_token: None,
        expiration: None,
    };
    let bucket = Bucket::new(
        minio_bucket.as_str(),
        private_region.clone(),
        credentials.clone(),
    )?
    .with_path_style();
    println!("=== Bucket list");

    // 2) Create bucket if does not exist
    let result = bucket.head_object("/").await;
    let is_404_error = match result {
        Err(S3Error::Http(404, _)) => true,
        _ => false,
    };
    if is_404_error {
        println!("=== Bucket creation");
        let create_result = Bucket::create_with_path_style(
            minio_bucket.as_str(),
            private_region,
            credentials.clone(),
            BucketConfiguration::default(),
        )
        .await?;

        println!(
            "=== Bucket created\n{} - {} - {}",
            bucket.name, create_result.response_code, create_result.response_text
        );
    }

    // 3) Create object (binary)
    println!("=== Put content");
    bucket
        .put_object_with_content_type(key, data, media_type.as_str())
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
pub async fn get_document_url(key: String, minio_bucket: String) -> Result<String> {
    let key_id = env::var("MINIO_ACCESS_KEY").expect(&format!("MINIO_ACCESS_KEY must be set"));
    let key_secret =
        env::var("MINIO_ACCESS_SECRET").expect(&format!("MINIO_ACCESS_SECRET must be set"));
    let minio_public_uri =
        env::var("MINIO_PUBLIC_URI").expect(&format!("MINIO_PUBLIC_URI must be set"));
    let minio_region = env::var("MINIO_REGION").expect(&format!("MINIO_REGION must be set"));

    let credentials = Credentials {
        access_key: Some(key_id.to_owned()),
        secret_key: Some(key_secret.to_owned()),
        security_token: None,
        session_token: None,
        expiration: None,
    };
    let public_region = Region::Custom {
        region: minio_region.to_owned(),
        endpoint: minio_public_uri.to_owned(),
    };
    let public_bucket =
        Bucket::new(minio_bucket.as_str(), public_region, credentials.clone())?.with_path_style();
    let url = public_bucket.presign_get(key, 86400, None)?;

    Ok(url)
}

#[instrument]
pub async fn get_upload_url(key: String) -> Result<String> {
    let key_id = env::var("MINIO_ACCESS_KEY").expect(&format!("MINIO_ACCESS_KEY must be set"));
    let key_secret =
        env::var("MINIO_ACCESS_SECRET").expect(&format!("MINIO_ACCESS_SECRET must be set"));
    let minio_public_uri =
        env::var("MINIO_PUBLIC_URI").expect(&format!("MINIO_PUBLIC_URI must be set"));
    let minio_region = env::var("MINIO_REGION").expect(&format!("MINIO_REGION must be set"));
    let minio_bucket = get_public_bucket();

    let credentials = Credentials {
        access_key: Some(key_id.to_owned()),
        secret_key: Some(key_secret.to_owned()),
        security_token: None,
        session_token: None,
        expiration: None,
    };
    let public_region = Region::Custom {
        region: minio_region.to_owned(),
        endpoint: minio_public_uri.to_owned(),
    };
    let public_bucket =
        Bucket::new(minio_bucket.as_str(), public_region, credentials.clone())?.with_path_style();

    let upload_url = public_bucket.presign_put(key, 3600, None)?;
    Ok(upload_url)
}
