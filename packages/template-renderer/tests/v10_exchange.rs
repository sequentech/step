// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
use base64::{engine::general_purpose::STANDARD, Engine};
use sequent_template_renderer::{execute, localization};
use serde_json::{json, Value};
use std::io::{Cursor, Read, Write};
use zip::{write::SimpleFileOptions, ZipArchive, ZipWriter};

fn report(id: &str) -> Value {
    execute(json!({"op":"catalog"}))["reports"]
        .as_array()
        .unwrap()
        .iter()
        .find(|report| report["id"] == id)
        .unwrap()
        .clone()
}
fn request(id: &str) -> Value {
    let report = report(id);
    json!({
        "op":"export_v10", "reportType":id, "source":report["source"],
        "template":report["configuration"], "languages":["en"],
        "channels":{"document":true,"email":false,"sms":false},
        "metadata":{"alias":id,"tenant_id":"e978b418-b4ce-4e80-8d76-3c6d7bc78bca","type":id.to_uppercase(),"communication_method":"DOCUMENT","labels":"{}","annotations":"{}"},
    })
}
fn csv_rows(result: &Value) -> Vec<Value> {
    let csv = result["csv"].as_str().unwrap_or_else(|| panic!("{result}"));
    assert_eq!(result["target"], "step-10.0");
    assert_eq!(result["mime"], "text/csv");
    execute(json!({"op":"import_v10","csv":csv}))["rows"]
        .as_array()
        .unwrap()
        .clone()
}
fn files(result: &Value) -> std::collections::BTreeMap<String, Vec<u8>> {
    let encoded = result["base64"]
        .as_str()
        .unwrap_or_else(|| panic!("{result}"));
    let mut zip = ZipArchive::new(Cursor::new(STANDARD.decode(encoded).unwrap())).unwrap();
    (0..zip.len())
        .map(|index| {
            let mut file = zip.by_index(index).unwrap();
            let mut bytes = Vec::new();
            file.read_to_end(&mut bytes).unwrap();
            (file.name().to_string(), bytes)
        })
        .collect()
}
fn native_render(source: &str, data: &Value) -> String {
    // No Studio helpers are registered. The stock v10 VIL uses native data
    // bindings and the lowered translations use native `with` only.
    let registry = handlebars::Handlebars::new();
    registry.render_template(source, data).unwrap()
}
fn error(input: Value) -> String {
    let result = execute(input);
    assert!(
        result.get("csv").is_none() && result.get("base64").is_none(),
        "{result}"
    );
    result["diagnostics"][0]["message"]
        .as_str()
        .unwrap()
        .to_string()
}

#[test]
fn v10_csv_preserves_runtime_credentials_and_exact_columns() {
    let mut input = request("credentials");
    input["data"] = json!({"password":"SAMPLE-MUST-NOT-BE-EXPORTED"});
    input["template"]["secret_attribute_names"] = json!(["external_id"]);
    input["template"]["audience_voter_ids"] = json!(["selected-voter"]);
    let result = execute(input);
    let csv = result["csv"].as_str().unwrap_or_else(|| panic!("{result}"));
    let mut reader = csv::Reader::from_reader(csv.as_bytes());
    assert_eq!(
        reader.headers().unwrap().iter().collect::<Vec<_>>(),
        vec![
            "alias",
            "tenant_id",
            "template",
            "created_by",
            "labels",
            "annotations",
            "created_at",
            "updated_at",
            "communication_method",
            "type"
        ]
    );
    assert!(!csv.contains("SAMPLE-MUST-NOT-BE-EXPORTED"));
    let rows = csv_rows(&result);
    assert_eq!(rows[0]["reportType"], "credentials");
    assert_eq!(
        rows[0]["template"]["secret_attribute_names"],
        json!(["external_id"])
    );
    assert_eq!(
        rows[0]["template"]["audience_voter_ids"],
        json!(["selected-voter"])
    );
    let source = rows[0]["template"]["document"].as_str().unwrap();
    let html = native_render(
        source,
        &json!({"username":"A&B <voter>","password":"0012-3456-7890-1234"}),
    );
    assert!(html.contains("0012-3456-7890-1234"));
    assert!(html.contains("A&amp;B &lt;voter&gt;"));
    assert!(source.contains("{{password}}"));
    assert!(!source.contains("studio_"));
}

