// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
use serde_json::{json, Value};
/// Validate the structural subset used by the versioned, offline report catalog.
/// Unknown fields are accepted because report annotations are extensible.
pub fn validate(schema: &Value, value: &Value, path: &str, diagnostics: &mut Vec<Value>) {
    let kind = schema["type"].as_str().unwrap_or("");
    let valid = match kind {
        "object" => value.is_object(),
        "array" => value.is_array(),
        "string" => value.is_string(),
        "number" => value.is_number(),
        "boolean" => value.is_boolean(),
        _ => true,
    };
    if !valid && !value.is_null() {
        diagnostics.push(json!({"severity":"error","code":"scenario-type","path":path,"message":format!("{path}: expected {kind}")}));
        return;
    }
    if let Some(object) = value.as_object() {
        if let Some(properties) = schema["properties"].as_object() {
            for (key, child_schema) in properties {
                if let Some(child) = object.get(key) {
                    validate(child_schema, child, &format!("{path}/{key}"), diagnostics)
                }
            }
        }
    }
    if let Some(items) = value.as_array() {
        if let Some(item_schema) = schema.get("items") {
            for (i, item) in items.iter().enumerate() {
                validate(item_schema, item, &format!("{path}/{i}"), diagnostics)
            }
        }
    }
}
