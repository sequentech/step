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
        .map(|(key, value)| (key, value.into_vec()))
        .collect()
}
