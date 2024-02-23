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
const LOGO_TEMPLATE: &'static str = "<div class=\"logo\"></div>";

pub async fn testing() -> Result<()> {
    let public_asset_path = env::var("PUBLIC_ASSETS_PATH")?;
    let file_vote_receipt_template = env::var("PUBLIC_ASSETS_VOTE_RECEIPT_TEMPLATE")?;
    let file_logo = env::var("PUBLIC_ASSETS_LOGO_IMG")?;
    let file_qrcode_lib = env::var("PUBLIC_ASSETS_QRCODE_LIB")?;
    let vote_receipt_title = env::var("VOTE_RECEIPT_TEMPLATE_TITLE")?;

    let minio_private_uri =
        env::var("AWS_S3_PRIVATE_URI").map_err(|err| anyhow!("AWS_S3_PRIVATE_URI must be set"))?;
    let bucket = s3::get_public_bucket()?;

    let minio_endpoint_base = format!("{}/{}", minio_private_uri, bucket);
    let vote_receipt_template = format!(
        "{}/{}/{}",
        minio_endpoint_base, public_asset_path, file_vote_receipt_template
    );

    let client = reqwest::Client::new();
    let response = client.get(vote_receipt_template).send().await?;

    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Err(anyhow!("File not found: {}", file_vote_receipt_template));
    } else if !response.status().is_success() {
        return Err(anyhow!(
            "Unexpected response status: {:?}",
            response.status()
        ));
    }

    let template_hbs: String = response.text().await?;

    let template = r#"
        <div>
        {{{data.logo}}}
        </div>
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
        {{{data.qrcode}}}
        </div>
    "#;

    let mut data = VoteReceiptData {
        ballot_id: "abc".to_string(),
        ballot_tracker_url: "https://localhost/".to_string(),
        qrcode: QR_CODE_TEMPLATE.to_string(),
        logo: LOGO_TEMPLATE.to_string(),
        file_logo: format!("{}/{}", minio_endpoint_base, file_logo),
        file_qrcode_lib: format!("{}/{}", minio_endpoint_base, file_qrcode_lib),
        title: vote_receipt_title.to_string(),
        template: None, // TODO
    };

    let sub_map = VoteReceiptRoot { data: data.clone() }.to_map()?;
    let computed_template = reports::render_template_text(&template, sub_map)?;
    dbg!(&computed_template);

    data.template = Some(computed_template.clone());

    let map = VoteReceiptRoot { data: data.clone() }.to_map()?;
    let computed_template_final = reports::render_template_text(&template_hbs, map)?;
    dbg!(&computed_template_final);

    Ok(())
}
