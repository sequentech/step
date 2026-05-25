// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Result};
use serde::Serialize;
use serde_json::{Map, Value};

/// Trait for converting a type to a JSON map representation.
/// Used for serializing structs and types to a map for flexible processing.
pub trait ToMap {
    /// Converts the type to a JSON map.
    ///
    /// # Errors
    /// Returns an error if serialization fails or the value cannot be converted to a map.
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
