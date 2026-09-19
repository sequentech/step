// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
use sequent_template_renderer::prerender::*;
use serde_json::json;
#[test]
fn known_loops_expand_stable_runtime_pointers_and_escape_labels() {
    let source = r#"<h1>[[event.name]]</h1>[[#each candidates]]<p>[[name]]</p>{{pdf_text "/results/[[id]]/votes" page=1 x=200 y=40 width=80 height=20 size=12}}[[/each]]"#;
    let result=prepare(source,&json!({"event":{"name":"<Demo>"},"candidates":[{"id":"a","name":"José"},{"id":"b","name":"Maya"}]})).unwrap();
    assert_eq!(result.fields.len(), 2);
    assert_eq!(result.fields[1].path, "/results/b/votes");
    assert!(result.html.contains("&lt;Demo&gt;"));
    assert!(result.html.contains("José"));
    assert!(!result.html.contains("pdf_text"));
}
#[test]
fn stage_boundaries_and_geometry_fail_closed() {
    for source in [
        "{{unknown}}",
        "{{#each votes}}x{{/each}}",
        "[[unknown]]",
        r#"{{pdf_text "/id" x=-1 y=1 width=3 height=4}}"#,
        r#"{{pdf_text "id" x=1 y=1 width=3 height=4}}"#,
        r#"{{pdf_mark "/selected" page=1.5 x=1 y=1 width=3 height=4}}"#,
    ] {
        assert!(prepare(source, &json!({})).is_err(), "{source}");
    }
}
#[test]
fn known_values_cannot_introduce_a_runtime_expression() {
    // Values remain data: braces in candidate names must not become template code.
    let result = prepare(
        "[[name]]",
        &json!({"name":"{{pdf_mark \"/choice\" x=0 y=0 width=10 height=10}}"}),
    );
    assert!(result.unwrap().fields.is_empty());
}
#[test]
fn every_pre_render_starter_localizes_exports_and_preserves_runtime_fields() {
    let catalog = sequent_template_renderer::execute(json!({"op":"catalog"}));
    for report in catalog["reports"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["preRenderExample"].is_object())
    {
        let example = &report["preRenderExample"];
        for language in ["en", "es"] {
            let input = json!({"reportType":report["id"],"source":example["source"],"language":language,"defaultLanguage":"en","translations":report["translations"],"template":{"pre_render":{"enabled":true,"version":1,"known_data":example["knownData"]},"pdf_options":example["pdfOptions"]},"data":example["scenarios"][0]["data"],"metadata":{"tenant_id":"00000000-0000-4000-8000-000000000099","type":report["id"].as_str().unwrap().to_uppercase(),"alias":report["id"]},"languages":[language]});
            let preview = sequent_template_renderer::execute(input.clone());
            assert!(preview["html"].is_string(), "{}: {preview}", report["id"]);
            let mut export = input.clone();
            export["op"] = json!("export");
            let exported = sequent_template_renderer::execute(export);
            assert!(exported["csv"].is_string(), "{exported}");
            let mut prep = input;
            prep["op"] = json!("prerender_prepare");
            let prepared = sequent_template_renderer::execute(prep);
            assert!(
                prepared["fields"].as_array().unwrap().len() > 0,
                "{prepared}"
            );
        }
    }
}
