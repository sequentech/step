// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The platform's ballot and signing encodings over wbraid's cryptography.

pub mod ballot;
pub mod hash;
pub mod signing;

pub use cryptography::context::{Context, RistrettoCtx};
pub use cryptography::utils::error::Error as CryptoError;
pub use cryptography::utils::serialization::{Deserializable, Serializable};

/// Application records keep their Borsh format independently of crypto values.
pub trait RecordSerialize {
    /// Encode an application record.
    fn record_serialize(&self) -> anyhow::Result<Vec<u8>>;
}

impl<T: borsh::BorshSerialize> RecordSerialize for T {
    fn record_serialize(&self) -> anyhow::Result<Vec<u8>> {
        Ok(borsh::to_vec(self)?)
    }
}

/// Decode a Borsh application record, rejecting trailing data.
pub trait RecordDeserialize: Sized {
    /// Decode one complete application record.
    fn record_deserialize(bytes: &[u8]) -> anyhow::Result<Self>;
}

impl<T: borsh::BorshDeserialize> RecordDeserialize for T {
    fn record_deserialize(bytes: &[u8]) -> anyhow::Result<Self> {
        Ok(borsh::from_slice(bytes)?)
    }
}

pub mod envelope;
