use crate::services::database::get_hasura_pool;
use crate::services::s3;
use crate::services::vote_receipt;
use anyhow::{anyhow, Context, Result};
use celery::error::TaskError;
use std::env;
use tracing::instrument;

use deadpool_postgres::{Client as DbClient, Transaction};

pub async fn testing() -> Result<()> {
    let minio_private_uri =
        env::var("AWS_S3_PRIVATE_URI").map_err(|err| anyhow!("AWS_S3_PRIVATE_URI must be set"))?;
    let bucket = s3::get_public_bucket()?;

    let file = "public-assets/vote_receipt.hbs";

    let minio_endpoint = format!("{}/{}/{}", minio_private_uri, bucket, file);

    // http://minio:9000/public/public-assets/vote_receipt.hbs
    dbg!(&minio_endpoint);

    let client = reqwest::Client::new();
    let response = client.get(minio_endpoint).send().await?;

    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Err(anyhow!("File not found: {}", file));
    } else if !response.status().is_success() {
        return Err(anyhow!(
            "Unexpected response status: {:?}",
            response.status()
        ));
    }

    let response_body: String = response.text().await?;
    dbg!(&response_body);

    Ok(())
}