#[test]
fn scalar_translations_compile_with_native_helpers_without_template_injection() {
    let mut input = request("credentials");
    input["source"] =
        json!("Éléction {{t \"title\"}} {{username}}\n{{#if username}}{{t \"title\"}}{{/if}}");
    let translation = "A & <B> \"quoted\" {{password}} {{/with}} \\ tail";
    input["translations"] = json!({"en":{"title":translation}});
    let rows = csv_rows(&execute(input));
    let source = rows[0]["template"]["document"].as_str().unwrap();
    let html = native_render(
        source,
        &json!({"username":"Runtime","password":"DO-NOT-EXPAND"}),
    );
    assert!(html.contains("Éléction A &amp; &lt;B&gt;"), "{html}");
    assert_eq!(html.matches("{{password}}").count(), 2);
    assert!(html.contains("Runtime"));
    assert!(!html.contains("DO-NOT-EXPAND"));
    assert!(!source.contains("studio_translate"));
}

#[test]
fn plain_translation_preserves_ampersands_and_runtime_fields() {
    let mut input = request("credentials");
    input["channel"] = json!("email");
    input["channels"]["email"] = json!(true);
    input["template"]["email"] = json!({"subject":"{{t \"title\"}}", "plaintext_body":"{{t \"title\"}}: {{username}}", "html_body":"<p>{{t \"title\"}}: {{username}}</p>"});
    input["translations"] = json!({"en":{"title":"A & B <C>"}});
    let rows = csv_rows(&execute(input));
    let config = &rows[0]["template"]["email"];
    assert_eq!(
        native_render(config["subject"].as_str().unwrap(), &json!({})),
        "A & B <C>"
    );
    assert_eq!(
        native_render(
            config["plaintext_body"].as_str().unwrap(),
            &json!({"username":"Voter"})
        ),
        "A & B <C>: Voter"
    );
    assert!(native_render(
        config["html_body"].as_str().unwrap(),
        &json!({"username":"Voter"})
    )
    .contains("A &amp; B &lt;C&gt;"));
    assert_eq!(rows[0]["metadata"]["communication_method"], "EMAIL");
}

#[test]
fn mixed_document_and_communications_compile_only_the_rendered_channel() {
    for include_email in [false, true] {
        let mut input = request("credentials");
        input["channels"]["sms"] = json!(true);
        input["channels"]["email"] = json!(include_email);
        input["template"]["sms"] = json!({"message":"{{t \"hello\"}} {{user.username}}"});
        input["template"]["email"] = json!({
            "subject":"Your report",
            "plaintext_body":"Report attached: A & B.",
            "html_body":"<p>Report attached: A &amp; B.</p>",
        });
        input["translations"] = json!({"en":{"hello":"Hello A & B"}});
        let rows = csv_rows(&execute(input.clone()));
        assert_eq!(rows.len(), if include_email { 3 } else { 2 });
        let document = rows
            .iter()
            .find(|row| row["metadata"]["communication_method"] == "DOCUMENT")
            .unwrap();
        assert_eq!(document["template"]["email"], input["template"]["email"]);
        let sms = rows
            .iter()
            .find(|row| row["metadata"]["communication_method"] == "SMS")
            .unwrap();
        let source = sms["template"]["sms"]["message"].as_str().unwrap();
        assert!(!source.contains("{{t "));
        assert!(!source.contains("studio_"));
        assert!(source.contains("{{user.username}}"));
        assert_eq!(
            native_render(source, &json!({"user":{"username":"Voter"}})),
            "Hello A & B Voter"
        );
        if include_email {
            let email = rows
                .iter()
                .find(|row| row["metadata"]["communication_method"] == "EMAIL")
                .unwrap();
            assert_eq!(email["template"]["email"], input["template"]["email"]);
        }
    }
}

#[test]
fn mixed_document_and_sms_reject_unsupported_sms_helpers() {
    let mut input = request("credentials");
    input["channels"]["sms"] = json!(true);
    input["template"]["sms"] = json!({"message":"{{number count}}"});
    assert!(error(input).contains("STEP 10.0"));
}

#[test]
fn literal_pdf_and_document_notification_fields_reject_handlebars() {
    for deployment in ["auto", "public-assets"] {
        for (section, fields) in [
            ("pdf_options", &["header_template", "footer_template"][..]),
            ("email", &["subject", "plaintext_body", "html_body"][..]),
        ] {
            for field in fields {
                for source in ["{{t \"title\"}}", "{{username}}"] {
                    let mut input = request("credentials");
                    input["deployment"] = json!(deployment);
                    input["template"][section][*field] = json!(source);
                    input["translations"] = json!({"en":{"title":"Literal translation"}});
                    let diagnostic = error(input);
                    assert!(diagnostic.contains(&format!("{section}.{field}")));
                    assert!(diagnostic.contains("Use literal wording"));
                }
            }
        }
    }
    let mut input = request("credentials");
    input["channels"]["email"] = json!(true);
    input["template"]["email"]["subject"] = json!("{{user.username}}");
    assert!(error(input).contains("without a DOCUMENT channel"));
}

