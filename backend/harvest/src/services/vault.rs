// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use reqwest;
use serde_json::json;
use std::env;

pub async fn save_secret(key: String, value: String) -> Result<()> {
    let server_url = env::var("VAULT_SERVER_URL")
        .expect(&format!("VAULT_SERVER_URL must be set"));
    let token =
        env::var("VAULT_TOKEN").expect(&format!("VAULT_TOKEN must be set"));
    let client = reqwest::Client::new();
    let pm_endpoint = format!("{}/v1/secrets/{}", &server_url, &key);
    let json_value = json!({"data": value});
    client
        .post(pm_endpoint)
        .bearer_auth(token)
        .json(&json_value)
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}
