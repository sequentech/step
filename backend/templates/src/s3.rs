// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use std::{error::Error, str};

use s3::bucket::Bucket;
use s3::creds::Credentials;
use s3::error::S3Error;
use s3::region::Region;
use s3::BucketConfiguration;
use std::env;

pub async fn upload_to_s3(
    data: &Vec<u8>,
    media_type: String,
) -> Result<String, Box<dyn Error>> {
    let key_id = env::var("MINIO_ROOT_USER")
        .expect(&format!("MINIO_ROOT_USER must be set"));
    let key_secret = env::var("MINIO_ROOT_PASSWORD")
        .expect(&format!("MINIO_ROOT_PASSWORD must be set"));
    let minio_uri =
        env::var("MINIO_URI").expect(&format!("MINIO_URI must be set"));
    let minio_region =
        env::var("MINIO_REGION").expect(&format!("MINIO_REGION must be set"));
    let minio_bucket =
        env::var("MINIO_BUCKET").expect(&format!("MINIO_BUCKET must be set"));

    // 1) Instantiate the bucket client
    println!("=== Bucket instantiation");

    let region = Region::Custom {
        region: minio_region.to_owned(),
        endpoint: minio_uri.to_owned(),
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
        region.clone(),
        credentials.clone(),
    )?
    .with_path_style();
    println!("=== Bucket list");

    // 2) Create bucket if does not exist
    let result = bucket.head_object("/").await;
    let is404Error = match result {
        Err(S3Error::Http(404, _)) => true,
        _ => false,
    };
    if is404Error {
        println!("=== Bucket creation");
        let create_result = Bucket::create_with_path_style(
            minio_bucket.as_str(),
            region,
            credentials,
            BucketConfiguration::default(),
        )
        .await?;

        println!(
            "=== Bucket created\n{} - {} - {}",
            bucket.name,
            create_result.response_code,
            create_result.response_text
        );
    }

    // 3) Create object (binary)
    let key = "test_file_3";
    println!("=== Put content");
    bucket
        .put_object_with_content_type(key, data, media_type.as_str())
        .await?;

    // 5) Get signed url to file
    let url = bucket.presign_get(key, 86400, None)?;

    Ok(url)
}

// based on https://gist.github.com/jeremychone/4a6eb58822b65c5c3458fcba2db846c1

pub async fn upload_to_s30() -> Result<String, Box<dyn Error>> {
    // 1) Instantiate the bucket client
    println!("=== Bucket instantiation");
    let bucket = Bucket::new(
        "rust-s3",
        Region::Custom {
            region: "".to_owned(),
            endpoint: "http://127.0.0.1:9000".to_owned(),
        },
        Credentials {
            //access_key: Some("LZAw7hwBziRjwAhfP6Xl".to_owned()),
            //secret_key:
            // Some("4x8krlfXgEquxp9KhlCrCdkrECrszGQQlJa5nGct".to_owned()),
            access_key: Some("minio_user".to_owned()),
            secret_key: Some("minio_pass".to_owned()),
            security_token: None,
            session_token: None,
            expiration: None,
        },
    )?
    .with_path_style();
    println!("=== Bucket list");

    // 2) Create bucket if does not exist
    let result = bucket.head_object("/").await;
    let is404Error = match result {
        Err(S3Error::Http(404, _)) => true,
        _ => false,
    };
    if is404Error {
        println!("=== Bucket creation");
        let create_result = Bucket::create_with_path_style(
            "rust-s3",
            Region::Custom {
                region: "".to_owned(),
                endpoint: "http://127.0.0.1:9000".to_owned(),
            },
            Credentials {
                access_key: Some("minio_user".to_owned()),
                secret_key: Some("minio_pass".to_owned()),
                security_token: None,
                session_token: None,
                expiration: None,
            },
            BucketConfiguration::default(),
        )
        .await?;

        println!(
            "=== Bucket created\n{} - {} - {}",
            bucket.name,
            create_result.response_code,
            create_result.response_text
        );
    }

    // 3) Create object (text/plain)
    let key = "test_file_3";
    println!("=== Put content");
    bucket
        .put_object_with_content_type(
            key,
            "NEW !!! Stuff!!!".as_bytes(),
            "text/plain",
        )
        .await?;

    // 4) List bucket content
    println!("=== List bucket content");
    let results = bucket.list("/".to_owned(), Some("/".to_owned())).await?;
    for result in results {
        for item in result.contents {
            println!("key: {}", item.key);
        }
    }

    // 5) Get object content back
    println!("=== Get content");
    let data = bucket.get_object(key).await?;
    let data = str::from_utf8(data.as_slice()).expect("Wrong data!!!");
    println!("data: {}", data);

    let url = bucket.presign_get(key, 86400, None)?;

    Ok(url)
}
