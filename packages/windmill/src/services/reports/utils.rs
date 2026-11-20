// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Context, Result};
use sequent_core::services::s3::get_minio_url;
use std::env;
use tracing::instrument;

/// Function to get the public assets path environment variable
#[instrument(err, skip_all)]
pub fn get_public_assets_path_env_var() -> Result<String> {
    env::var("PUBLIC_ASSETS_PATH").map_err(|_| anyhow!("PUBLIC_ASSETS_PATH env var missing"))
}

/// Helper function to get public asset templates
#[instrument(err, skip_all)]
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

/// A CSV cell a spreadsheet would run as a formula starts with one of these;
/// it is written after a `'`, so opening the file runs nothing.
pub fn defuse_formula(text: &str) -> String {
    match text.chars().next() {
        Some('=' | '+' | '-' | '@' | '\t' | '\r') => format!("'{text}"),
        _ => text.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::defuse_formula;

    #[test]
    fn a_cell_a_spreadsheet_would_run_is_written_after_a_quote() {
        for (cell, written) in [
            ("=1+1", "'=1+1"),
            ("+63", "'+63"),
            ("-x", "'-x"),
            ("@SUM(A1)", "'@SUM(A1)"),
            ("\tx", "'\tx"),
            ("Dela Cruz", "Dela Cruz"),
            ("", ""),
            ("María-José", "María-José"),
        ] {
            assert_eq!(defuse_formula(cell), written);
        }
    }
}
