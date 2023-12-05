// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use base64::engine::general_purpose;
use base64::Engine;
use sequent_core::serialization::base64::Base64Deserialize;

use strand::backend::ristretto::RistrettoCtx;
use strand::serialization::StrandSerialize;
use tracing::instrument;

use super::protocol_manager;

#[instrument]
pub async fn get_trustee_encrypted_private_key(
    board_name: &str,
    trustee_pub_key: &str
) -> Result<String>
{
    let private_key = protocol_manager::get_trustee_encrypted_private_key::<RistrettoCtx>(
        board_name,
        trustee_pub_key
    ).await?;

    let private_key_bytes = private_key.strand_serialize()?;
    Ok(general_purpose::STANDARD_NO_PAD.encode(private_key_bytes))
}