#[test]
fn literal_pdf_and_document_notification_markup_is_preserved() {
    let mut input = request("credentials");
    input["template"]["pdf_options"]["header_template"] =
        json!("<span class='title'></span> — A &amp; B");
    input["template"]["pdf_options"]["footer_template"] =
        json!("<span class='pageNumber'></span>/<span class='totalPages'></span>");
    input["template"]["email"] = json!({
        "subject":"Your A & B report",
        "plaintext_body":"Report attached. <literal text>",
        "html_body":"<p>Report attached: A &amp; B.</p>",
    });
    for deployment in ["auto", "public-assets"] {
        input["deployment"] = json!(deployment);
        let result = execute(input.clone());
        let rows = if deployment == "auto" {
            csv_rows(&result)
        } else {
            execute(json!({"op":"import_v10","base64":result["base64"]}))["rows"]
                .as_array()
                .unwrap_or_else(|| panic!("{result}"))
                .clone()
        };
        assert_eq!(rows[0]["template"]["email"], input["template"]["email"]);
        assert_eq!(
            rows[0]["template"]["pdf_options"],
            input["template"]["pdf_options"]
        );
    }
}

#[test]
fn comments_and_escaped_translation_examples_are_not_executed() {
    let source = "{{! {{number secret}} }}\\{{t \"missing\"}}\n{{format_u64 count}}";
    let mut diagnostics = Vec::new();
    let compiled =
        localization::compile_v10(source, "en", "en", &json!({}), &mut diagnostics, true).unwrap();
    assert_eq!(source, compiled);
    assert!(diagnostics.is_empty());
}

#[test]
fn localized_or_unknown_runtime_helpers_fail_before_export() {
    for source in [
        "{{number count}}",
        "{{date issue_date}}",
        "{{studio_translate \"en\" \"x\"}}",
        "{{missing_helper username}}",
        "{{#if (number count)}}yes{{/if}}",
    ] {
        let mut input = request("credentials");
        input["source"] = json!(source);
        assert!(error(input).contains("STEP 10.0"), "{source}");
    }
    let mut input = request("credentials");
    input["source"] = json!("{{t \"votes\" count}}");
    input["translations"] = json!({"en":{"votes":{"one":"one vote","other":"{count} votes"}}});
    assert!(error(input).contains("plural"));
}

#[test]
fn incompatible_features_require_an_explicit_deployment_choice() {
    let mut input = request("credentials");
    input["template"]["pre_render"] = json!({"enabled":true});
    assert!(error(input.clone()).contains("Disable pre-render"));
    input["template"]["pre_render"]["enabled"] = json!(false);
    input["template"]["assets"] =
        json!({"logo.svg":{"mime":"image/svg+xml","base64":STANDARD.encode("<svg/>")}});
    for deployment in ["auto", "public-assets"] {
        input["deployment"] = json!(deployment);
        let diagnostic = error(input.clone());
        assert!(diagnostic.contains("temporary file://"));
        assert!(diagnostic.contains("inline data URLs or absolute deployment URLs"));
        assert!(diagnostic.contains("remove attached files"));
        assert!(diagnostic.contains("Studio archive"));
    }
    input["deployment"] = json!("auto");
    input["template"]["assets"] = json!({});
    input["template"]["system_template"] =
        json!("<html><body class='custom'>{{{rendered_user_template}}}</body></html>");
    assert!(error(input.clone()).contains("MinIO"));
    input["deployment"] = json!("public-assets");
    let result = execute(input);
    assert!(files(&result).contains_key("public-assets/credentials_system.hbs"));
}

#[test]
fn unknown_configuration_is_rejected_instead_of_silently_dropped() {
    for deployment in ["auto", "public-assets"] {
        let mut input = request("credentials");
        input["deployment"] = json!(deployment);
        input["template"]["future_runtime_feature"] = json!({"enabled":true});
        assert!(error(input).contains("future_runtime_feature"));
    }
    let mut input = request("credentials");
    input["template"]["pre_render"] = json!({"enabled":false});
    input["template"]["assets"] = json!({});
    input["template"]["system_template"] = report("credentials")["wrapper"].clone();
    input["template"]["selected_methods"] = json!(["DOCUMENT"]);
    let rows = csv_rows(&execute(input));
    for key in [
        "pre_render",
        "assets",
        "system_template",
        "selected_methods",
        "communication_templates",
    ] {
        assert!(rows[0]["template"].get(key).is_none(), "{key}");
    }
}

