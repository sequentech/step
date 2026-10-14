// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use base64::engine::general_purpose;
use base64::Engine;
use cryptography::utils::serialization::{Deserializable, Serializable};

use crate::error::BallotError;

pub trait Base64Serialize {
    fn serialize(&self) -> Result<String, BallotError>;
}

pub trait Base64Deserialize {
    fn deserialize(value: String) -> Result<Self, BallotError>
    where
        Self: Sized;
}

impl<T: Serializable> Base64Serialize for T {
    fn serialize(&self) -> Result<String, BallotError> {
        let bytes = self.ser();
        Ok(general_purpose::STANDARD.encode(bytes))
    }
}

impl<T: Deserializable> Base64Deserialize for T {
    fn deserialize(value: String) -> Result<Self, BallotError>
    where
        Self: Sized,
    {
        let bytes_vec =
            general_purpose::STANDARD.decode(value).map_err(|error| {
                BallotError::Serialization(format!(
                    "Error decoding base64 string: {}",
                    error
                ))
            })?;
        Deserializable::deser(&bytes_vec.as_slice()).map_err(|error| {
            BallotError::Serialization(format!(
                "Error deserializing canonical crypto bytes: {}",
                error
            ))
        })
    }
}
