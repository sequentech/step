// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Tests for [`super`].

use super::*;

fn artifact(name: &str, bytes: &[u8]) -> Artifact {
    Artifact {
        name: name.to_string(),
        bytes: bytes.to_vec(),
    }
}

fn delivery_members() -> Vec<Artifact> {
    let nested = zip(&[
        artifact("export_election_event-1.json", b"{}\n"),
        artifact("export_areas-1.csv", b"id,name\n"),
    ])
    .unwrap();
    vec![
        artifact("official_election_setup.zip", &nested),
        artifact("blueprint.json", b"{\"version\":4}"),
    ]
}

fn content() -> Content {
    Content {
        files: file_entries(&delivery_members()).unwrap(),
        ballot_designs: vec![BallotDesign {
            area: "North".to_string(),
            election: "officers".to_string(),
            version: 1,
            sha256: "ab".repeat(32),
        }],
        reports: vec![ReportSetting {
            report_type: "ELECTORAL_RESULTS".to_string(),
            formats: vec![ReportFormat::Pdf, ReportFormat::Xml],
            copies: 7,
            template: Some("comelec-er".to_string()),
            template_sha256: Some("cd".repeat(32)),
        }],
    }
}

fn manifest() -> Manifest {
    let content = content();
    Manifest {
        format: ManifestFormat::V1,
        configuration: ConfigurationRevision {
            external_id: "ov-2028".to_string(),
            name: "Overseas Voting 2028".to_string(),
            revision: 8,
            previous: Some(PreviousRevision {
                revision: 7,
                manifest_sha256: "ef".repeat(32),
            }),
        },
        content_sha256: content.sha256().unwrap(),
        content,
        approvals: Vec::new(),
        produced: Produced {
            at: "2027-01-10T08:00:00Z".to_string(),
            custodian: "Key Custodian".to_string(),
            key_label: "configuration-signing-2027".to_string(),
            producer: BTreeMap::from([(
                "sequent-core".to_string(),
                "test".to_string(),
            )]),
        },
        revocation_lists: Vec::new(),
    }
}

#[test]
fn every_member_is_listed_with_its_size_and_digest() {
    let entries = file_entries(&delivery_members()).unwrap();
    let paths: Vec<&str> = entries.iter().map(|e| e.path.as_str()).collect();
    assert_eq!(paths, vec!["blueprint.json", "official_election_setup.zip"]);
    assert_eq!(entries[0].size, 13);
    assert_eq!(entries[0].sha256, sha256_hex(b"{\"version\":4}"));
    assert!(entries[0].members.is_empty());
}

#[test]
fn a_nested_zip_lists_its_own_members() {
    let entries = file_entries(&delivery_members()).unwrap();
    let nested: Vec<(&str, u64)> = entries[1]
        .members
        .iter()
        .map(|e| (e.path.as_str(), e.size))
        .collect();
    assert_eq!(
        nested,
        vec![
            ("export_areas-1.csv", 8),
            ("export_election_event-1.json", 3)
        ]
    );
    assert_eq!(entries[1].members[1].sha256, sha256_hex(b"{}\n"));
}

#[test]
fn a_zip_that_names_a_file_twice_is_refused() {
    // The zip writer won't write a name twice, so rename the second member
    // in place: same length, in both its local header and the directory.
    let mut twice =
        zip(&[artifact("a.csv", b"1"), artifact("b.csv", b"2")]).unwrap();
    let mut at = 0;
    while let Some(found) = twice[at..].windows(5).position(|w| w == b"b.csv") {
        twice[at + found] = b'a';
        at += found + 5;
    }
    let problem = read_zip(&twice, "package").unwrap_err();
    assert_eq!(problem.id.as_deref(), Some("package.duplicate-member"));
}

#[test]
fn a_member_that_is_not_a_zip_but_is_named_one_is_refused() {
    let problem =
        file_entries(&[artifact("broken.zip", b"not a zip")]).unwrap_err();
    assert_eq!(problem.id.as_deref(), Some("package.unreadable-zip"));
    assert_eq!(problem.code, Code::Unreadable);
}

#[test]
fn the_content_digest_is_stable_and_follows_every_field() {
    assert_eq!(content().sha256().unwrap(), content().sha256().unwrap());

    let mut copies = content();
    copies.reports[0].copies = 6;
    assert_ne!(content().sha256().unwrap(), copies.sha256().unwrap());

    let mut design = content();
    design.ballot_designs[0].version = 2;
    assert_ne!(content().sha256().unwrap(), design.sha256().unwrap());

    let mut file = content();
    file.files[0].sha256 = "00".repeat(32);
    assert_ne!(content().sha256().unwrap(), file.sha256().unwrap());
}

#[test]
fn the_approval_payload_is_canonical_json() {
    assert_eq!(
        approval_payload("ov-2028", 8, "abc").unwrap(),
        r#"{"content_sha256":"abc","external_id":"ov-2028","format":"sequent.configuration-approval/1","revision":8}"#
    );
}

