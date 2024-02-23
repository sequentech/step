use crate::services::database::get_hasura_pool;
use crate::services::s3;
use crate::services::vote_receipt;
use crate::services::vote_receipt::VoteReceiptData;
use crate::services::vote_receipt::VoteReceiptRoot;
use anyhow::{anyhow, Context, Result};
use celery::error::TaskError;
use sequent_core::services::reports;
use std::env;
use tracing::instrument;

use deadpool_postgres::{Client as DbClient, Transaction};

const QR_CODE_TEMPLATE: &'static str = "<div id=\"qrcode\"></div>";

pub async fn testing() -> Result<()> {
    let minio_private_uri =
        env::var("AWS_S3_PRIVATE_URI").map_err(|err| anyhow!("AWS_S3_PRIVATE_URI must be set"))?;
    let bucket = s3::get_public_bucket()?;

    let file = "public-assets/vote_receipt_custom.hbs";

    let minio_endpoint = format!("{}/{}/{}", minio_private_uri, bucket, file);

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

    let template_hbs: String = response.text().await?;

    let template = r#"
    <div>{{logo}}</div>
    <div>
      <h2>Your vote has been cast</h2>
      <p>
        The confirmation code bellow verifies that your ballot has been cast
        successfully. You can use this code to verify that your ballot has
        been counted.
      </p>
      <p>
        Your Ballot ID: <span class="id-content">{{data.ballot_id}}</span>
      </p>
    </div>

    <div>
      <h3>Verify that your ballot has been cast</h3>
      <p>
        You can verify your ballot has been cast correctly at any moment using
        the following QR code:
      </p>
      {{qrcode}}
      <div id="qrcode"></div>
    </div>
    "#;

    let mut data = VoteReceiptData {
        ballot_id: "abc".to_string(),
        ballot_tracker_url: "https://localhost/".to_string(),
        qrcode: QR_CODE_TEMPLATE.to_string(),
        template: None,
    };

    let sub_map = VoteReceiptRoot { data: data.clone() }.to_map()?;
    let computed_template = reports::render_template_text(&template, sub_map)?;

    let map = VoteReceiptRoot { data: data.clone() }.to_map()?;
    let computed_template_final = reports::render_template_text(&template_hbs, map)?;

    dbg!(&computed_template);

    Ok(())
}
