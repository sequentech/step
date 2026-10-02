// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
use sequent_template_renderer::{assets, execute, helpers};
use serde_json::{json, Value};

fn catalog() -> Value {
    execute(json!({"op": "catalog"}))
}

fn report(id: &str) -> Value {
    catalog()["reports"]
        .as_array()
        .unwrap()
        .iter()
        .find(|report| report["id"] == id)
        .unwrap()
        .clone()
}

fn clean_html(result: &Value) -> String {
    let html = result["html"]
        .as_str()
        .unwrap_or_else(|| panic!("{result}"));
    assets::detach(html).unwrap().0
}

#[test]
fn catalog_covers_the_pinned_release_and_marks_deployment_targets() {
    let catalog = catalog();
    assert_eq!(
        catalog["sourceCommit"],
        "4c97c734f67f648cb7fca93556fdb34ccdc7df38"
    );
    let reports = catalog["reports"].as_array().unwrap();
    assert_eq!(reports.len(), 28);
    let database: Vec<_> = reports
        .iter()
        .filter(|report| report["exportTarget"] == "database")
        .map(|report| report["id"].as_str().unwrap())
        .collect();
    assert_eq!(
        database,
        [
            "activity_logs",
            "ballot_images",
            "ballot_receipt",
            "credentials",
            "electoral_results",
            "initialization_report",
            "manual_verification",
            "participation_report"
        ]
    );
    for report in reports {
        assert_eq!(report["baseName"], report["id"]);
        assert_eq!(report["release"], "10.0");
        assert!(report.get("preRenderExample").is_none());
        assert!(report["sourcePaths"]["user"]
            .as_str()
            .unwrap()
            .ends_with("_user.hbs"));
        assert!(report["sourcePaths"]["system"]
            .as_str()
            .unwrap()
            .ends_with("_system.hbs"));
        assert!(report["configuration"].get("assets").is_none());
    }
    assert_eq!(
        report("ov_turnout_per_aboard_status_sex")["sourcePaths"]["extraConfig"],
        Value::Null
    );
    assert!(report("credentials")["label"]
        .as_str()
        .unwrap()
        .contains("VIL"));
    let ballot_images = report("ballot_images");
    assert_eq!(
        ballot_images["scenarios"][0]["data"]["data"]["ballot_data"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(ballot_images["sampleExact"], false);
    assert_eq!(
        ballot_images["sampleSha256"],
        "bb163b12fe6c0d7bedb8efdbc14b4cb032ae35e0db1e2c051091f0bcea973ae0"
    );
    assert_eq!(
        ballot_images["configuration"]["report_options"]["max_items_per_report"],
        100
    );
}

#[test]
fn every_release_document_matches_its_two_stage_handlebars_contract() {
    let root_tag = regex::Regex::new(r"<html[^>]*>").unwrap();
    for report in catalog()["reports"].as_array().unwrap() {
        for scenario in report["scenarios"].as_array().unwrap() {
            let user = helpers::render_template_text(
                report["source"].as_str().unwrap(),
                scenario["data"].as_object().unwrap().clone(),
            )
            .unwrap_or_else(|err| panic!("{} / {}: {err}", report["id"], scenario["name"]));
            let mut system = report["systemData"].as_object().unwrap().clone();
            system.insert("rendered_user_template".into(), json!(user));
            let expected =
                helpers::render_template_text(report["wrapper"].as_str().unwrap(), system).unwrap();
            let expected = root_tag.replace(&expected, "<html lang=\"en\" dir=\"ltr\">");
            let result = execute(
                json!({"reportType": report["id"], "source": report["source"], "data": scenario["data"]}),
            );
            assert_eq!(
                clean_html(&result),
                expected,
                "{} / {}",
                report["id"],
                scenario["name"]
            );
        }
    }
}

#[test]
fn system_context_is_separate_and_custom_system_template_is_used() {
    let result = execute(json!({
        "reportType": "credentials", "source": "{{password}}",
        "data": {"password": "private credential", "file_logo": "incorrect-user-logo"},
        "template": {"system_template": "{{password}}|{{{rendered_user_template}}}|{{file_logo}}"},
        "wrapper": "wrong wrapper"
    }));
    assert_eq!(
        clean_html(&result),
        "|private credential|/assets/sequent-logo.svg"
    );
}

#[test]
fn release_qr_and_logo_resources_are_available_offline_and_user_assets_override() {
    let report = report("manual_verification");
    let custom = json!({"mime": "image/svg+xml", "base64": "PHN2Zy8+"});
    let result = execute(json!({
        "reportType": report["id"], "source": report["source"], "data": report["scenarios"][0]["data"],
        "template": {"assets": {"assets/sequent-logo.svg": custom}}
    }));
    let (html, files) = assets::detach(result["html"].as_str().unwrap()).unwrap();
    assert!(html.contains("src=\"/assets/qrcode.min.js\""));
    assert!(html.contains("/assets/sequent-logo.svg"));
    assert_eq!(
        serde_json::to_value(&files["assets/sequent-logo.svg"]).unwrap(),
        custom
    );
    assert!(
        assets::decode(&files).unwrap()["assets/qrcode.min.js"]
            .bytes
            .len()
            > 10_000
    );
}

#[test]
fn communications_use_send_template_variables_and_production_escaping() {
    let configuration = json!({
        "email": {"subject": "To {{user.first_name}}", "plaintext_body": "{{user.username}} | {{vote_url}} | {{password}}", "html_body": null},
        "sms": {"message": "{{user.username}} | {{password}}"}
    });
    let context = json!({"user": {"first_name": "A & B", "username": "<voter>"}, "vote_url": "https://vote.example/?a=1&b=2"});
    for channel in ["email", "sms"] {
        let result = execute(json!({
            "reportType": "credentials", "channel": channel,
            "data": {"password": "document-only-secret"},
            "communicationData": context, "template": configuration
        }));
        let html = clean_html(&result);
        assert!(html.contains("&amp;lt;voter&amp;gt;"), "{html}");
        assert!(!html.contains("document-only-secret"));
        if channel == "email" {
            assert!(html.contains("To A &amp;amp; B"), "{html}");
        }
    }
    let result = execute(
        json!({"reportType": "credentials", "channel": "sms", "data": {"password": "document-only-secret"}, "template": configuration}),
    );
    assert!(clean_html(&result).contains("voter-001"));
    let result = execute(
        json!({"reportType": "credentials", "channel": "sms", "data": {"user":{"username":"canonical-user"},"password": "document-only-secret"}, "template": configuration}),
    );
    assert!(clean_html(&result).contains("canonical-user"));
    assert!(!clean_html(&result).contains("document-only-secret"));
}

#[test]
fn vil_keeps_server_formatted_credentials_dates_and_escaped_names() {
    let report = report("credentials");
    for scenario in report["scenarios"].as_array().unwrap() {
        let result = execute(
            json!({"reportType": "credentials", "source": report["source"], "data": scenario["data"]}),
        );
        let html = clean_html(&result);
        for field in ["password", "username", "issue_date"] {
            assert!(html.contains(&handlebars::html_escape(
                scenario["data"][field].as_str().unwrap()
            )));
        }
        assert!(!html.contains("<Anne>"));
    }
}

#[test]
fn release_images_have_sources_and_turnout_has_its_qr_target() {
    let image = regex::Regex::new(r"(?s)<img\b[^>]*>").unwrap();
    let source = regex::Regex::new(r#"\bsrc\s*=\s*"[^"]+""#).unwrap();
    for report in catalog()["reports"].as_array().unwrap() {
        for part in ["source", "wrapper"] {
            for tag in image.find_iter(report[part].as_str().unwrap()) {
                assert!(
                    source.is_match(tag.as_str()),
                    "{}: {}",
                    report["id"],
                    tag.as_str()
                );
            }
        }
    }
    assert!(report("voters_turnout_percentage")["source"]
        .as_str()
        .unwrap()
        .contains("id=\"qrcode\""));
}

#[test]
fn custom_plain_documents_do_not_load_unreferenced_release_assets() {
    let rendered = execute(
        json!({"reportType":"credentials","source":"<h1>Custom</h1><script>window.demo=true</script>","data":{}}),
    );
    assert!(!assets::has_envelope(rendered["html"].as_str().unwrap()));
    let report = report("credentials");
    let rendered = execute(
        json!({"reportType":"credentials","source":report["source"],"data":report["scenarios"][0]["data"]}),
    );
    let (_, files) = assets::detach(rendered["html"].as_str().unwrap()).unwrap();
    assert!(files.contains_key("sequent-logo.svg"));
    assert!(files.contains_key("assets/sequent-logo.svg"));
}

#[test]
fn ballot_images_keep_inlined_assets_and_unique_qr_targets_across_contests() {
    let report = report("ballot_images");
    let mut data = report["scenarios"][0]["data"].clone();
    let ballots = data["data"]["ballot_data"].as_array_mut().unwrap();
    ballots.truncate(1);
    let contests = ballots[0]["contest_choices"].as_array_mut().unwrap();
    contests.push(contests[0].clone());
    let rendered =
        execute(json!({"reportType":"ballot_images","source":report["source"],"data":data}));
    let html = clean_html(&rendered);
    assert!(html.contains("id=\"qrcode-0-0\""));
    assert!(html.contains("id=\"qrcode-1-0\""));
    assert!(html.contains("src=\"printing-office-logo.jpg\""));
    assert!(html.contains("img.getAttribute('src')?.startsWith('data:')"));
}
