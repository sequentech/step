// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
use base64::{engine::general_purpose::STANDARD, Engine};
use sequent_template_renderer::{
    assets::{self, TemplateAsset, TemplateAssets},
    bundle,
    platform_csv::ImportedTemplate,
};
use serde_json::json;
use std::io::{Cursor, Write};
use zip::{write::SimpleFileOptions, ZipWriter};
fn asset(mime: &str, data: &[u8]) -> TemplateAsset {
    TemplateAsset {
        mime: mime.into(),
        base64: STANDARD.encode(data),
    }
}
fn files() -> TemplateAssets {
    [
        (
            "images/logo.png".into(),
            asset("image/png", &[137, 80, 78, 71, 0, 255]),
        ),
        (
            "scripts/report.js".into(),
            asset("text/javascript", b"document.title = 'Rendered';"),
        ),
        (
            "styles/main.css".into(),
            asset(
                "text/css",
                b"body { background: url('../images/logo.png') }",
            ),
        ),
        (
            "data/results.json".into(),
            asset("application/json", b"{\"count\":7}"),
        ),
        (
            "fonts/report.woff2".into(),
            asset("font/woff2", b"wOF2\0\xff"),
        ),
    ]
    .into_iter()
    .collect()
}
fn record() -> ImportedTemplate {
    ImportedTemplate {
        alias: "receipt".into(),
        tenant_id: "8dff067c-4cce-4566-bd52-25dc905f3cac".into(),
        template: json!({"document":"<h1>{{voter_full_name}}</h1><script src=\"/scripts/report.js\"></script>", "pdf_options":{"paper_width":8.5},"assets":files()}),
        created_by: "Studio".into(),
        labels: Some(json!({"team":"design"})),
        annotations: Some(json!("legacy cell")),
        created_at: None,
        updated_at: None,
        communication_method: "DOCUMENT".into(),
        r#type: "CREDENTIALS".into(),
    }
}
fn zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for (name, data) in entries {
        writer
            .start_file(*name, SimpleFileOptions::default())
            .unwrap();
        writer.write_all(data).unwrap();
    }
    writer.finish().unwrap().into_inner()
}
#[test]
fn zip_round_trip_preserves_paths_bytes_metadata_and_runtime_bindings() {
    let original = record();
    let bytes = bundle::encode(vec![original.clone()]).unwrap();
    assert!(bytes.starts_with(b"PK"));
    let decoded = bundle::decode(&bytes).unwrap();
    assert_eq!(
        serde_json::to_value(&decoded[0]).unwrap(),
        serde_json::to_value(original).unwrap()
    );
    assert_eq!(
        decoded[0].template["assets"]["fonts/report.woff2"]["base64"],
        STANDARD.encode(b"wOF2\0\xff")
    );
}
#[test]
fn file_envelope_survives_system_wrapper_and_is_removed_before_execution() {
    let part = assets::attach("<h1>Report</h1>", &files()).unwrap();
    let wrapped = format!("<html><body>{part}</body></html>");
    let (html, decoded) = assets::detach(&wrapped).unwrap();
    assert_eq!(html, "<html><body><h1>Report</h1></body></html>");
    assert_eq!(decoded, files());
    assert!(assets::detach(&format!("{part}{part}")).is_err());
}
#[test]
fn unsafe_paths_types_collisions_and_oversized_files_are_rejected() {
    for path in [
        "../x",
        "/etc/passwd",
        "a/../b",
        "a//b",
        "C:\\file",
        "a%2fb",
        "a?b",
        "x/",
        "a\0b",
        "__proto__",
        "__sequent_document__.html",
    ] {
        assert!(assets::validate_path(path).is_err(), "{path}");
    }
    let mut a = files();
    a.insert("IMAGES/logo.png".into(), asset("image/png", b"x"));
    assert!(assets::decode(&a).unwrap_err().contains("letter case"));
    let mut a = TemplateAssets::new();
    a.insert(
        "x.js".into(),
        asset("text/javascript\r\nX-Injected: yes", b"x"),
    );
    assert!(assets::decode(&a).is_err());
    a.insert(
        "x.js".into(),
        asset("text/javascript", &vec![0; assets::MAX_FILE_BYTES + 1]),
    );
    assert!(assets::decode(&a).unwrap_err().contains("2 MB"));
}
#[test]
fn zip_paths_links_and_undeclared_files_cannot_escape_the_package() {
    for path in ["../escape", "/absolute", "a\\b", "a%2fb"] {
        assert!(bundle::decode(&zip(&[(path, b"x")])).is_err());
    }
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    writer
        .add_symlink("link", "/etc/passwd", SimpleFileOptions::default())
        .unwrap();
    assert!(bundle::decode(&writer.finish().unwrap().into_inner())
        .unwrap_err()
        .contains("Links"));
    let manifest = br#"{"format":"sequent-template-bundle","version":2,"templates":[]}"#;
    assert!(bundle::decode(&zip(&[("manifest.json", manifest)]))
        .unwrap_err()
        .contains("version"));
    let bytes = bundle::encode(vec![record()]).unwrap();
    let mut writer = ZipWriter::new_append(Cursor::new(bytes)).unwrap();
    writer
        .start_file("secret.txt", SimpleFileOptions::default())
        .unwrap();
    writer.write_all(b"unlisted").unwrap();
    assert!(bundle::decode(&writer.finish().unwrap().into_inner())
        .unwrap_err()
        .contains("not declared"));
}
#[test]
fn request_resolution_has_no_external_or_filesystem_fallback() {
    assert_eq!(
        assets::request_path("https://template.invalid/images/logo.png?v=2"),
        Some("images/logo.png".into())
    );
    for url in [
        "file:///etc/passwd",
        "https://template.invalid.evil/image",
        "http://template.invalid/x",
        "https://template.invalid/a%2fb",
        "https://template.invalid/%2e%2e/private",
        "https://localhost/x",
    ] {
        assert!(assets::request_path(url).is_none(), "{url}");
    }
}
#[test]
fn studio_zip_operations_localize_and_round_trip_the_same_production_bundle() {
    let source = json!({"op":"export_zip", "source":"<h1>{{t \"title\"}} {{voter_full_name}}</h1>", "defaultLanguage":"en", "translations":{"en":{"title":"Hello"},"es":{"title":"Hola"}}, "languages":["en","es"], "template":{"assets":files()}, "metadata":{"alias":"receipt", "tenant_id":"8dff067c-4cce-4566-bd52-25dc905f3cac", "communication_method":"DOCUMENT", "type":"CREDENTIALS", "labels":"{\"team\":\"design\"}"}});
    let output = sequent_template_renderer::execute(source);
    assert_eq!(output["diagnostics"], json!([]), "{output}");
    let imported =
        sequent_template_renderer::execute(json!({"op":"import_zip", "base64":output["base64"]}));
    let rows = imported["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[1]["metadata"]["alias"], "receipt--es");
    let document = rows[1]["template"]["document"].as_str().unwrap();
    assert!(document.contains("Hola") && document.contains("{{voter_full_name}}"));
    assert_eq!(rows[1]["template"]["assets"], json!(files()));
}

#[test]
fn longest_asset_paths_still_round_trip_inside_zip_directories() {
    let mut row = record();
    let path = format!("images/{}.png", "x".repeat(229));
    assert_eq!(path.len(), 240);
    row.template["assets"][&path] = serde_json::to_value(asset("image/png", b"long path")).unwrap();
    let bytes = bundle::encode(vec![row.clone()]).unwrap();
    assert_eq!(bundle::decode(&bytes).unwrap()[0].template, row.template);
}
#[test]
fn repeated_file_references_cannot_amplify_decompressed_data() {
    let mut row = record();
    row.template.as_object_mut().unwrap().remove("assets");
    row.template.as_object_mut().unwrap().remove("document");
    let manifest = json!({"format":"sequent-template-bundle","version":1,"templates":[{
        "record":row,"files":{"a.txt":{"file":"shared.txt","mime":"text/plain"},"b.txt":{"file":"shared.txt","mime":"text/plain"}}
    }]});
    let bytes = zip(&[
        ("manifest.json", manifest.to_string().as_bytes()),
        ("shared.txt", b"one copy"),
    ]);
    assert!(bundle::decode(&bytes)
        .unwrap_err()
        .contains("only be referenced once"));
}
