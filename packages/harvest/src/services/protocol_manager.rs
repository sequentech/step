// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use braid::protocol_manager::{
    add_config_to_board, gen_protocol_manager, serialize_protocol_manager,
};
use reqwest;
use std::env;
use strand::backend::ristretto::RistrettoCtx;
use strand::signature::StrandSignaturePk;
use tracing::instrument;

use crate::services::events::create_keys;
use crate::services::vault;

#[instrument(skip(trustee_pks, threshold))]
pub async fn create_keys(
    board_name: &str,
    trustee_pks: Vec<String>,
    threshold: usize,
) -> Result<()> {
    // 1. get env vars
    let user =
        env::var("IMMUDB_USER").expect(&format!("IMMUDB_USER must be set"));
    let password = env::var("IMMUDB_PASSWORD")
        .expect(&format!("IMMUDB_PASSWORD must be set"));
    let server_url = env::var("IMMUDB_SERVER_URL")
        .expect(&format!("IMMUDB_SERVER_URL must be set"));

    // 2. create protocol manager keys
    let pm = gen_protocol_manager::<RistrettoCtx>();

    // 3. save pm keys in vault
    let pm_config = serialize_protocol_manager::<RistrettoCtx>(&pm);
    vault::save_secret(
        format!("boards/{}/protocol-manager", board_name),
        pm_config,
    )
    .await?;

    // 4. create trustees keys from input strings
    let trustee_pks: Vec<StrandSignaturePk> = trustee_pks
        .clone()
        .into_iter()
        .map(|public_key_string| {
            let public_key: StrandSignaturePk =
                public_key_string.try_into().unwrap();
            public_key
        })
        .collect();

    // 5. add config to board on immudb
    add_config_to_board::<RistrettoCtx>(
        server_url.as_str(),
        user.as_str(),
        password.as_str(),
        threshold.clone(),
        board_name,
        trustee_pks,
        pm,
    )
    .await?;

    Ok(())
}
