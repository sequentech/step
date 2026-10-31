// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Stable object-key ordering, independent of serde_json's preserve_order feature.
//! That feature is unified by Cargo: a CLI dependency can enable it while the
//! browser build does not, so ordinary Value serialization is not a wire contract.

use serde::ser::{SerializeMap, SerializeSeq};
use serde::{Serialize, Serializer};
use serde_json::Value;

struct Canonical<'a>(&'a Value);

impl Serialize for Canonical<'_> {
    fn serialize<S: Serializer>(
        &self,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        match self.0 {
            Value::Object(fields) => {
                let mut entries: Vec<_> = fields.iter().collect();
                entries
                    .sort_unstable_by(|(left, _), (right, _)| left.cmp(right));
                let mut map = serializer.serialize_map(Some(entries.len()))?;
                for (key, value) in entries {
                    map.serialize_entry(key, &Canonical(value))?;
                }
                map.end()
            }
            Value::Array(values) => {
                let mut sequence =
                    serializer.serialize_seq(Some(values.len()))?;
                for value in values {
                    sequence.serialize_element(&Canonical(value))?;
                }
                sequence.end()
            }
            value => value.serialize(serializer),
        }
    }
}

/// Compact JSON with recursively sorted object keys; array order is preserved.
pub fn compact(value: &Value) -> Result<String, serde_json::Error> {
    serde_json::to_string(&Canonical(value))
}

/// The same canonical ordering with the native archive's two-space indentation.
pub fn pretty(value: &Value) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(&Canonical(value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Map};

    #[test]
    fn object_keys_are_sorted_recursively_and_array_order_is_unchanged() {
        let mut nested = Map::new();
        nested.insert("z".into(), json!(1));
        nested.insert("b".into(), json!(2));
        let mut outer = Map::new();
        outer.insert("z".into(), json!(0));
        outer.insert("a".into(), json!([Value::Object(nested), 3, 1]));
        assert_eq!(
            compact(&Value::Object(outer)).unwrap(),
            r#"{"a":[{"b":2,"z":1},3,1],"z":0}"#
        );
    }
}
