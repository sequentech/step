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

#[test]
fn every_catalog_report_shares_its_source_across_english_and_spanish_channels() {
    let catalog = execute(json!({"op": "catalog"}));
    for report in catalog["reports"].as_array().unwrap() {
        assert_eq!(report["defaultLanguage"], "en");
        assert_eq!(report["supportedLanguages"], json!(["en", "es"]));
        assert!(report["source"].as_str().unwrap().contains("{{t \""));
        for scenario in report["scenarios"].as_array().unwrap() {
            for channel in ["document", "email", "sms"] {
                let mut outputs = Vec::new();
                for language in ["en", "es"] {
                    let output = execute(json!({
                        "reportType": report["id"], "source": report["source"],
                        "data": scenario["data"], "language": language,
                        "channel": channel, "template": report["configuration"],
                    }));
                    let html = output["html"].as_str().unwrap_or_else(|| {
                        panic!(
                            "{} / {} / {language} / {channel}: {output}",
                            report["id"], scenario["name"]
                        )
                    });
                    assert!(html.contains(&format!("lang=\"{language}\"")), "{output}");
                    assert!(!output["diagnostics"].as_array().unwrap().iter().any(|d|
                        d["severity"] == "error" || d["code"] == "missing-translation"
                    ), "{} / {language} / {channel}: {output}", report["id"]);
                    assert!(!html.contains("{{t "), "{output}");
                    outputs.push(html.to_string());
                }
                // Actual translated wording changes, not just the document's lang attribute.
                assert_ne!(
                    outputs[0].replace("lang=\"en\"", ""),
                    outputs[1].replace("lang=\"es\"", "")
                );
            }
        }
    }
}

#[test]
fn a_single_translation_override_preserves_other_catalog_defaults() {
    let catalog = execute(json!({"op": "catalog"}));
    let report = catalog["reports"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == "credentials")
        .unwrap();
    let output = execute(json!({
        "reportType": "credentials", "source": report["source"],
        "data": report["scenarios"][0]["data"], "language": "es",
        "translations": {"es": {"username": "Identificador personal"}},
    }));
    let html = output["html"].as_str().unwrap();
    assert!(html.contains("Identificador personal"));
    assert!(html.contains("Contraseña"));
    assert!(html.contains("<title>Carta de información al votante</title>"));
    assert!(!output["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .any(|d| d["code"] == "missing-translation"));
}

#[test]
fn email_and_sms_plaintext_cannot_inject_markup() {
    let input = json!({
        "reportType": "credentials", "language": "es", "data": {"username": "A & B <img src=x>"},
        "template": {
            "email": {"subject": "<script>{{username}}</script>", "plaintext_body": "{{t \"username\"}}: {{username}}"},
            "sms": {"message": "{{t \"username\"}}: {{username}}"}
        }
    });
    for channel in ["email", "sms"] {
        let mut request = input.clone();
        request["channel"] = json!(channel);
        let output = execute(request);
        let html = output["html"].as_str().unwrap();
        assert!(
            html.contains("Nombre de usuario: A &amp; B &lt;img src&#x3D;x&gt;"),
            "{output}"
        );
        assert!(!html.contains("<img"));
        assert!(!html.contains("<script>"));
        assert!(!html.contains("&amp;amp;"));
    }
}

#[test]
fn export_emits_enabled_channels_and_localizes_without_rendering_sample_facts() {
    let catalog = execute(json!({"op": "catalog"}));
    let report = catalog["reports"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == "credentials")
        .unwrap();
    let mut input = json!({
        "op": "export_zip", "reportType": "credentials", "source": report["source"],
        "template": report["configuration"], "languages": ["en", "es"],
        "channels": {"document": true, "email": true, "sms": true},
        "metadata": {"alias": "welcome", "tenant_id": "e978b418-b4ce-4e80-8d76-3c6d7bc78bca", "type": "CREDENTIALS"}
    });
    let exported = execute(input.clone());
    assert!(exported["base64"].is_string(), "{exported}");
    let imported = execute(json!({"op": "import_zip", "base64": exported["base64"]}));
    let rows = imported["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 6);
    for (index, method) in ["DOCUMENT", "EMAIL", "SMS", "DOCUMENT", "EMAIL", "SMS"]
        .iter()
        .enumerate()
    {
        assert_eq!(rows[index]["metadata"]["communication_method"], *method);
        assert_eq!(rows[index]["template"]["communication_method"], *method);
    }
    assert_eq!(rows[0]["metadata"]["alias"], "welcome--en");
    assert_eq!(rows[4]["metadata"]["alias"], "welcome--email--es");
    assert_eq!(rows[5]["metadata"]["alias"], "welcome--sms--es");
    let document = rows[3]["template"]["document"].as_str().unwrap();
    let email = rows[4]["template"]["email"]["plaintext_body"]
        .as_str()
        .unwrap();
    let sms = rows[5]["template"]["sms"]["message"].as_str().unwrap();
    assert!(document.contains("{{username}}"));
    assert!(email.contains("{{username}}"));
    assert!(email.contains("Contraseña"));
    assert!(sms.contains("{{voting_portal_url}}"));
    assert!(sms.contains("Su portal de votación:"));
    assert!(!document.contains("{{t "));
    input["channel"] = json!("sms");
    let exported = execute(input.clone());
    let imported = execute(json!({"op": "import_zip", "base64": exported["base64"]}));
    assert_eq!(imported["rows"].as_array().unwrap().len(), 2);
    input["channels"]["sms"] = json!(false);
    assert!(execute(input)["diagnostics"][0]["message"]
        .as_str()
        .unwrap()
        .contains("Enable"));
}

#[test]
fn project_zip_keeps_report_types_channels_and_aliases_separate() {
    let catalog = execute(json!({"op": "catalog"}));
    let reports = catalog["reports"].as_array().unwrap();
    let templates: Vec<_> = ["credentials", "ballot_receipt"].iter().map(|id| {
        let report = reports.iter().find(|r| r["id"] == *id).unwrap();
        json!({
            "reportType": id, "source": report["source"], "template": report["configuration"],
            "supportedLanguages": ["en", "es"],
            "channels": {"document": true, "email": true, "sms": false},
            "metadata": {"alias": id, "tenant_id": "e978b418-b4ce-4e80-8d76-3c6d7bc78bca", "type": id.to_uppercase()}
        })
    }).collect();
    let output = execute(json!({"op": "export_zip", "templates": templates}));
    assert!(output["base64"].is_string(), "{output}");
    let imported = execute(json!({"op": "import_zip", "base64": output["base64"]}));
    let rows = imported["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 8);
    assert_eq!(rows[0]["metadata"]["type"], "CREDENTIALS");
    assert_eq!(rows[7]["metadata"]["type"], "BALLOT_RECEIPT");
    assert_eq!(rows[7]["metadata"]["alias"], "ballot_receipt--email--es");
    let duplicate = execute(json!({"op": "export_zip", "templates": [templates[0], templates[0]]}));
    assert!(duplicate["diagnostics"][0]["message"]
        .as_str()
        .unwrap()
        .contains("Duplicate"));
}
