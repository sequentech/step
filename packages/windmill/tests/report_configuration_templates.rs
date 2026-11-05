// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Every default report template names the signed configuration its event
//! was imported from, and the copy it draws, and says neither for any other
//! event. A template added without them fails here.

use sequent_core::election_config::manifest::ConfigurationStamp;
use sequent_core::services::reports::render_template_text;
use serde_json::{json, Map, Value};
use std::fs;
use std::path::{Path, PathBuf};
use windmill::services::consolidation::transmission_package::render_eml;
use windmill::services::reports::generation::{copy_template_data, ReportCopy};
use windmill::services::reports::report_variables::stamp_template_data;

const STAMP_VARIABLES: [&str; 2] = [
    "execution_annotations.configuration_revision",
    "execution_annotations.configuration_manifest_sha256",
];
const COPY_VARIABLES: [&str; 2] = [
    "execution_annotations.copy_number",
    "execution_annotations.copy_total",
];

fn public_assets() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.devcontainer/minio/public-assets")
}

fn velvet_resources() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../velvet/src/resources")
}

fn templates_in(directory: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().to_string())
        .filter(|name| name.ends_with(".hbs"))
        .collect();
    names.sort();
    names
}

/// The templates that draw a report: each `<report>_user.hbs` of the public
/// assets, and Velvet's own contents. What only wraps them is left out.
fn report_templates() -> Vec<PathBuf> {
    let public = templates_in(&public_assets())
        .into_iter()
        .filter(|name| name.ends_with("_user.hbs"))
        .map(|name| public_assets().join(name));
    let velvet = templates_in(&velvet_resources())
        .into_iter()
        .filter(|name| !name.ends_with("_system.hbs") && !name.starts_with("report_base_"))
        .map(|name| velvet_resources().join(name));
    public.chain(velvet).collect()
}

fn stamp() -> ConfigurationStamp {
    ConfigurationStamp {
        external_id: "ov-2028".to_string(),
        revision: 3,
        manifest_sha256: "ab".repeat(32),
        template_sha256: "cd".repeat(32),
    }
}

fn stamp_line() -> String {
    format!(
        "Configuration revision 3, manifest SHA-256 {}",
        "ab".repeat(32)
    )
}

fn preview_data(base: &str) -> Map<String, Value> {
    let text = fs::read_to_string(public_assets().join(format!("{base}.json"))).unwrap();
    match serde_json::from_str(&text).unwrap() {
        Value::Object(data) => data,
        other => panic!("{base}.json is not an object: {other}"),
    }
}

fn user_template(base: &str) -> String {
    fs::read_to_string(public_assets().join(format!("{base}_user.hbs"))).unwrap()
}

#[test]
fn every_default_report_template_names_the_configuration_and_the_copy() {
    let templates = report_templates();
    assert!(templates.len() >= 30, "{templates:?}");
    for path in templates {
        let template = fs::read_to_string(&path).unwrap();
        for variable in STAMP_VARIABLES.iter().chain(&COPY_VARIABLES) {
            assert!(
                template.contains(variable),
                "{} does not print {variable}",
                path.display()
            );
        }
        assert!(
            template.contains("execution_annotations.configuration_revision}}"),
            "{} does not guard the configuration line",
            path.display()
        );
    }
}

/// The reports the platform generates from the Reports tab and the tally,
/// drawn with their preview data.
const PREVIEWED: [&str; 8] = [
    "electoral_results",
    "initialization_report",
    "activity_logs",
    "participation_report",
    "ballot_images",
    "ballot_receipt",
    "manual_verification",
    "credentials",
];

#[test]
fn a_report_of_a_signed_configuration_prints_its_revision_and_manifest_digest() {
    for base in PREVIEWED {
        let mut data = preview_data(base);
        stamp_template_data(&mut data, &stamp());
        let rendered = render_template_text(&user_template(base), data)
            .unwrap_or_else(|error| panic!("{base}: {error}"));
        assert!(rendered.contains(&stamp_line()), "{base}: {rendered}");
        assert!(!rendered.contains("Copy "), "{base}");
    }
}

#[test]
fn a_report_of_any_other_event_prints_neither() {
    for base in PREVIEWED {
        let rendered = render_template_text(&user_template(base), preview_data(base))
            .unwrap_or_else(|error| panic!("{base}: {error}"));
        assert!(!rendered.contains("Configuration revision"), "{base}");
        assert!(!rendered.contains("manifest SHA-256"), "{base}");
        assert!(!rendered.contains("Copy "), "{base}");
    }
}

#[test]
fn a_report_printed_in_several_copies_numbers_each() {
    for base in PREVIEWED {
        let mut data = preview_data(base);
        stamp_template_data(&mut data, &stamp());
        copy_template_data(
            &mut data,
            ReportCopy {
                number: 2,
                total: 7,
            },
        );
        let rendered = render_template_text(&user_template(base), data)
            .unwrap_or_else(|error| panic!("{base}: {error}"));
        assert!(rendered.contains("Copy 2 of 7"), "{base}: {rendered}");
        assert!(rendered.contains(&stamp_line()), "{base}");
    }
}

fn eml_template() -> String {
    fs::read_to_string(public_assets().join("eml_base.hbs")).unwrap()
}

fn eml_data() -> Value {
    json!({
        "id": "tally",
        "header": {
            "transaction_id": "1234",
            "issue_date": "2028-04-09T00:00:00",
            "official_status_detail": {
                "official_status": "official",
                "status_date": "2028-04-09"
            }
        },
        "counts": []
    })
}

#[test]
fn the_returns_xml_names_the_configuration_in_its_header() {
    let xml = render_eml(&eml_template(), eml_data(), Some(&stamp())).unwrap();
    let comment = format!("<!-- {} -->", stamp_line());
    assert_eq!(xml.matches(&comment).count(), 1, "{xml}");
    let header = xml.find("<EMLHeader>").unwrap();
    let transaction = xml.find("<TransactionId>1234</TransactionId>").unwrap();
    let at = xml.find(&comment).unwrap();
    assert!(header < at && at < transaction, "{xml}");
}

#[test]
fn the_returns_xml_of_any_other_event_is_what_it_was() {
    let template = eml_template();
    let before = template.replace(
        "    {{#if configuration}}\n    <!-- {{configuration}} -->\n    {{/if}}\n",
        "",
    );
    assert_ne!(before, template);
    let xml = render_eml(&template, eml_data(), None).unwrap();
    assert!(!xml.contains("<!--"), "{xml}");
    assert_eq!(xml, render_eml(&before, eml_data(), None).unwrap());
}
