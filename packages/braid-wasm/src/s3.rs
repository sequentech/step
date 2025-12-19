// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use wasm_bindgen::prelude::*;

/// Upload data to S3 using a pre-signed URL
pub async fn upload_to_s3(upload_url: &str, data: &[u8]) -> Result<(), JsValue> {
    let client = reqwest::Client::new();

    client
        .put(upload_url)
        .body(data.to_vec())
        .send()
        .await
        .map_err(|e| JsValue::from_str(&format!("S3 upload failed: {}", e)))?;

    Ok(())
}

/// Download data from S3 using a pre-signed URL
pub async fn download_from_s3(download_url: &str) -> Result<Vec<u8>, JsValue> {
    let client = reqwest::Client::new();

    let response = client
        .get(download_url)
        .send()
        .await
        .map_err(|e| JsValue::from_str(&format!("S3 download failed: {}", e)))?;

    response
        .bytes()
        .await
        .map(|b| b.to_vec())
        .map_err(|e| JsValue::from_str(&format!("Failed to read S3 response: {}", e)))
}
