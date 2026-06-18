// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use base64::engine::general_purpose;
use base64::Engine;
use strand::serialization::{StrandDeserialize, StrandSerialize};

use crate::error::BallotError;

/// Encodes a strand-serializable value as a standard base64 string.
pub trait Base64Serialize {
    /// Serializes `self` to a base64 string without padding.
    ///
    /// # Errors
    ///
    /// Returns [`BallotError::Serialization`] when strand serialization fails.
    fn serialize(&self) -> Result<String, BallotError>;
}

/// Decodes a base64 string into a strand-deserializable value.
pub trait Base64Deserialize {
    /// Deserializes a base64 string into `Self`.
    ///
    /// # Errors
    ///
    /// Returns [`BallotError::Serialization`] when decoding or deserialization fails.
    fn deserialize(value: String) -> Result<Self, BallotError>
    where
        Self: Sized;
}

impl<T: StrandSerialize> Base64Serialize for T {
    fn serialize(&self) -> Result<String, BallotError> {
        let bytes = self
            .strand_serialize()
            .map_err(|error| BallotError::Serialization(error.to_string()))?;
        Ok(general_purpose::STANDARD_NO_PAD.encode(bytes))
    }
}

impl<T: StrandDeserialize> Base64Deserialize for T {
    fn deserialize(value: String) -> Result<Self, BallotError>
    where
        Self: Sized,
    {
        let bytes_vec = general_purpose::STANDARD_NO_PAD
            .decode(value)
            .map_err(|error| {
                BallotError::Serialization(format!(
                    "Error decoding base64 string: {}",
                    error
                ))
            })?;
        StrandDeserialize::strand_deserialize(&bytes_vec.as_slice()).map_err(
            |error| {
                BallotError::Serialization(format!(
                    "Error deserializing borsh/strand bytes: {}",
                    error
                ))
            },
        )
    }
}
