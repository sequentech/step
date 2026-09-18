// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
use sequent_template_renderer::execute;
use serde_json::{json, Value};
fn request() -> Value {
    json!({"op":"render","reportType":"credentials","source":"{{t \"title\"}} {{t \"votes\" count}} {{number count}} {{date day}} {{username}}","wrapper":"<!doctype html><html><body>{{{rendered_user_template}}}</body></html>","data":{"count":2,"day":"2026-09-18","username":"<voter>"},"language":"es-MX","defaultLanguage":"en","translations":{"en":{"title":"Default","votes":{"one":"{count} vote","other":"{count} votes"}},"es":{"title":"Título","votes":{"one":"{count} voto","other":"{count} votos"}}}})
}
#[test]
fn regional_language_falls_back_per_key_and_escapes() {
    let result = execute(request());
    let html = result["html"].as_str().unwrap();
    assert!(html.contains("Título 2 votos"), "{result}");
    assert!(html.contains("&lt;voter&gt;"));
    assert_eq!(result["diagnostics"][0]["code"], "language-fallback");
}
#[test]
fn exact_override_and_default_fallback() {
    let mut req = request();
    req["overrides"] = json!({"es":"{{t \"title\"}}","es-MX":"Exact {{username}}"});
    assert!(execute(req.clone())["html"]
        .as_str()
        .unwrap()
        .contains("Exact"));
    req["language"] = json!("fr");
    assert!(execute(req)["html"].as_str().unwrap().contains("Default"));
}
#[test]
fn missing_key_is_visible() {
    let mut req = request();
    req["source"] = json!("{{t \"absent\"}}");
    let result = execute(req);
    assert!(result["html"].as_str().unwrap().contains("[absent]"));
    assert_eq!(result["diagnostics"][0]["code"], "missing-translation");
}
#[test]
fn arabic_plurals_and_rtl() {
    let mut req = request();
    req["language"] = json!("ar");
    req["translations"]["ar"] = json!({"title":"نتائج","votes":{"zero":"zero","one":"one","two":"two","few":"few","many":"many","other":"other"}});
    let result = execute(req);
    assert!(result["html"].as_str().unwrap().contains("two"), "{result}");
    assert_eq!(result["direction"], "rtl");
    assert!(result["html"].as_str().unwrap().contains("dir=\"rtl\""));
}
#[test]
fn csv_preserves_runtime_bindings_and_configuration() {
    let mut req = request();
    req["op"] = json!("export");
    req["languages"] = json!(["en", "es"]);
    req["metadata"] = json!({"alias":"test","tenant_id":"e978b418-b4ce-4e80-8d76-3c6d7bc78bca","type":"CREDENTIALS","communication_method":"DOCUMENT","labels":"{\"x\":1}","annotations":"{\"y\":2}"});
    req["template"] = json!({"pdf_options":{"landscape":true},"audience_voter_ids":["voter-1"],"email":{"subject":"Hello {{username}}","plaintext_body":"{{password}}"}});
    let result = execute(req);
    let csv = result["csv"].as_str().unwrap();
    let platform = sequent_template_renderer::platform_csv::parse(csv.as_bytes()).unwrap();
    assert_eq!(platform.len(), 2);
    assert_eq!(platform[0].template["pdf_options"]["landscape"], true);
    assert_eq!(platform[0].labels, Some(json!("{\"x\":1}")));
    assert!(platform[0].template["document"]
        .as_str()
        .unwrap()
        .contains("{{username}}"));
    let imported = execute(json!({"op":"import","csv":csv}));
    let rows = imported["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["metadata"]["alias"], "test--en");
    assert_eq!(rows[1]["metadata"]["alias"], "test--es");
    for row in rows {
        assert!(row["template"]["document"]
            .as_str()
            .unwrap()
            .contains("{{username}}"));
        assert_eq!(row["template"]["pdf_options"]["landscape"], true);
        assert_eq!(row["template"]["email"]["plaintext_body"], "{{password}}");
        assert_eq!(row["metadata"]["labels"], "{\"x\":1}");
    }
}
