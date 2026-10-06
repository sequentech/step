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

fn limits() -> ArchiveLimits {
    ArchiveLimits {
        members: 8,
        member_bytes: 64,
        total_bytes: 4096,
        nesting_depth: 2,
    }
}

fn read_within(
    bytes: &[u8],
    limits: ArchiveLimits,
) -> Result<Vec<Artifact>, Problem> {
    read_members(bytes, "package", &mut Budget::new(limits), |_| true)
}

fn refusal<T: std::fmt::Debug>(result: Result<T, Problem>) -> String {
    result.unwrap_err().id.unwrap_or_default()
}

#[test]
fn a_zip_with_more_members_than_allowed_is_refused() {
    let three = zip(&[
        artifact("a.csv", b"1"),
        artifact("b.csv", b"2"),
        artifact("c.csv", b"3"),
    ])
    .unwrap();
    assert_eq!(read_within(&three, limits()).unwrap().len(), 3);
    assert_eq!(
        refusal(read_within(
            &three,
            ArchiveLimits {
                members: 2,
                ..limits()
            }
        )),
        "package.too-many-members"
    );
}

#[test]
fn a_member_larger_than_allowed_is_refused() {
    let fits = zip(&[artifact("a.csv", &[b'a'; 64])]).unwrap();
    assert_eq!(read_within(&fits, limits()).unwrap()[0].bytes.len(), 64);

    let over = zip(&[artifact("a.csv", &[b'a'; 65])]).unwrap();
    let problem = read_within(&over, limits()).unwrap_err();
    assert_eq!(problem.id.as_deref(), Some("package.member-too-large"));
    assert_eq!(problem.details["file"], "a.csv");
    assert_eq!(problem.details["limit"], "64");
}

#[test]
fn a_member_that_understates_its_size_is_stopped_while_it_is_read() {
    let mut lying = zip(&[artifact("a.csv", &[b'a'; 100_000])]).unwrap();
    let directory = lying
        .windows(CENTRAL_DIRECTORY_HEADER.len())
        .rposition(|window| window == CENTRAL_DIRECTORY_HEADER)
        .unwrap();
    // The uncompressed size, in the directory entry.
    lying[directory + 24..directory + 28].copy_from_slice(&1u32.to_le_bytes());

    assert_eq!(
        refusal(read_within(&lying, limits())),
        "package.member-too-large"
    );
}

#[test]
fn members_larger_together_than_allowed_are_refused() {
    let two = zip(&[
        artifact("a.csv", &[b'a'; 40]),
        artifact("b.csv", &[b'b'; 40]),
    ])
    .unwrap();
    assert_eq!(read_within(&two, limits()).unwrap().len(), 2);
    assert_eq!(
        refusal(read_within(
            &two,
            ArchiveLimits {
                total_bytes: 79,
                ..limits()
            }
        )),
        "package.too-large"
    );
}

#[test]
fn a_nested_zips_members_count_against_the_packages_limits() {
    let inner = zip(&[
        artifact("a.csv", &[b'a'; 40]),
        artifact("b.csv", &[b'b'; 40]),
    ])
    .unwrap();
    let outer = zip(&[artifact("inner.zip", &inner)]).unwrap();
    let roomy = ArchiveLimits {
        member_bytes: 1024,
        ..limits()
    };
    let held = inner.len() as u64 + 80;

    let whole = expand(
        &outer,
        "package",
        ArchiveLimits {
            total_bytes: held,
            members: 3,
            ..roomy
        },
    )
    .unwrap();
    assert_eq!(whole.files[0].members.len(), 2);

    assert_eq!(
        refusal(expand(
            &outer,
            "package",
            ArchiveLimits {
                total_bytes: held - 1,
                ..roomy
            }
        )),
        "package.too-large"
    );
    assert_eq!(
        refusal(expand(
            &outer,
            "package",
            ArchiveLimits {
                members: 2,
                ..roomy
            }
        )),
        "package.too-many-members"
    );
}

#[test]
fn a_zip_nested_deeper_than_allowed_is_refused() {
    let leaf = zip(&[artifact("leaf.csv", b"1")]).unwrap();
    let middle = zip(&[artifact("inner.zip", &leaf)]).unwrap();
    let outer = zip(&[artifact("inner.zip", &middle)]).unwrap();
    let roomy = ArchiveLimits {
        member_bytes: 1024,
        ..limits()
    };

    assert!(expand(&outer, "package", roomy).is_ok());
    assert_eq!(
        refusal(expand(
            &outer,
            "package",
            ArchiveLimits {
                nesting_depth: 1,
                ..roomy
            }
        )),
        "package.nested-too-deep"
    );
}

