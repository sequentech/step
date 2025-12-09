// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use base64::engine::general_purpose;
use base64::Engine;
use strand::serialization::{StrandDeserialize, StrandSerialize};

use crate::error::BallotError;

pub trait Base64Serialize {
    fn serialize(&self) -> Result<String, BallotError>;
}

pub trait Base64Deserialize {
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
