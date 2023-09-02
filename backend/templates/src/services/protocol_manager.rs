// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::connection;
use anyhow::Result;
use reqwest;

#[derive(Debug)]
pub struct ProtocolManagerClient {
    client: reqwest::Client,
    pm_url: String,
}

impl ProtocolManagerClient {
    pub async fn new(pm_url: &str) -> Result<ProtocolManagerClient> {
        let client = reqwest::Client::new();
        Ok(ProtocolManagerClient {
            client: client,
            pm_url: String::from(pm_url),
        })
    }
}
