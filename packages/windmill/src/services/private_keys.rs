// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use base64::engine::general_purpose;
use base64::Engine;

use strand::backend::ristretto::RistrettoCtx;
use strand::serialization::StrandSerialize;
use strand::signature::StrandSignaturePk;
use tracing::instrument;

use super::protocol_manager;
use super::public_keys::deserialize_public_key;

#[instrument(err)]
pub async fn get_trustee_encrypted_private_key(
    board_name: &str,
    trustee_pub_key: &str,
) -> Result<String> {
    let trustee_deserialized_pub_key: StrandSignaturePk =
        deserialize_public_key(trustee_pub_key.to_string())?;
    let private_key = protocol_manager::get_trustee_encrypted_private_key::<RistrettoCtx>(
        board_name,
        &trustee_deserialized_pub_key,
    )
    .await?;

    let private_key_bytes = private_key.strand_serialize()?;
    Ok(general_purpose::STANDARD_NO_PAD.encode(private_key_bytes))
}
