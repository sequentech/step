// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use base64::engine::general_purpose;
use base64::Engine;
use braid::protocol_manager;
use std::env;
use strand::backend::ristretto::RistrettoCtx;
use strand::serialization::StrandSerialize;
use tracing::instrument;

#[instrument]
pub async fn get_public_key(board_name: String) -> Result<String> {
    // 1. get env vars
    let user = env::var("IMMUDB_USER").expect(&format!("IMMUDB_USER must be set"));
    let password = env::var("IMMUDB_PASSWORD").expect(&format!("IMMUDB_PASSWORD must be set"));
    let server_url =
        env::var("IMMUDB_SERVER_URL").expect(&format!("IMMUDB_SERVER_URL must be set"));
    let pk = protocol_manager::get_board_public_key::<RistrettoCtx>(
        server_url.as_str(),
        user.as_str(),
        password.as_str(),
        board_name.as_str(),
    )
    .await?;
    let pk_bytes = pk.strand_serialize()?;
    Ok(general_purpose::STANDARD_NO_PAD.encode(pk_bytes))
}
