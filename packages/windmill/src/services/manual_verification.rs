// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
// SPDX-FileCopyrightText: 2024 Félix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2024 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::env;

use super::s3::{self, get_minio_url};
use crate::postgres::{self, election, template};
use crate::services::database::get_hasura_pool;
use crate::services::{documents::upload_and_return_document, temp_path::*};
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::{Client as DbClient, Transaction};
use sequent_core::services::keycloak;
use sequent_core::services::{pdf, reports};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use tracing::{event, instrument, Level};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ManualVerificationOutput {
    pub link: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ManualVerificationData {
    pub manual_verification_url: String,
    pub qrcode: String,
    pub logo: String,
    pub file_logo: String,
    pub file_qrcode_lib: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ManualVerificationRoot {
    pub data: ManualVerificationData,
}

trait ToMap {
    fn to_map(&self) -> Result<Map<String, Value>>;
}

impl ToMap for ManualVerificationRoot {
    fn to_map(&self) -> Result<Map<String, Value>> {
        let Value::Object(map) = serde_json::to_value(self.clone())? else {
            return Err(anyhow!("Can't convert ManualVerificationRoot to Map"));
        };
        Ok(map)
    }
}

impl ToMap for ManualVerificationData {
    fn to_map(&self) -> Result<Map<String, Value>> {
        let Value::Object(map) = serde_json::to_value(self.clone())? else {
            return Err(anyhow!("Can't convert ManualVerificationData to Map"));
        };
        Ok(map)
    }
}

#[instrument(err)]
async fn get_manual_verification_url(
    tenant_id: &str,
    election_event_id: &str,
    voter_id: &str,
) -> Result<String> {
    let keycloak_url =
        env::var("KEYCLOAK_URL").map_err(|_| anyhow!("KEYCLOAK_URL env var missing"))?;
    let base_url = std::env::var("VOTING_PORTAL_URL")
        .map_err(|_| anyhow!("VOTING_PORTAL_URL env var missing"))?;

    // redirect to login
    let login_url = format!("{base_url}/tenant/{tenant_id}/event/{election_event_id}/login");

    let generate_token_url = format!(
    "{keycloak_url}/realms/tenant-{tenant_id}-event-{election_event_id}/manual-verification/generate-link?userId={voter_id}&redirectUri={login_url}"
  );

    let client = reqwest::Client::new();

    event!(Level::INFO, "Requesting HTTP GET {:?}", generate_token_url);
    let response = client.get(generate_token_url).send().await?;

    let unwrapped_response = if response.status() != reqwest::StatusCode::OK {
        return Err(anyhow!("Error during generate_token_url"));
    } else {
        response
    };
    let response_body: ManualVerificationOutput = unwrapped_response.json().await?;

    Ok(response_body.link)
}

#[instrument(err)]
pub async fn get_manual_verification_pdf(
    document_id: &str,
    tenant_id: &str,
    election_event_id: &str,
    voter_id: &str,
) -> Result<()> {
    let public_asset_path = env::var("PUBLIC_ASSETS_PATH")?;
    let manual_verification_url =
        get_manual_verification_url(tenant_id, election_event_id, voter_id).await?;

    let minio_endpoint_base = get_minio_url()?;

    let data = ManualVerificationData {
        manual_verification_url: manual_verification_url.to_string(),
        qrcode: QR_CODE_TEMPLATE.to_string(),
        logo: LOGO_TEMPLATE.to_string(),
        file_logo: format!(
            "{}/{}/{}",
            minio_endpoint_base, public_asset_path, PUBLIC_ASSETS_LOGO_IMG
        ),
        file_qrcode_lib: format!(
            "{}/{}/{}",
            minio_endpoint_base, public_asset_path, PUBLIC_ASSETS_QRCODE_LIB
        ),
    };
    let map = ManualVerificationRoot { data: data.clone() }.to_map()?;
    let render = reports::render_template_text(
        r#"
        <html lang="en-US">

        <head>
          <meta charset="utf-8" />
          <meta name="viewport" content="width=device-width,initial-scale=1" />
          <title>Authenticate to Vote</title>
          <script src="{{data.file_qrcode_lib}}"></script>
          <style>
            body {
              font-family: Arial, sans-serif;
              margin: auto;
              max-width: 640px;
            }
        
            h1, h2, h3, h4 {
              margin-top: 48px;
              margin-bottom: 48px;
            }

            .logo {
              content: url("{{data.file_logo}}");
              text-align: center;
              margin: 16px auto;
            }
        
            .main {
              margin-top: 32px;
            }
        
            .main div {
              margin-top: 32px;
            }

            .id-content {
              print-color-adjust: exact;
              font-family: monospace;
              font-size:11px;
              padding: 6px 12px;
              background: #ecfdf5;
              color: #191d23;
              border-radius: 4px;
            }

            .info {
              margin: 32px 0;
            }
            .info p {
              margin: 12px 0;
            }
        
            #qrcode {
              margin-top: 32px;
              display: flex;
              justify-content: center;
            }
          </style>
        </head>

        <body>
          <main class="main">
            <div>
            {{{data.logo}}}
            </div>
            <div>
            <h2>Authenticate to Vote</h2>
            <p>
                Use the link below allows you to authenticate
                after having performed Manual Verification:
            </p>
            <div class="info">
                <p>
                <a href="{{data.manual_verification_url}}">Login Link</a>
                </p>
            </div>
            </div>
            
            <div>
            <p>
                You can also enter the link using the following QR code:
            </p>
            {{{data.qrcode}}}
            </div>
          </main>
        </body>

        <script>
          const qrcode = new QRCode(document.getElementById("qrcode"), {
            text: "{{data.manual_verification_url}}".replace("&#x3D;", "="),
            width: 480,
            height: 480,
            colorDark: '#000000',
            colorLight: '#ffffff',
            correctLevel: QRCode.CorrectLevel.M,
          });
        </script>

        </html>
        "#,
        map,
    )?;

    // Gen pdf
    let bytes_pdf = pdf::html_to_pdf(render)
        .map_err(|err| anyhow!("error rendering manual verification pdf: {}", err))?;
    let (_temp_path, temp_path_string, file_size) =
        write_into_named_temp_file(&bytes_pdf, "manual-verification-", ".pdf")
            .with_context(|| "Error writing to file")?;

    let auth_headers = keycloak::get_client_credentials().await?;
    let _document = upload_and_return_document(
        temp_path_string,
        file_size,
        "application/pdf".to_string(),
        auth_headers.clone(),
        tenant_id.to_string(),
        election_event_id.to_string(),
        format!("manual-verification-{voter_id}.pdf"),
        Some(document_id.to_string()),
        true,
    )
    .await?;

    Ok(())
}
