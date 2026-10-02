// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
use sequent_template_renderer::sample_data::*;
use serde_json::json;
use std::io::{Cursor, Write};
fn export() -> serde_json::Value {
    json!({"tenant_id":"tenant","keycloak_event_realm":{"secret":"never import"},"election_event":{"id":"event","name":"Municipal election"},"elections":[{"id":"election","name":"General"}],"contests":[{"id":"contest","election_id":"election","name":"Mayor","max_votes":1}],"candidates":[{"id":"candidate","contest_id":"contest","name":"María Santos","annotations":{"private":"omit"}}],"voters":[{"password":"never import"}]})
}
#[test]
fn event_zip_import_extracts_only_structure_and_round_trips() {
    let mut source = export();
    source["candidates"][0]["presentation"] =
        json!({"i18n":{"es":{"name":"María Santos","description":"Candidata"}}});
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    zip.start_file(
        "event/export_election_event-demo.json",
        zip::write::SimpleFileOptions::default(),
    )
    .unwrap();
    zip.write_all(source.to_string().as_bytes()).unwrap();
    zip.start_file("voters.csv", zip::write::SimpleFileOptions::default())
        .unwrap();
    zip.write_all(b"password,secret").unwrap();
    let data = import(&zip.finish().unwrap().into_inner()).unwrap();
    assert_eq!(data["counts"]["candidates"], 1);
    assert_eq!(
        data["knownData"]["contests"][0]["candidates"][0]["name"],
        "María Santos"
    );
    assert!(!data.to_string().contains("never import"));
    assert!(!data.to_string().contains("annotations"));
    assert_eq!(import(data.to_string().as_bytes()).unwrap(), data);
}
#[test]
fn malformed_relationships_and_ambiguous_archives_are_rejected() {
    let mut value = export();
    value["candidates"][0]["contest_id"] = json!("missing");
    assert!(from_export(&value).is_err());
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for name in [
        "export_election_event-a.json",
        "export_election_event-b.json",
    ] {
        zip.start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(export().to_string().as_bytes()).unwrap();
    }
    assert!(import(&zip.finish().unwrap().into_inner()).is_err());
}
