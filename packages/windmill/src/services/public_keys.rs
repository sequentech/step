// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Result};
use base64::engine::general_purpose;
use base64::Engine;

use deadpool_postgres::Transaction;
use strand::backend::ristretto::RistrettoCtx;
use strand::serialization::StrandSerialize;
use strand::signature::StrandSignaturePk;
use tracing::{info, instrument};

use super::protocol_manager;

#[instrument(err)]
pub fn deserialize_public_key(public_key_string: String) -> Result<StrandSignaturePk> {
    StrandSignaturePk::from_der_b64_string(&public_key_string).map_err(|err| anyhow!("{:?}", err))
}

#[instrument(skip(trustee_pks, threshold), err)]
pub async fn create_keys(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    board_name: &str,
    trustee_pks: Vec<String>,
    threshold: usize,
) -> Result<()> {
    // get protocol manager keys
    let pm = protocol_manager::get_protocol_manager(
        hasura_transaction,
        tenant_id,
        Some(election_event_id),
        board_name,
    )
    .await?;

    info!("test felix");

    // create trustees keys from input strings
    let trustee_pks: Vec<StrandSignaturePk> = trustee_pks
        .clone()
        .into_iter()
        .map(deserialize_public_key)
        .collect::<Result<Vec<_>>>()?;

    // add config to board on immudb
    protocol_manager::add_config_to_board::<RistrettoCtx>(threshold, board_name, trustee_pks, pm)
        .await?;

    Ok(())
}

#[instrument(err)]
pub async fn get_public_key(board_name: String) -> Result<String> {
    let pk = protocol_manager::get_board_public_key::<RistrettoCtx>(board_name.as_str()).await?;
    let pk_bytes = pk.strand_serialize()?;
    Ok(general_purpose::STANDARD_NO_PAD.encode(pk_bytes))
}