#[test]
fn the_signature_members_are_read_without_the_delivery() {
    let manifest = manifest().to_bytes().unwrap();
    let mut delivery = delivery_members();
    delivery.push(artifact("voters.csv", &vec![b'v'; 1_000_000]));
    let signed =
        package("p.zip", &delivery, &manifest, b"signature", "chain").unwrap();

    // A budget the delivery doesn't fit in.
    let mut budget = Budget::new(ArchiveLimits {
        member_bytes: manifest.len() as u64,
        total_bytes: manifest.len() as u64 + 64,
        ..limits()
    });
    let names: Vec<String> = read_members(
        &signed.bytes,
        "package",
        &mut budget,
        is_signature_member,
    )
    .unwrap()
    .into_iter()
    .map(|member| member.name)
    .collect();
    assert_eq!(names, vec![MANIFEST_MEMBER, SIGNATURE_MEMBER, CHAIN_MEMBER]);

    let found = signature_members(&signed.bytes).unwrap();
    assert_eq!(found.manifest.as_deref(), Some(manifest.as_slice()));
    assert_eq!(found.signature.as_deref(), Some(&b"signature"[..]));
    assert_eq!(found.chain.as_deref(), Some(&b"chain"[..]));
    assert!(found.missing().is_empty());

    let rest = payload(&signed.bytes).unwrap();
    assert_eq!(rest.members, delivery);
    assert_eq!(rest.files, file_entries(&delivery).unwrap());
}

#[test]
fn a_signature_member_larger_than_allowed_is_refused() {
    let oversized = vec![b' '; MAX_SIGNATURE_MEMBER_BYTES as usize + 1];
    let signed =
        package("p.zip", &delivery_members(), &oversized, b"s", "c").unwrap();
    assert_eq!(
        refusal(signature_members(&signed.bytes)),
        "package.member-too-large"
    );
}

#[test]
fn a_file_is_known_to_be_signed_by_its_directory() {
    let signed =
        package("p.zip", &delivery_members(), b"{}", b"s", "c").unwrap();
    assert!(has_signature_members(&signed.bytes));
    assert!(!has_signature_members(&zip(&delivery_members()).unwrap()));
    assert!(!has_signature_members(b"not a zip"));

    let plain = signature_members(&zip(&delivery_members()).unwrap()).unwrap();
    assert_eq!(
        plain.missing(),
        vec![MANIFEST_MEMBER, SIGNATURE_MEMBER, CHAIN_MEMBER]
    );
}

#[test]
fn a_report_set_twice_is_drawn_with_either_design() {
    let mut signed = manifest();
    signed.content.reports[0].template_sha256 = Some(sha256_hex(b"event"));
    let mut second = signed.content.reports[0].clone();
    second.template_sha256 = Some(sha256_hex(b"election"));
    signed.content.reports.push(second);

    for template in ["event", "election"] {
        assert!(
            report_stamp(&signed, "m", "ELECTORAL_RESULTS", template).is_ok()
        );
    }
    let problem =
        report_stamp(&signed, "m", "ELECTORAL_RESULTS", "edited").unwrap_err();
    assert_eq!(
        problem.id.as_deref(),
        Some("package.report-template-changed")
    );

    signed.content.reports[1].template_sha256 = None;
    assert!(report_stamp(&signed, "m", "ELECTORAL_RESULTS", "edited").is_ok());
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
fn a_report_drawn_with_the_approved_template_is_stamped() {
    let mut signed = manifest();
    signed.content.reports[0].template_sha256 =
        Some(sha256_hex(b"<h1>Election Returns</h1>"));
    let stamp = report_stamp(
        &signed,
        "manifest-digest",
        "ELECTORAL_RESULTS",
        "<h1>Election Returns</h1>",
    )
    .unwrap();
    assert_eq!(stamp.revision, 8);
    assert_eq!(stamp.manifest_sha256, "manifest-digest");
    assert_eq!(
        stamp.template_sha256,
        sha256_hex(b"<h1>Election Returns</h1>")
    );
}

#[test]
fn a_report_drawn_with_another_template_is_refused() {
    let mut signed = manifest();
    signed.content.reports[0].template_sha256 = Some("00".repeat(32));
    let problem =
        report_stamp(&signed, "m", "ELECTORAL_RESULTS", "<h1>Edited</h1>")
            .unwrap_err();
    assert_eq!(
        problem.id.as_deref(),
        Some("package.report-template-changed")
    );
    assert_eq!(problem.details["expected"], "00".repeat(32));
}

#[test]
fn a_report_the_configuration_sets_no_design_for_is_stamped_with_its_template()
{
    let stamp =
        report_stamp(&manifest(), "m", "ACTIVITY_LOGS", "anything").unwrap();
    assert_eq!(stamp.template_sha256, sha256_hex(b"anything"));
}

#[test]
fn a_report_manifest_lists_each_file_with_its_digest() {
    let stamp = report_stamp(&manifest(), "m", "ACTIVITY_LOGS", "t").unwrap();
    let written = report_manifest(
        "ACTIVITY_LOGS",
        &stamp,
        &[
            artifact("report.pdf", b"%PDF"),
            artifact("report.csv", b"a,b\n"),
        ],
    );
    let paths: Vec<&str> =
        written.files.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(paths, vec!["report.csv", "report.pdf"]);
    assert_eq!(written.files[1].sha256, sha256_hex(b"%PDF"));
    assert_eq!(
        serde_json::to_value(&written).unwrap()["format"],
        "sequent.report-manifest/1"
    );
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
