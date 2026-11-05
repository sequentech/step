// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The browser's view of monitoring configuration: the Admin Portal's
//! editor checks a document on every keystroke with the same policy
//! Harvest's save route runs.

use crate::monitoring::check::check_document;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(typescript_custom_section)]
const ITYPES: &'static str = r#"
export interface MonitoringProblem {
    severity: "error" | "warning";
    /** Stable identifier, safe to match on. Message wording is not. */
    code: string;
    /** Dotted path into the document: `chart.charts.bars[0].query`. */
    path: string;
    message: string;
    engine_code?: string;
}

export interface MonitoringReport {
    problems: MonitoringProblem[];
}
"#;

/// Checks `yaml` as the monitoring document of `kind` (`widget`,
/// `dashboard`, `theme` or `settings`) stored under `key`, alone or, given
/// the event's documents as JSON in `config_set_json`, against them.
#[wasm_bindgen(js_name = validateMonitoringConfig)]
pub fn validate_monitoring_config_js(
    kind: &str,
    key: &str,
    yaml: &str,
    config_set_json: &str,
) -> Result<JsValue, JsValue> {
    let report = check_document(kind, key, yaml, config_set_json);
    serde_wasm_bindgen::to_value(&report)
        .map_err(|error| JsValue::from_str(&error.to_string()))
}
