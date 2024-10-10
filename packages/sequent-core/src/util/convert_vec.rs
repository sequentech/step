// # SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
// #
// # SPDX-License-Identifier: AGPL-3.0-only

use serde_json::Value;
use std::collections::HashMap;

pub trait IntoVec {
    fn into_vec(self) -> Vec<String>;
}

impl IntoVec for String {
    fn into_vec(self) -> Vec<String> {
        vec![self]
    }
}

impl IntoVec for Vec<String> {
    fn into_vec(self) -> Vec<String> {
        self
    }
}

impl IntoVec for Value {
    fn into_vec(self) -> Vec<String> {
        match self {
            Value::String(s) => vec![s],
            Value::Array(arr) => arr
                .into_iter()
                .filter_map(|v| {
                    if let Value::String(s) = v {
                        Some(s)
                    } else {
                        None
                    }
                })
                .collect(),
            _ => vec![],
        }
    }
}

pub fn convert_map(
    original_map: HashMap<String, Value>,
) -> HashMap<String, Vec<String>> {
    original_map
        .into_iter()
        .map(|(key, value)| {
            let vec = match value {
                Value::Array(arr) => arr
                    .into_iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect(),
                Value::String(s) => vec![s],
                _ => Vec::new(),
            };
            (key, vec)
        })
        .collect()
}
