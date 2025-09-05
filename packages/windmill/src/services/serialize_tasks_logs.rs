// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::ceremonies::serialize_logs::sort_logs;
use sequent_core::types::ceremonies::Log;
use sequent_core::{serialization::deserialize_with_path, services::date::ISO8601};
use serde_json::value::Value;
use tracing::{event, instrument, Level};

#[instrument]
pub fn general_start_log() -> Vec<Log> {
    vec![Log {
        created_date: ISO8601::to_string(&ISO8601::now()),
        log_text: format!("Task started"),
    }]
}

#[instrument(skip(current_logs))]
pub fn append_general_log(current_logs: &Option<Value>, message: &str) -> Vec<Log> {
    let value = current_logs.clone().unwrap_or(Value::Array(vec![]));
    let mut logs: Vec<Log> =
        deserialize_with_path::deserialize_value(value).unwrap_or_else(|_| Vec::new());
    logs.push(Log {
        created_date: ISO8601::to_string(&ISO8601::now()),
        log_text: format!("{}", message),
    });
    sort_logs(&logs)
}
