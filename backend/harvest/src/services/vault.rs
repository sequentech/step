// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use reqwest;
use std::env;

pub async fn save_secret(key: String, value: String) -> Result<()> {
    let server_url = env::var("VAULT_SERVER_URL")
        .expect(&format!("VAULT_SERVER_URL must be set"));
    let token =
        env::var("VAULT_TOKEN").expect(&format!("VAULT_TOKEN must be set"));
    let client = reqwest::Client::new();
    let pm_endpoint = format!("{}/v1/{}", &server_url, &key);
    let json_body = serde_json::from_str(value.as_str())?;
    let _res = client
        .post(pm_endpoint)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", token))
        .json(&json_body)
        .send()
        .await?;
    Ok(())
}
