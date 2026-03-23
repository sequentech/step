// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
//use serde::Deserialize;
use serde::de::{Deserialize, DeserializeOwned, IntoDeserializer};
use serde_json::{self, Value};
use serde_path_to_error;
use serde_path_to_error::Error;

/// Deserialize a value of type `T` from a JSON string, tracking the path to any error.
///
/// # Errors
/// Returns an error if deserialization fails, including the path to the error in the JSON structure.
pub fn deserialize_str<'de, T>(
    contents: &'de str,
) -> Result<T, Error<serde_json::Error>>
where
    T: Deserialize<'de>,
{
    let jd = &mut serde_json::Deserializer::from_str(contents);
    serde_path_to_error::deserialize(jd)
}

/// Deserialize a value of type `T` from a `serde_json::Value`, tracking the path to any error.
///
/// # Errors
/// Returns an error if deserialization fails, including the path to the error in the JSON structure.
pub fn deserialize_value<T>(value: Value) -> Result<T, Error<serde_json::Error>>
where
    T: DeserializeOwned, // Use DeserializeOwned since we consume the Value
{
    // Create a Deserializer from serde_json::Value
    let jd = value.into_deserializer();
    // Attempt to deserialize into type T, converting any errors using
    // serde_path_to_error
    serde_path_to_error::deserialize(jd)
}
