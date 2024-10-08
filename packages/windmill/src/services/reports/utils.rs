// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::s3::get_minio_url;
use anyhow::{anyhow, Context, Result};
use serde::Serialize;
use serde_json::{Map, Value};
use std::env;

pub trait ToMap {
    fn to_map(&self) -> Result<Map<String, Value>>;
}

impl<T> ToMap for T
where
    T: Serialize + Clone,
{
    fn to_map(&self) -> Result<Map<String, Value>> {
        serde_json::to_value(self)
            .map_err(|e| anyhow!("Serialization error: {e}"))
            .and_then(|value| {
                if let Value::Object(map) = value {
                    Ok(map)
                } else {
                    Err(anyhow!(
                        "Error converting to serde_json::Value::Object: {value:?}"
                    ))
                }
            })
    }
}

/// Function to get the public assets path environment variable
pub fn get_public_assets_path_env_var() -> Result<String> {
    env::var("PUBLIC_ASSETS_PATH").map_err(|_| anyhow!("PUBLIC_ASSETS_PATH env var missing"))
}

/// Helper function to get public asset templates
pub async fn get_public_asset_template(filename: &str) -> Result<String> {
    let public_asset_path = get_public_assets_path_env_var()?;

    let minio_endpoint_base = get_minio_url().with_context(|| "Error getting minio endpoint")?;

    let template_url = format!("{}/{}/{}", minio_endpoint_base, public_asset_path, filename);

    let client = reqwest::Client::new();
    let response = client
        .get(&template_url)
        .send()
        .await
        .with_context(|| format!("Error sending request for template {}", filename))?;

    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Err(anyhow!("File not found: {}", template_url));
    }
    if !response.status().is_success() {
        return Err(anyhow!(
            "Unexpected response status: {:?}",
            response.status()
        ));
    }

    let template_hbs: String = response
        .text()
        .await
        .with_context(|| format!("Error reading the template response for {}", filename))?;

    Ok(template_hbs)
}