#[test]
fn public_assets_roundtrip_keeps_native_names_styles_and_options() {
    let mut input = request("credentials");
    input["deployment"] = json!("public-assets");
    input["template"]["system_template"] = json!(
        "<!doctype html><html><body class='custom'>{{{rendered_user_template}}}</body></html>"
    );
    let output = execute(input.clone());
    let entries = files(&output);
    for path in [
        "public-assets/credentials_user.hbs",
        "public-assets/credentials_system.hbs",
        "public-assets/credentials_extra_config.json",
    ] {
        assert!(entries.contains_key(path), "Missing {path}");
    }
    assert!(!entries.contains_key("public-assets/credentials.json"));
    assert!(!entries.contains_key("manifest.json"));
    let imported = execute(json!({"op":"import_v10","base64":output["base64"]}));
    let row = &imported["rows"][0];
    assert_eq!(row["reportType"], "credentials", "{imported}");
    assert_eq!(row["deployment"], "public-assets");
    assert_eq!(
        row["template"]["system_template"],
        input["template"]["system_template"]
    );
    assert_eq!(
        row["template"]["pdf_options"],
        input["template"]["pdf_options"]
    );
}

#[test]
fn public_assets_import_preserves_files_without_claiming_export_compatibility() {
    let image = "<svg xmlns='http://www.w3.org/2000/svg'/>";
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for (path, content) in [
        (
            "public-assets/credentials_user.hbs",
            "<img src='logos/brand.svg'> {{username}}",
        ),
        ("public-assets/logos/brand.svg", image),
    ] {
        writer
            .start_file(path, SimpleFileOptions::default())
            .unwrap();
        writer.write_all(content.as_bytes()).unwrap();
    }
    let bytes = writer.finish().unwrap().into_inner();
    let imported = execute(json!({"op":"import_v10","base64":STANDARD.encode(bytes)}));
    let row = &imported["rows"][0];
    assert_eq!(row["reportType"], "credentials", "{imported}");
    assert_eq!(
        row["template"]["assets"],
        json!({"logos/brand.svg":{"mime":"image/svg+xml","base64":STANDARD.encode(image)}})
    );
    let mut input = request("credentials");
    input["deployment"] = row["deployment"].clone();
    input["template"] = row["template"].clone();
    assert!(error(input).contains("remove attached files"));
}

#[test]
fn multiple_asset_locales_are_explicit_choices_without_overwrites() {
    let mut input = request("credentials");
    input["deployment"] = json!("public-assets");
    input["languages"] = json!(["en", "es"]);
    input["source"] = json!("{{t \"dear\"}} {{voter_full_name}}");
    let output = execute(input);
    let entries = files(&output);
    assert!(entries.contains_key("language/en/public-assets/credentials_user.hbs"));
    assert!(entries.contains_key("language/es/public-assets/credentials_user.hbs"));
    let imported = execute(json!({"op":"import_v10","base64":output["base64"]}));
    assert_eq!(imported["rows"].as_array().unwrap().len(), 2);
    assert_eq!(imported["rows"][0]["language"], "en");
    assert_eq!(imported["rows"][1]["language"], "es");
}

#[test]
fn mixed_deployment_contains_csv_and_public_assets() {
    let catalog = execute(json!({"op":"catalog"}));
    let public = catalog["reports"]
        .as_array()
        .unwrap()
        .iter()
        .find(|report| report["exportTarget"] == "public-assets")
        .unwrap();
    let output = execute(
        json!({"op":"export_v10","templates":[request("credentials"),request(public["id"].as_str().unwrap())]}),
    );
    let entries = files(&output);
    assert!(entries.contains_key("templates.csv"));
    let imported = execute(json!({"op":"import_v10","base64":output["base64"]}));
    assert_eq!(imported["rows"].as_array().unwrap().len(), 2, "{imported}");
    assert_eq!(imported["rows"][0]["reportType"], "credentials");
}

#[test]
fn duplicate_aliases_and_public_asset_families_are_rejected() {
    let input = request("credentials");
    assert!(error(json!({"op":"export_v10","templates":[input,input]}))
        .contains("Duplicate template alias"));
    let mut input = request("credentials");
    input["deployment"] = json!("public-assets");
    assert!(error(json!({"op":"export_v10","templates":[input,input]})).contains("collide"));
}

#[test]
fn every_catalog_family_has_a_native_export_path() {
    let catalog = execute(json!({"op":"catalog"}));
    for report in catalog["reports"].as_array().unwrap() {
        let output = execute(request(report["id"].as_str().unwrap()));
        assert!(
            output["csv"].is_string() || output["base64"].is_string(),
            "{}: {output}",
            report["id"]
        );
    }
}

#[test]
fn import_rejects_traversal_and_unknown_families() {
    for path in [
        "../credentials_user.hbs",
        "public-assets/unknown_family_user.hbs",
    ] {
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        writer
            .start_file(path, SimpleFileOptions::default())
            .unwrap();
        writer.write_all(b"Hello").unwrap();
        let bytes = writer.finish().unwrap().into_inner();
        assert!(!error(json!({"op":"import_v10","base64":STANDARD.encode(bytes)})).is_empty());
    }
}