#[test]
fn the_approval_code_is_the_payloads_first_forty_bits() {
    let payload = approval_payload("ov-2028", 8, "abc").unwrap();
    let code = approval_code(&payload);
    assert_eq!(code.len(), 9);
    assert_eq!(&code[4..5], "-");
    assert!(code
        .chars()
        .filter(|c| *c != '-')
        .all(|c| "0123456789ABCDEFGHJKMNPQRSTVWXYZ".contains(c)));

    let digest = Sha256::digest(payload.as_bytes());
    let mut leading = [0u8; 5];
    leading.copy_from_slice(&digest[..5]);
    assert_eq!(code, crockford_code(leading));
    assert_ne!(
        code,
        approval_code(&approval_payload("ov-2028", 9, "abc").unwrap())
    );
}

#[test]
fn the_manifest_is_written_once_with_a_trailing_newline() {
    let bytes = manifest().to_bytes().unwrap();
    assert_eq!(bytes.last(), Some(&b'\n'));
    let back: Manifest = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(back, manifest());
    assert_eq!(
        declared_format(&bytes).as_deref(),
        Some("sequent.configuration-manifest/1")
    );
}

#[test]
fn an_unknown_format_is_not_read_as_this_one() {
    let mut value = serde_json::to_value(manifest()).unwrap();
    value["format"] = json!("sequent.configuration-manifest/2");
    let bytes = serde_json::to_vec(&value).unwrap();
    assert!(serde_json::from_slice::<Manifest>(&bytes).is_err());
    assert_eq!(
        declared_format(&bytes).as_deref(),
        Some("sequent.configuration-manifest/2")
    );
}

#[test]
fn a_package_opens_into_its_members_and_its_signature() {
    let manifest = manifest().to_bytes().unwrap();
    let signed = package(
        "ov-2028-r8.zip",
        &delivery_members(),
        &manifest,
        b"signature",
        "-----BEGIN CERTIFICATE-----\n",
    )
    .unwrap();
    assert_eq!(signed.name, "ov-2028-r8.zip");

    let opened = open_package(&signed.bytes).unwrap();
    assert_eq!(opened.members, delivery_members());
    assert_eq!(opened.manifest.as_deref(), Some(manifest.as_slice()));
    assert_eq!(opened.signature.as_deref(), Some(&b"signature"[..]));
    assert!(opened.is_signed());
}

#[test]
fn a_plain_delivery_opens_unsigned() {
    let delivery = zip(&delivery_members()).unwrap();
    let opened = open_package(&delivery).unwrap();
    assert!(!opened.is_signed());
    assert_eq!(opened.members, delivery_members());
}

#[test]
fn the_same_content_has_no_changes() {
    assert_eq!(changes(&content(), &content()), Vec::new());
}

#[test]
fn a_change_inside_a_nested_zip_is_named_inside_it() {
    let mut members = delivery_members();
    members[0].bytes = zip(&[
        artifact("export_election_event-1.json", b"{\"changed\":1}\n"),
        artifact("export_areas-1.csv", b"id,name\n"),
        artifact("export_reports-1.csv", b"id\n"),
    ])
    .unwrap();
    let mut after = content();
    after.files = file_entries(&members).unwrap();

    assert_eq!(
        changes(&content(), &after),
        vec![
            Change {
                subject: ChangeSubject::File,
                kind: ChangeKind::Changed,
                name:
                    "official_election_setup.zip/export_election_event-1.json"
                        .to_string(),
                versions: None,
            },
            Change {
                subject: ChangeSubject::File,
                kind: ChangeKind::Added,
                name: "official_election_setup.zip/export_reports-1.csv"
                    .to_string(),
                versions: None,
            },
        ]
    );
}

#[test]
fn designs_and_reports_say_what_changed() {
    let mut after = content();
    after.ballot_designs[0].version = 2;
    after.ballot_designs[0].sha256 = "ff".repeat(32);
    after.ballot_designs.push(BallotDesign {
        area: "South".to_string(),
        election: "officers".to_string(),
        version: 1,
        sha256: "ee".repeat(32),
    });
    after.reports[0].copies = 3;
    after.reports.push(ReportSetting {
        report_type: "ACTIVITY_LOGS".to_string(),
        formats: vec![ReportFormat::Csv],
        copies: 1,
        template: None,
        template_sha256: None,
    });

    let summary: Vec<(ChangeSubject, ChangeKind, String, Option<(u32, u32)>)> =
        changes(&content(), &after)
            .into_iter()
            .map(|change| {
                (change.subject, change.kind, change.name, change.versions)
            })
            .collect();
    assert_eq!(
        summary,
        vec![
            (
                ChangeSubject::BallotDesign,
                ChangeKind::Changed,
                "North / officers".to_string(),
                Some((1, 2))
            ),
            (
                ChangeSubject::BallotDesign,
                ChangeKind::Added,
                "South / officers".to_string(),
                None
            ),
            (
                ChangeSubject::Report,
                ChangeKind::Changed,
                "ELECTORAL_RESULTS".to_string(),
                None
            ),
            (
                ChangeSubject::Report,
                ChangeKind::Added,
                "ACTIVITY_LOGS".to_string(),
                None
            ),
        ]
    );
    let removed = changes(&after, &content());
    assert!(removed
        .iter()
        .any(|change| change.kind == ChangeKind::Removed
            && change.subject == ChangeSubject::Report));
}

#[test]
fn packaging_the_same_members_gives_the_same_bytes() {
    let manifest = manifest().to_bytes().unwrap();
    let make = || {
        package("p.zip", &delivery_members(), &manifest, b"s", "c")
            .unwrap()
            .bytes
    };
    assert_eq!(make(), make());
}
