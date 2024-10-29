// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Context, Result};
use serde::Serialize;
use serde_json::{Map, Value};

pub trait ToMap {
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
