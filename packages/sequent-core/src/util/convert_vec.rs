// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
// #
// # SPDX-License-Identifier: AGPL-3.0-only

use serde_json::Value;
use std::{collections::HashMap, hash::BuildHasher};

/// A trait for converting a value into a `Vec<String>`.
pub trait IntoVec {
    /// Converts a value into a `Vec<String>`.
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

#[must_use]
/// Converts a `HashMap<String, Value>` to a `HashMap<String, Vec<String>>`,
///  where the `Value` can be either a `String` or an `Array` of `Strings`.
pub fn convert_map<S>(
    original_map: HashMap<String, Value, S>,
) -> HashMap<String, Vec<String>, S>
where
    S: BuildHasher + Default,
{
    original_map
        .into_iter()
        .map(|(key, value)| {
            let vec = match value {
                Value::Array(arr) => arr
                    .into_iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect(),
                Value::String(s) => vec![s],
                _ => Vec::new(),
            };
            (key, vec)
        })
        .collect()
}
