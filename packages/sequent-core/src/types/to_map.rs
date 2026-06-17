// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Serialize structs into JSON object maps for GraphQL variables and templates.

use anyhow::{anyhow, Context, Result};
use serde::Serialize;
use serde_json::{Map, Value};

/// Converts a serializable value into a flat JSON object map.
pub trait ToMap {
    /// Serializes `self` and returns the resulting JSON object.
    ///
    /// Returns an error if serialization fails or the value is not a JSON object.
    fn to_map(&self) -> Result<Map<String, Value>>;
}

impl<T> ToMap for T
where
    T: Serialize + Clone,
{
    fn to_map(&self) -> Result<Map<String, Value>> {
        serde_json::to_value(self)
            .map_err(|e| anyhow!("Serialization error: {e}"))
            .and_then(|value| {
                if let Value::Object(map) = value {
                    Ok(map)
                } else {
                    Err(anyhow!(
                        "Error converting to serde_json::Value::Object: {value:?}"
                    ))
                }
            })
    }
}
