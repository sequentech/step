// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Persisted application framing for encrypted values. Crypto operations are
//! supplied by wbraid; decoding never rewrites storage or accesses plaintext.

use crate::RecordDeserialize;
use anyhow::{bail, Result};
use borsh::{BorshDeserialize, BorshSerialize};
use cryptography::utils::serialization::Deserializable;
use cryptography::utils::symm::EncryptionData;

/// The fixed v1 application format used by encrypted Keycloak attributes.
#[derive(BorshSerialize, BorshDeserialize)]
pub struct StoredEnvelope {
    pub encrypted_bytes: Vec<u8>,
    pub nonce: [u8; 12],
}

impl From<EncryptionData> for StoredEnvelope {
    fn from(value: EncryptionData) -> Self {
        Self {
            encrypted_bytes: value.encrypted_bytes,
            nonce: value.nonce,
        }
    }
}
impl From<StoredEnvelope> for EncryptionData {
    fn from(value: StoredEnvelope) -> Self {
        Self {
            encrypted_bytes: value.encrypted_bytes,
            nonce: value.nonce,
        }
    }
}

/// Read either supported persisted format, strictly consuming all bytes.
pub fn read_encrypted_value(bytes: &[u8]) -> Result<EncryptionData> {
    let native = EncryptionData::deser(bytes).and_then(|value| {
        if value.encrypted_bytes.len() < 16 {
            Err(cryptography::utils::error::Error::DeserializationError(
                "missing authentication tag".into(),
            ))
        } else {
            Ok(value)
        }
    });
    let stored = StoredEnvelope::record_deserialize(bytes).and_then(|value| {
        if value.encrypted_bytes.len() < 16 {
            bail!("missing authentication tag");
        }
        Ok(value)
    });
    match (native, stored) {
        (Ok(current), Err(_)) => Ok(current),
        (Err(_), Ok(stored)) => Ok(stored.into()),
        (Err(_), Err(_)) => bail!("invalid stored encrypted envelope"),
        (Ok(_), Ok(_)) => bail!("ambiguous stored encrypted envelope"),
    }
}
