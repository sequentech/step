// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
use sequent_template_renderer::*;
use serde_json::json;
#[test]
fn preserves_helpers_and_composition() {
    let output = execute(
        json!({"op":"render", "reportType":"credentials", "source":"{{format_u64 count}} {{name}}", "data":{"count":1234,"name":"<Jane>"}, "wrapper":"<main>{{{rendered_user_template}}}</main>"}),
    );
    assert!(output["html"]
        .as_str()
        .unwrap()
        .contains("<main>1,234 &lt;Jane&gt;</main>"));
}
#[test]
fn invalid_template_returns_diagnostics() {
    let output =
        execute(json!({"op":"render","reportType":"credentials","source":"{{#if}}","data":{}}));
    assert!(output["html"].is_null());
    assert_eq!(output["diagnostics"][0]["severity"], "error");
}
#[test]
fn catalog_is_exactly_eight() {
    assert_eq!(
        execute(json!({"op":"catalog"}))["reports"]
            .as_array()
            .unwrap()
            .len(),
        8
    );
}
#[test]
fn invalid_scenario_type_has_a_structured_path() {
    let result = execute(
        json!({"op":"render","reportType":"electoral_results","source":"hello","data":{"reports":"not an array"}}),
    );
    assert!(result["html"].is_null());
    assert_eq!(result["diagnostics"][0]["path"], "/data/reports");
}
#[test]
fn helper_fallback_is_visible_without_changing_production_output() {
    let result = execute(
        json!({"op":"render","reportType":"credentials","source":"{{format_u64 missing}}","wrapper":"{{{rendered_user_template}}}","data":{}}),
    );
    assert_eq!(result["html"], "-");
    assert_eq!(result["diagnostics"][0]["code"], "helper-fallback");
}
