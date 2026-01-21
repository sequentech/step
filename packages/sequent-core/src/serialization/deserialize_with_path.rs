// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
//use serde::Deserialize;
use serde::de::{Deserialize, DeserializeOwned, IntoDeserializer};
use serde_json::{self, Value};
use serde_path_to_error;
use serde_path_to_error::Error;

pub fn deserialize_str<'de, T>(
    contents: &'de str,
) -> Result<T, Error<serde_json::Error>>
where
    T: Deserialize<'de>,
{
    let jd = &mut serde_json::Deserializer::from_str(contents);

    serde_path_to_error::deserialize(jd)
}

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
