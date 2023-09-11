// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use reqwest;

use crate::services::events::create_keys;
use std::env;
use braid::protocol_manager::{gen_protocol_manager, add_config_to_board};
use strand::backend::ristretto::RistrettoCtx;
use strand::signature::StrandSignaturePk;

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
        create_keys_body: create_keys::CreateKeysBody,
    ) -> Result<()> {
        let user = env::var("IMMUDB_USER")
            .expect(&format!("IMMUDB_USER must be set"));
        let password = env::var("IMMUDB_PASSWORD")
            .expect(&format!("IMMUDB_PASSWORD must be set"));
        let server_url = env::var("IMMUDB_SERVER_URL")
            .expect(&format!("IMMUDB_SERVER_URL must be set"));
        let pm = gen_protocol_manager::<RistrettoCtx>();

        let trustee_pks = create_keys_body
            .trustee_pks
            .clone()
            .into_iter()
            .map(|public_key_string| {
                let public_key: StrandSignaturePk = public_key_string.try_into().unwrap();
                public_key
            })
            .collect();
        add_config_to_board::<RistrettoCtx>(
            server_url.as_str(),
            user.as_str(),
            password.as_str(),
            create_keys_body.threshold.clone(),
            create_keys_body.board_name.as_str(),
            trustee_pks,
            pm
        ).await?;
        let pm_endpoint = format!("{}/create-keys", self.pm_url);
        let _res = self
            .client
            .post(pm_endpoint)
            .json(&create_keys_body)
            .send()
            .await?;

        Ok(())
    }
}
