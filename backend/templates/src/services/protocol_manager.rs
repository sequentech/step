// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use reqwest;

use crate::connection;
use crate::services::events::create_keys;
use std::env;

#[derive(Debug)]
pub struct ProtocolManagerClient {
    client: reqwest::Client,
    pm_url: String,
}

impl ProtocolManagerClient {
    pub fn new() -> Result<ProtocolManagerClient> {
        let pm_url = env::var("PROTOCOL_MANAGER_URI")
            .expect(&format!("HASURA_ENDPOINT must be set"));
        let client = reqwest::Client::new();
        Ok(ProtocolManagerClient {
            client: client,
            pm_url: pm_url,
        })
    }

    pub async fn create_keys(
        &mut self,
        create_keys_body: create_keys::CreateKeysBody
    ) -> Result<()> {
        let pm_endpoint = format!("{}/create-keys", self.pm_url);
        let _res = self.client
            .post(pm_endpoint)
            .json(&create_keys_body)
            .send()
            .await?;

        Ok(())
    }
}
