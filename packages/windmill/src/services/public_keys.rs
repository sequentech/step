// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use base64::engine::general_purpose;
use base64::Engine;
use sequent_core::serialization::base64::Base64Deserialize;
use std::env;
use strand::backend::ristretto::RistrettoCtx;
use strand::serialization::{StrandDeserialize, StrandSerialize};
use strand::signature::StrandSignaturePk;
use tracing::instrument;

use super::protocol_manager;
use crate::services::vault;

pub fn deserialize_pk(public_key_string: String) -> StrandSignaturePk {
    Base64Deserialize::deserialize(public_key_string).unwrap()
}

#[instrument(skip(trustee_pks, threshold))]
pub async fn create_keys(
    board_name: &str,
    trustee_pks: Vec<String>,
    threshold: usize,
) -> Result<()> {
    // 2. create protocol manager keys
    let pm = protocol_manager::gen_protocol_manager::<RistrettoCtx>();

    // 3. save pm keys in vault
    let pm_config = protocol_manager::serialize_protocol_manager::<RistrettoCtx>(&pm);
    vault::save_secret(format!("boards/{}/protocol-manager", board_name), pm_config).await?;

    // 4. create trustees keys from input strings
    let trustee_pks: Vec<StrandSignaturePk> = trustee_pks
        .clone()
        .into_iter()
        .map(deserialize_pk)
        .collect();

    // 5. add config to board on immudb
    protocol_manager::add_config_to_board::<RistrettoCtx>(threshold, board_name, trustee_pks, pm)
        .await?;

    Ok(())
}

#[instrument]
pub async fn get_public_key(board_name: String) -> Result<String> {
    let pk = protocol_manager::get_board_public_key::<RistrettoCtx>(board_name.as_str()).await?;
    let pk_bytes = pk.strand_serialize()?;
    Ok(general_purpose::STANDARD_NO_PAD.encode(pk_bytes))
}
