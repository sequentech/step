// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use reqwest;

use crate::services::events::create_keys;
use crate::services::vault;
use braid::protocol_manager::{
    add_config_to_board, gen_protocol_manager, serialize_protocol_manager,
};
use std::env;
use strand::backend::ristretto::RistrettoCtx;
use strand::signature::StrandSignaturePk;

pub async fn create_keys(
    create_keys_body: create_keys::CreateKeysBody,
) -> Result<()> {
    let user =
        env::var("IMMUDB_USER").expect(&format!("IMMUDB_USER must be set"));
    let password = env::var("IMMUDB_PASSWORD")
        .expect(&format!("IMMUDB_PASSWORD must be set"));
    let server_url = env::var("IMMUDB_SERVER_URL")
        .expect(&format!("IMMUDB_SERVER_URL must be set"));
    let pm = gen_protocol_manager::<RistrettoCtx>();
    let pm_config = serialize_protocol_manager::<RistrettoCtx>(&pm);
    vault::save_secret(
        format!("boards/{}/protocol-manager", &create_keys_body.board_name),
        pm_config,
    )
    .await?;

    let trustee_pks = create_keys_body
        .trustee_pks
        .clone()
        .into_iter()
        .map(|public_key_string| {
            let public_key: StrandSignaturePk =
                public_key_string.try_into().unwrap();
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
        pm,
    )
    .await?;

    Ok(())
}
