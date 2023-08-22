use std::{error::Error, str};

use s3::bucket::Bucket;
use s3::creds::Credentials;
use s3::region::Region;
use s3::BucketConfiguration;

// help from https://gist.github.com/jeremychone/4a6eb58822b65c5c3458fcba2db846c1

pub async fn upload_to_s3() -> Result<(), Box<dyn Error>> {
	// 1) Instantiate the bucket client
	println!("=== Bucket instanciation");
	let bucket = Bucket::new(
		"rust-s3",
		Region::Custom {
			region: "".to_owned(),
			endpoint: "http://minio:9000".to_owned(),
		},
		Credentials {
			access_key: Some("LZAw7hwBziRjwAhfP6Xi".to_owned()),
			secret_key: Some("4x8krlfXgEquxp9KhlCrCdkrECrszGQQlJa5nGct".to_owned()),
			security_token: None,
			session_token: None,
            expiration: None,
		}
	)?;
	println!("=== Bucket list");

	// 2) Create bucket if does not exist
	let (_, code) = bucket.head_object("/").await?;
	if code == 404 {
		println!("=== Bucket creation");
		let create_result = Bucket::create_with_path_style(
            "rust-s3",
            Region::Custom {
                region: "".to_owned(),
                endpoint: "http://minio:9000".to_owned(),
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
			bucket.name, create_result.response_code, create_result.response_text
		);
	}

	// 3) Create object (text/plain)
	let key = "test_file_2";
	println!("=== Put content");
	bucket
		.put_object_with_content_type(key, "NEW !!! Stuff!!!".as_bytes(), "text/plain")
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

    Ok(())
}