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
fn archived_pre_render_receipt_still_localizes_exports_and_renders() {
    let catalog = sequent_template_renderer::execute(json!({"op":"catalog"}));
    let report = catalog["reports"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == "ballot_receipt")
        .unwrap();
    // The v10 catalog deliberately has no prerender starters. Keep exercising
    // an actual saved project instead of an empty filtered catalog iterator.
    let source = include_str!("../catalog/v1/prerender/ballot_receipt.hbs");
    let known = json!({"event":{"name":"Archived demonstration"}});
    let data = json!({"ballot_id":"receipt-1","timestamp":"October 2, 2026","ballot_tracker_url":"https://vote.example.org/track/receipt-1"});
    for language in ["en", "es"] {
        let input = json!({"reportType":report["id"],"source":source,"language":language,"defaultLanguage":"en","translations":report["translations"],"template":{"pre_render":{"enabled":true,"version":1,"known_data":known},"pdf_options":{"paper_width":8.5,"paper_height":11.0}},"data":data,"metadata":{"tenant_id":"00000000-0000-4000-8000-000000000099","type":report["id"].as_str().unwrap().to_uppercase(),"alias":report["id"]},"languages":[language]});
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
