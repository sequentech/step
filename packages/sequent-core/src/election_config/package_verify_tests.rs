// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Tests for [`super`], against a test PKI: the organization's root, which
//! issues the package signing certificate, and a staff root, which issues
//! the approvers'.

use std::collections::BTreeMap;

use rcgen::{
    date_time_ymd, BasicConstraints, CertificateParams,
    CertificateRevocationListParams, CertifiedIssuer, DnType,
    ExtendedKeyUsagePurpose, IsCa, KeyIdMethod, KeyPair, KeyUsagePurpose,
    RevokedCertParams, SerialNumber, PKCS_ECDSA_P256_SHA256,
};
use ring::rand::SystemRandom;
use ring::signature::{EcdsaKeyPair, ECDSA_P256_SHA256_ASN1_SIGNING};

use super::*;
use crate::election_config::archive::zip;
use crate::election_config::design::BallotDesign;
use crate::election_config::manifest::{
    approval_payload, file_entries, package, ConfigurationRevision, Content,
    Produced, ReportFormat, ReportSetting, CHAIN_MEMBER, MANIFEST_MEMBER,
    SIGNATURE_MEMBER,
};

type Ca = CertifiedIssuer<'static, KeyPair>;

fn ca(name: &str) -> Ca {
    let mut params = CertificateParams::new(Vec::<String>::new()).unwrap();
    params.distinguished_name.push(DnType::CommonName, name);
    params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    params.key_usages = vec![
        KeyUsagePurpose::KeyCertSign,
        KeyUsagePurpose::CrlSign,
        KeyUsagePurpose::DigitalSignature,
    ];
    params.not_before = date_time_ymd(2020, 1, 1);
    params.not_after = date_time_ymd(2040, 1, 1);
    params.key_identifier_method = KeyIdMethod::Sha256;
    CertifiedIssuer::self_signed(
        params,
        KeyPair::generate_for(&PKCS_ECDSA_P256_SHA256).unwrap(),
    )
    .unwrap()
}

/// A person's or a key's signing certificate.
struct Signer {
    certificate_pem: String,
    chain_pem: String,
    key: KeyPair,
    serial: u64,
}

struct LeafOptions {
    serial: u64,
    key_usages: Vec<KeyUsagePurpose>,
    extended: Vec<ExtendedKeyUsagePurpose>,
    not_after: (i32, u8, u8),
}

impl Default for LeafOptions {
    fn default() -> Self {
        LeafOptions {
            serial: 1,
            key_usages: vec![KeyUsagePurpose::DigitalSignature],
            extended: Vec::new(),
            not_after: (2035, 1, 1),
        }
    }
}

fn leaf(name: &str, issuer: &Ca, options: LeafOptions) -> Signer {
    let mut params = CertificateParams::new(Vec::<String>::new()).unwrap();
    params.distinguished_name.push(DnType::CommonName, name);
    params.serial_number = Some(SerialNumber::from(options.serial));
    params.key_usages = options.key_usages;
    params.extended_key_usages = options.extended;
    params.not_before = date_time_ymd(2025, 1, 1);
    let (year, month, day) = options.not_after;
    params.not_after = date_time_ymd(year, month, day);
    let key = KeyPair::generate_for(&PKCS_ECDSA_P256_SHA256).unwrap();
    let certificate = params.signed_by(&key, issuer).unwrap();
    Signer {
        certificate_pem: certificate.pem(),
        chain_pem: format!("{}{}", certificate.pem(), issuer.pem()),
        key,
        serial: options.serial,
    }
}

fn sign(key: &KeyPair, message: &[u8]) -> Vec<u8> {
    let rng = SystemRandom::new();
    let pair = EcdsaKeyPair::from_pkcs8(
        &ECDSA_P256_SHA256_ASN1_SIGNING,
        &key.serialize_der(),
        &rng,
    )
    .unwrap();
    pair.sign(&rng, message).unwrap().as_ref().to_vec()
}

fn crl(issuer: &Ca, revoked: &[u64]) -> String {
    CertificateRevocationListParams {
        this_update: date_time_ymd(2026, 6, 1),
        next_update: date_time_ymd(2026, 7, 1),
        crl_number: SerialNumber::from(1u64),
        issuing_distribution_point: None,
        revoked_certs: revoked
            .iter()
            .map(|serial| RevokedCertParams {
                serial_number: SerialNumber::from(*serial),
                revocation_time: date_time_ymd(2026, 6, 1),
                reason_code: Some(rcgen::RevocationReason::KeyCompromise),
                invalidity_date: None,
            })
            .collect(),
        key_identifier_method: KeyIdMethod::Sha256,
    }
    .signed_by(issuer)
    .unwrap()
    .pem()
    .unwrap()
}

struct World {
    root: Ca,
    staff_root: Ca,
    key: Signer,
    manager: Signer,
    officer: Signer,
}

fn world() -> World {
    let root = ca("Organization Root");
    let staff_root = ca("Staff Root");
    let key = leaf("Configuration Signing Key", &root, LeafOptions::default());
    let manager = leaf(
        "Configuration Manager",
        &staff_root,
        LeafOptions {
            serial: 10,
            key_usages: vec![KeyUsagePurpose::ContentCommitment],
            extended: vec![ExtendedKeyUsagePurpose::ClientAuth],
            ..LeafOptions::default()
        },
    );
    let officer = leaf(
        "Security Officer",
        &staff_root,
        LeafOptions {
            serial: 11,
            ..LeafOptions::default()
        },
    );
    World {
        root,
        staff_root,
        key,
        manager,
        officer,
    }
}

fn trust(world: &World) -> PackageTrust {
    PackageTrust::from_pem(&world.root.pem(), &world.staff_root.pem(), &[], 2)
        .unwrap()
}

fn members() -> Vec<Artifact> {
    let nested = zip(&[
        Artifact {
            name: "export_election_event-1.json".to_string(),
            bytes: b"{}\n".to_vec(),
        },
        Artifact {
            name: "export_areas-1.csv".to_string(),
            bytes: b"id,name\n".to_vec(),
        },
    ])
    .unwrap();
    vec![
        Artifact {
            name: "official_election_setup.zip".to_string(),
            bytes: nested,
        },
        Artifact {
            name: "blueprint.json".to_string(),
            bytes: b"{\"version\":4}".to_vec(),
        },
    ]
}

fn unsigned_manifest(revision: u64) -> Manifest {
    let content = Content {
        files: file_entries(&members()).unwrap(),
        ballot_designs: vec![BallotDesign {
            area: "North".to_string(),
            election: "officers".to_string(),
            version: 1,
            sha256: "ab".repeat(32),
        }],
        reports: vec![ReportSetting {
            report_type: "ELECTORAL_RESULTS".to_string(),
            formats: vec![ReportFormat::Pdf],
            copies: 7,
            template: None,
            template_sha256: None,
        }],
    };
    Manifest {
        format: ManifestFormat::V1,
        configuration: ConfigurationRevision {
            external_id: "ov-2028".to_string(),
            name: "Overseas Voting 2028".to_string(),
            revision,
            previous: None,
        },
        content_sha256: content.sha256().unwrap(),
        content,
        approvals: Vec::new(),
        produced: Produced {
            at: "2026-06-15T08:00:00Z".to_string(),
            custodian: "Key Custodian".to_string(),
            key_label: "configuration-signing".to_string(),
            producer: BTreeMap::new(),
        },
        revocation_lists: Vec::new(),
    }
}

fn approve(manifest: &mut Manifest, signer: &Signer, name: &str) {
    let payload = approval_payload(
        &manifest.configuration.external_id,
        manifest.configuration.revision,
        &manifest.content_sha256,
    )
    .unwrap();
    manifest.approvals.push(Approval {
        name: name.to_string(),
        role: "approver".to_string(),
        signed_at: "2026-06-14T10:00:00Z".to_string(),
        algorithm: SignatureAlgorithm::EcdsaP256Sha256,
        certificate_chain: signer.chain_pem.clone(),
        signature: STANDARD.encode(sign(&signer.key, payload.as_bytes())),
    });
}

fn approved(world: &World, revision: u64) -> Manifest {
    let mut manifest = unsigned_manifest(revision);
    approve(&mut manifest, &world.manager, "Configuration Manager");
    approve(&mut manifest, &world.officer, "Security Officer");
    manifest
}

fn signed(manifest: &Manifest, key: &Signer, members: &[Artifact]) -> Vec<u8> {
    let bytes = manifest.to_bytes().unwrap();
    package(
        "p.zip",
        members,
        &bytes,
        &sign(&key.key, &bytes),
        &key.chain_pem,
    )
    .unwrap()
    .bytes
}

fn ids(report: &Report) -> Vec<String> {
    report
        .errors()
        .filter_map(|problem| problem.id.clone())
        .collect()
}

fn refused(bytes: &[u8], trust: &PackageTrust) -> Vec<String> {
    match verify_package(bytes, trust) {
        Ok(_) => panic!("expected the package to be refused"),
        Err(report) => ids(&report),
    }
}

#[test]
fn a_signed_and_approved_package_verifies() {
    let world = world();
    let manifest = approved(&world, 8);
    let bytes = signed(&manifest, &world.key, &members());

    let verified = verify_package(&bytes, &trust(&world)).unwrap();
    assert_eq!(verified.manifest, manifest);
    assert_eq!(verified.members, members());
    assert_eq!(verified.signer.subject, "CN=Configuration Signing Key");
    assert_eq!(verified.signer.serial, "01");
    assert_eq!(verified.approvers.len(), 2);
    assert_eq!(
        verified.manifest_sha256,
        sha256_hex(&manifest.to_bytes().unwrap())
    );
}

#[test]
fn a_delivery_without_a_signature_is_refused() {
    let world = world();
    let bytes = zip(&members()).unwrap();
    assert_eq!(refused(&bytes, &trust(&world)), vec!["package.unsigned"]);
}

#[test]
fn a_manifest_without_its_signature_is_refused() {
    let world = world();
    let mut all = members();
    all.push(Artifact {
        name: MANIFEST_MEMBER.to_string(),
        bytes: approved(&world, 8).to_bytes().unwrap(),
    });
    let report =
        verify_package(&zip(&all).unwrap(), &trust(&world)).unwrap_err();
    let problem = report.errors().next().unwrap();
    assert_eq!(problem.id.as_deref(), Some("package.unsigned"));
    assert_eq!(
        problem.details.get("missing").map(String::as_str),
        Some("manifest.sig, manifest.chain.pem")
    );
}

#[test]
fn a_file_changed_after_signing_is_named_with_both_digests() {
    let world = world();
    let manifest = approved(&world, 8);
    let mut changed = members();
    changed[1].bytes = b"{\"version\":5}".to_vec();
    let bytes = signed(&manifest, &world.key, &changed);

    let report = verify_package(&bytes, &trust(&world)).unwrap_err();
    let problem = report.errors().next().unwrap();
    assert_eq!(problem.id.as_deref(), Some("package.file-changed"));
    assert_eq!(problem.details["file"], "blueprint.json");
    assert_eq!(problem.details["expected"], sha256_hex(b"{\"version\":4}"));
    assert_eq!(problem.details["actual"], sha256_hex(b"{\"version\":5}"));
}

#[test]
fn a_changed_member_of_a_nested_zip_is_named_inside_it() {
    let world = world();
    let manifest = approved(&world, 8);
    let mut changed = members();
    changed[0].bytes = zip(&[
        Artifact {
            name: "export_election_event-1.json".to_string(),
            bytes: b"{\"tampered\":true}\n".to_vec(),
        },
        Artifact {
            name: "export_areas-1.csv".to_string(),
            bytes: b"id,name\n".to_vec(),
        },
    ])
    .unwrap();
    let bytes = signed(&manifest, &world.key, &changed);

    let report = verify_package(&bytes, &trust(&world)).unwrap_err();
    let files: Vec<&str> = report
        .errors()
        .map(|problem| problem.details["file"].as_str())
        .collect();
    assert_eq!(
        files,
        vec![
            "official_election_setup.zip",
            "official_election_setup.zip/export_election_event-1.json"
        ]
    );
}

#[test]
fn a_missing_file_is_refused() {
    let world = world();
    let manifest = approved(&world, 8);
    let bytes = signed(&manifest, &world.key, &members()[..1]);
    assert_eq!(
        refused(&bytes, &trust(&world)),
        vec!["package.file-missing"]
    );
}

#[test]
fn an_extra_file_is_refused() {
    let world = world();
    let manifest = approved(&world, 8);
    let mut more = members();
    more.push(Artifact {
        name: "admin_users.csv".to_string(),
        bytes: b"username,password\n".to_vec(),
    });
    let bytes = signed(&manifest, &world.key, &more);
    assert_eq!(refused(&bytes, &trust(&world)), vec!["package.file-extra"]);
}

#[test]
fn a_changed_manifest_no_longer_matches_its_signature() {
    let world = world();
    let manifest = approved(&world, 8);
    let original = manifest.to_bytes().unwrap();
    let signature = sign(&world.key.key, &original);
    let mut tampered = manifest.clone();
    tampered.content.reports[0].copies = 1;
    tampered.content_sha256 = tampered.content.sha256().unwrap();
    let bytes = package(
        "p.zip",
        &members(),
        &tampered.to_bytes().unwrap(),
        &signature,
        &world.key.chain_pem,
    )
    .unwrap()
    .bytes;
    assert_eq!(
        refused(&bytes, &trust(&world)),
        vec!["package.bad-signature"]
    );
}

#[test]
fn a_signature_by_a_key_the_organization_does_not_trust_is_refused() {
    let world = world();
    let elsewhere = ca("Somebody Else");
    let key = leaf(
        "Configuration Signing Key",
        &elsewhere,
        LeafOptions::default(),
    );
    let bytes = signed(&approved(&world, 8), &key, &members());
    assert_eq!(
        refused(&bytes, &trust(&world)),
        vec!["package.untrusted-signer"]
    );
}

#[test]
fn a_chain_naming_a_trusted_issuer_does_not_make_another_key_trusted() {
    // The chain file is the package's word, not the verifier's: a key signed
    // by an untrusted CA stays untrusted with the trusted root appended.
    let world = world();
    let elsewhere = ca("Somebody Else");
    let mut key = leaf(
        "Configuration Signing Key",
        &elsewhere,
        LeafOptions::default(),
    );
    key.chain_pem = format!("{}{}", key.certificate_pem, world.root.pem());
    let bytes = signed(&approved(&world, 8), &key, &members());
    assert_eq!(
        refused(&bytes, &trust(&world)),
        vec!["package.untrusted-signer"]
    );
}

#[test]
fn a_revoked_signing_key_is_refused() {
    let world = world();
    let bytes = signed(&approved(&world, 8), &world.key, &members());
    let mut trust = trust(&world);
    trust.revocation_lists =
        crls_from_pem(&crl(&world.root, &[world.key.serial])).unwrap();
    assert_eq!(refused(&bytes, &trust), vec!["package.revoked-signer"]);
}

#[test]
fn a_revocation_list_carried_in_a_package_is_applied() {
    let world = world();
    let mut manifest = approved(&world, 8);
    manifest.revocation_lists = vec![crl(&world.root, &[world.key.serial])];
    let bytes = signed(&manifest, &world.key, &members());
    assert_eq!(
        refused(&bytes, &trust(&world)),
        vec!["package.revoked-signer"]
    );
}

#[test]
fn a_retired_key_that_was_not_revoked_still_verifies() {
    let world = world();
    let retired = leaf(
        "Configuration Signing Key 2025",
        &world.root,
        LeafOptions {
            serial: 2,
            ..LeafOptions::default()
        },
    );
    let mut trust = trust(&world);
    trust.revocation_lists = crls_from_pem(&crl(&world.root, &[3])).unwrap();
    let bytes = signed(&approved(&world, 8), &retired, &members());
    assert!(verify_package(&bytes, &trust).is_ok());
}

#[test]
fn a_signing_key_whose_certificate_is_not_for_signing_is_refused() {
    let world = world();
    let key = leaf(
        "Configuration Signing Key",
        &world.root,
        LeafOptions {
            key_usages: vec![KeyUsagePurpose::KeyEncipherment],
            ..LeafOptions::default()
        },
    );
    let bytes = signed(&approved(&world, 8), &key, &members());
    assert_eq!(
        refused(&bytes, &trust(&world)),
        vec!["package.signer-key-usage"]
    );
}

#[test]
fn a_package_signed_after_the_key_expired_is_refused() {
    let world = world();
    let key = leaf(
        "Configuration Signing Key",
        &world.root,
        LeafOptions {
            not_after: (2026, 1, 1),
            ..LeafOptions::default()
        },
    );
    let bytes = signed(&approved(&world, 8), &key, &members());
    assert_eq!(
        refused(&bytes, &trust(&world)),
        vec!["package.untrusted-signer"]
    );
}

fn at(rfc3339: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(rfc3339)
        .unwrap()
        .with_timezone(&Utc)
}

fn refused_at(bytes: &[u8], trust: &PackageTrust, now: &str) -> Vec<String> {
    match verify_package_at(bytes, trust, at(now)) {
        Ok(_) => panic!("expected the package to be refused"),
        Err(report) => ids(&report),
    }
}

#[test]
fn a_package_dated_after_the_verifiers_time_is_refused() {
    // Signed at 08:00 by its own account.
    let world = world();
    let bytes = signed(&approved(&world, 8), &world.key, &members());
    let trust = trust(&world);

    assert!(
        verify_package_at(&bytes, &trust, at("2026-06-15T08:00:00Z")).is_ok()
    );
    assert_eq!(
        refused_at(&bytes, &trust, "2026-06-15T07:00:00Z"),
        vec!["package.signed-in-the-future"]
    );
}

#[test]
fn a_signing_clock_a_little_ahead_of_the_verifiers_is_allowed() {
    let world = world();
    let bytes = signed(&approved(&world, 8), &world.key, &members());
    let trust = trust(&world);

    assert!(
        verify_package_at(&bytes, &trust, at("2026-06-15T07:55:00Z")).is_ok()
    );
    assert_eq!(
        refused_at(&bytes, &trust, "2026-06-15T07:54:59Z"),
        vec!["package.signed-in-the-future"]
    );
}

#[test]
fn an_approval_dated_after_the_verifiers_time_does_not_count() {
    let world = world();
    let mut manifest = approved(&world, 8);
    manifest.approvals[1].signed_at = "2026-06-20T10:00:00Z".to_string();
    let bytes = signed(&manifest, &world.key, &members());

    let report =
        verify_package_at(&bytes, &trust(&world), at("2026-06-15T09:00:00Z"))
            .unwrap_err();
    assert_eq!(
        ids(&report),
        vec!["package.approval-invalid", "package.too-few-approvals"]
    );
    let invalid = report.errors().next().unwrap();
    assert!(invalid.details["reason"].contains("after now"));
}

#[test]
fn a_key_that_expired_after_it_signed_still_verifies() {
    let world = world();
    let retired = leaf(
        "Configuration Signing Key 2026",
        &world.root,
        LeafOptions {
            serial: 2,
            not_after: (2027, 1, 1),
            ..LeafOptions::default()
        },
    );
    let bytes = signed(&approved(&world, 8), &retired, &members());
    assert!(verify_package_at(
        &bytes,
        &trust(&world),
        at("2031-03-01T00:00:00Z")
    )
    .is_ok());
}

#[test]
fn a_key_revoked_after_it_signed_is_refused_whenever_it_is_checked() {
    let world = world();
    let bytes = signed(&approved(&world, 8), &world.key, &members());
    let mut trust = trust(&world);
    trust.revocation_lists =
        crls_from_pem(&crl(&world.root, &[world.key.serial])).unwrap();
    assert_eq!(
        refused_at(&bytes, &trust, "2031-03-01T00:00:00Z"),
        vec!["package.revoked-signer"]
    );
}

#[test]
fn an_importer_with_a_clock_refuses_a_package_from_the_future() {
    let world = world();
    let bytes = signed(&approved(&world, 8), &world.key, &members());
    let settings = settings(&world, ConfigurationSigningPolicy::Optional);

    assert!(matches!(
        admit_at(
            &bytes,
            Some(&settings),
            Vec::new(),
            at("2026-06-15T08:00:00Z")
        ),
        Ok(Admission::Verified(_))
    ));
    let report = admit_at(
        &bytes,
        Some(&settings),
        Vec::new(),
        at("2026-06-01T00:00:00Z"),
    )
    .unwrap_err();
    assert_eq!(ids(&report), vec!["package.signed-in-the-future"]);
}

#[test]
fn the_delivery_is_not_expanded_until_the_signature_holds() {
    // A member that can't be expanded is only found by expanding it.
    let world = world();
    let mut broken = members();
    broken.push(Artifact {
        name: "broken.zip".to_string(),
        bytes: b"not a zip".to_vec(),
    });
    let manifest = approved(&world, 8).to_bytes().unwrap();
    let forged = package(
        "p.zip",
        &broken,
        &manifest,
        b"not a signature",
        &world.key.chain_pem,
    )
    .unwrap()
    .bytes;
    assert_eq!(
        refused(&forged, &trust(&world)),
        vec!["package.bad-signature"]
    );

    let signed = signed(&approved(&world, 8), &world.key, &broken);
    assert_eq!(
        refused(&signed, &trust(&world)),
        vec!["package.unreadable-zip"]
    );
}

#[test]
fn too_few_approvals_are_refused() {
    let world = world();
    let mut manifest = unsigned_manifest(8);
    approve(&mut manifest, &world.manager, "Configuration Manager");
    let bytes = signed(&manifest, &world.key, &members());
    let report = verify_package(&bytes, &trust(&world)).unwrap_err();
    let problem = report.errors().next().unwrap();
    assert_eq!(problem.id.as_deref(), Some("package.too-few-approvals"));
    assert_eq!(problem.details["count"], "1");
    assert_eq!(problem.details["required"], "2");
}

#[test]
fn one_person_approving_twice_counts_once() {
    let world = world();
    let mut manifest = unsigned_manifest(8);
    approve(&mut manifest, &world.manager, "Configuration Manager");
    approve(&mut manifest, &world.manager, "Configuration Manager");
    let bytes = signed(&manifest, &world.key, &members());
    let report = verify_package(&bytes, &trust(&world)).unwrap_err();
    assert_eq!(ids(&report), vec!["package.too-few-approvals"]);
    assert!(report
        .warnings()
        .any(|problem| problem.id.as_deref()
            == Some("package.approval-repeated")));
}

#[test]
fn an_approval_of_another_revision_does_not_count() {
    let world = world();
    let mut manifest = approved(&world, 8);
    let mut other = unsigned_manifest(7);
    approve(&mut other, &world.officer, "Security Officer");
    manifest.approvals[1] = other.approvals[0].clone();
    let bytes = signed(&manifest, &world.key, &members());
    assert_eq!(
        refused(&bytes, &trust(&world)),
        vec!["package.approval-invalid", "package.too-few-approvals"]
    );
}

#[test]
fn an_approval_from_outside_the_staff_roots_does_not_count() {
    let world = world();
    let mut manifest = unsigned_manifest(8);
    approve(&mut manifest, &world.manager, "Configuration Manager");
    // Issued by the package root, which is not trusted for staff.
    let stranger = leaf(
        "Stranger",
        &world.root,
        LeafOptions {
            serial: 20,
            ..LeafOptions::default()
        },
    );
    approve(&mut manifest, &stranger, "Stranger");
    let bytes = signed(&manifest, &world.key, &members());
    let report = verify_package(&bytes, &trust(&world)).unwrap_err();
    let invalid = report.errors().next().unwrap();
    assert_eq!(invalid.id.as_deref(), Some("package.approval-invalid"));
    assert!(invalid.details["reason"].contains("not issued by a trusted"));
}

#[test]
fn an_approval_by_a_revoked_certificate_does_not_count() {
    let world = world();
    let bytes = signed(&approved(&world, 8), &world.key, &members());
    let mut trust = trust(&world);
    trust.revocation_lists =
        crls_from_pem(&crl(&world.staff_root, &[world.officer.serial]))
            .unwrap();
    assert_eq!(
        refused(&bytes, &trust),
        vec!["package.approval-invalid", "package.too-few-approvals"]
    );
}

#[test]
fn no_approvals_are_needed_where_none_are_required() {
    let world = world();
    let bytes = signed(&unsigned_manifest(1), &world.key, &members());
    let mut trust = trust(&world);
    trust.required_approvals = 0;
    assert!(verify_package(&bytes, &trust).is_ok());
}

#[test]
fn a_content_digest_that_does_not_match_the_content_is_refused() {
    let world = world();
    let mut manifest = approved(&world, 8);
    manifest.content.reports[0].copies = 6;
    let bytes = signed(&manifest, &world.key, &members());
    let errors = refused(&bytes, &trust(&world));
    assert_eq!(errors[0], "package.content-digest");
}

#[test]
fn a_manifest_in_an_unknown_format_is_refused_as_such() {
    let world = world();
    let mut value = serde_json::to_value(approved(&world, 8)).unwrap();
    value["format"] = serde_json::json!("sequent.configuration-manifest/2");
    let manifest = serde_json::to_vec(&value).unwrap();
    let bytes = package(
        "p.zip",
        &members(),
        &manifest,
        &sign(&world.key.key, &manifest),
        &world.key.chain_pem,
    )
    .unwrap()
    .bytes;
    let report = verify_package(&bytes, &trust(&world)).unwrap_err();
    let problem = report.errors().next().unwrap();
    assert_eq!(problem.id.as_deref(), Some("package.unknown-format"));
    assert_eq!(problem.code, Code::IncompatibleVersion);
}

#[test]
fn nothing_is_trusted_without_roots() {
    let world = world();
    let bytes = signed(&approved(&world, 8), &world.key, &members());
    assert_eq!(
        refused(&bytes, &PackageTrust::default()),
        vec!["package.untrusted-signer"]
    );
}

#[test]
fn the_signature_verifies_with_the_public_key_alone() {
    // What `openssl dgst -sha256 -verify` does with the certificate's key.
    let world = world();
    let manifest = approved(&world, 8).to_bytes().unwrap();
    let signature = sign(&world.key.key, &manifest);
    let key = ring::signature::UnparsedPublicKey::new(
        &ring::signature::ECDSA_P256_SHA256_ASN1,
        world.key.key.public_key_raw(),
    );
    assert!(key.verify(&manifest, &signature).is_ok());
}

fn verified(world: &World, revision: u64) -> VerifiedPackage {
    let bytes = signed(&approved(world, revision), &world.key, &members());
    verify_package(&bytes, &trust(world)).unwrap()
}

#[test]
fn the_first_package_of_a_configuration_is_new() {
    let world = world();
    assert_eq!(freshness(&verified(&world, 1), None), Ok(Freshness::New));
}

#[test]
fn a_newer_revision_is_new() {
    let world = world();
    let last = LastImport {
        revision: 7,
        manifest_sha256: "00".repeat(32),
    };
    assert_eq!(
        freshness(&verified(&world, 8), Some(&last)),
        Ok(Freshness::New)
    );
}

#[test]
fn the_same_package_again_is_already_imported() {
    let world = world();
    let package = verified(&world, 8);
    let last = LastImport {
        revision: 8,
        manifest_sha256: package.manifest_sha256.clone(),
    };
    assert_eq!(
        freshness(&package, Some(&last)),
        Ok(Freshness::AlreadyImported)
    );
}

#[test]
fn an_older_or_replayed_revision_is_a_rollback() {
    let world = world();
    for revision in [7, 8] {
        let last = LastImport {
            revision: 8,
            manifest_sha256: "00".repeat(32),
        };
        let problem =
            freshness(&verified(&world, revision), Some(&last)).unwrap_err();
        assert_eq!(problem.id.as_deref(), Some("package.rollback"));
        assert_eq!(problem.code, Code::Rollback);
    }
}

#[test]
fn a_signing_certificate_is_checked_before_it_is_used() {
    let world = world();
    let trust = trust(&world);
    let identity = verify_signing_certificate(
        &world.key.chain_pem,
        &trust.package_roots,
        &[],
        "2026-06-15T08:00:00Z",
    )
    .unwrap();
    assert_eq!(identity.subject, "CN=Configuration Signing Key");

    let problem = verify_signing_certificate(
        &world.manager.chain_pem,
        &trust.package_roots,
        &[],
        "2026-06-15T08:00:00Z",
    )
    .unwrap_err();
    assert_eq!(problem.id.as_deref(), Some("package.untrusted-signer"));
}

fn settings(
    world: &World,
    policy: ConfigurationSigningPolicy,
) -> ConfigurationSigning {
    ConfigurationSigning {
        policy,
        package_roots: world.root.pem(),
        staff_roots: world.staff_root.pem(),
        required_approvals: 2,
    }
}

#[test]
fn the_setting_reads_from_the_tenants_json() {
    let read: ConfigurationSigning =
        serde_json::from_value(serde_json::json!({
            "policy": "required",
            "package_roots": "-----BEGIN CERTIFICATE-----",
            "required_approvals": 2
        }))
        .unwrap();
    assert_eq!(read.policy, ConfigurationSigningPolicy::Required);
    assert_eq!(read.staff_roots, "");
    let empty: ConfigurationSigning =
        serde_json::from_value(serde_json::json!({})).unwrap();
    assert_eq!(empty.policy, ConfigurationSigningPolicy::Optional);
}

#[test]
fn an_unsigned_file_imports_as_before_unless_signatures_are_required() {
    let world = world();
    let plain = zip(&members()).unwrap();
    assert!(matches!(
        admit(&plain, None, Vec::new()),
        Ok(Admission::Unsigned)
    ));
    assert!(matches!(
        admit(
            &plain,
            Some(&settings(&world, ConfigurationSigningPolicy::Optional)),
            Vec::new()
        ),
        Ok(Admission::Unsigned)
    ));
    let report = admit(
        &plain,
        Some(&settings(&world, ConfigurationSigningPolicy::Required)),
        Vec::new(),
    )
    .unwrap_err();
    assert_eq!(ids(&report), vec!["package.unsigned"]);
    let not_a_zip = admit(
        b"{}",
        Some(&settings(&world, ConfigurationSigningPolicy::Required)),
        Vec::new(),
    )
    .unwrap_err();
    assert_eq!(ids(&not_a_zip), vec!["package.unsigned"]);
}

#[test]
fn a_signed_package_is_always_checked() {
    let world = world();
    let bytes = signed(&approved(&world, 8), &world.key, &members());
    let Ok(Admission::Verified(verified)) = admit(
        &bytes,
        Some(&settings(&world, ConfigurationSigningPolicy::Optional)),
        Vec::new(),
    ) else {
        panic!("a good package is admitted");
    };
    assert_eq!(
        importable_member(&verified).unwrap(),
        members()[0].bytes.as_slice()
    );

    // Signed with no trust configured: refused, not imported unchecked.
    let report = admit(&bytes, None, Vec::new()).unwrap_err();
    assert_eq!(ids(&report), vec!["package.untrusted-signer"]);

    // A revocation list the installation saw applies.
    let lists = crls_from_pem(&crl(&world.root, &[world.key.serial])).unwrap();
    let revoked = admit(
        &bytes,
        Some(&settings(&world, ConfigurationSigningPolicy::Optional)),
        lists,
    )
    .unwrap_err();
    assert_eq!(ids(&revoked), vec!["package.revoked-signer"]);
}

#[test]
fn a_package_without_the_importable_archive_has_nothing_to_import() {
    let world = world();
    let only_plan = vec![members()[1].clone()];
    let mut manifest = approved(&world, 8);
    manifest.content.files = file_entries(&only_plan).unwrap();
    manifest.content_sha256 = manifest.content.sha256().unwrap();
    let mut fresh = unsigned_manifest(8);
    fresh.content = manifest.content.clone();
    fresh.content_sha256 = manifest.content_sha256.clone();
    approve(&mut fresh, &world.manager, "Configuration Manager");
    approve(&mut fresh, &world.officer, "Security Officer");
    let bytes = signed(&fresh, &world.key, &only_plan);
    let verified = verify_package(&bytes, &trust(&world)).unwrap();
    let problem = importable_member(&verified).unwrap_err();
    assert_eq!(problem.id.as_deref(), Some("package.no-importable"));
}

#[test]
fn trust_settings_that_are_not_pem_are_reported() {
    let problem =
        PackageTrust::from_pem("not a certificate", "", &[], 2).unwrap_err();
    assert_eq!(problem.id.as_deref(), Some("package.unreadable-trust"));
    assert_eq!(problem.details["setting"], "package_roots");
}

#[test]
fn every_trust_setting_that_cannot_be_read_is_named() {
    let world = world();
    let root = world.root.pem();
    let staff = world.staff_root.pem();

    let problem = PackageTrust::from_pem(&root, "", &[], 2).unwrap_err();
    assert_eq!(problem.details["setting"], "staff_roots");
    assert_eq!(problem.details["reason"], "there are no certificates in it");

    let broken = "-----BEGIN X509 CRL-----\n!!!\n-----END X509 CRL-----\n";
    let problem =
        PackageTrust::from_pem(&root, &staff, &[broken.to_string()], 2)
            .unwrap_err();
    assert_eq!(problem.id.as_deref(), Some("package.unreadable-trust"));
    assert_eq!(problem.details["setting"], "revocation_lists");
    assert!(problem.details["reason"].starts_with("unreadable PEM"));

    let lists = [crl(&world.root, &[3]), crl(&world.staff_root, &[4])];
    let trust = PackageTrust::from_pem(&root, &staff, &lists, 2).unwrap();
    assert_eq!(trust.revocation_lists.len(), 2);
    assert_eq!(trust.required_approvals, 2);
}

#[test]
fn an_installations_settings_that_are_not_pem_admit_nothing() {
    let world = world();
    let bytes = signed(&approved(&world, 8), &world.key, &members());

    let mut settings = settings(&world, ConfigurationSigningPolicy::Optional);
    settings.staff_roots = "not a certificate".to_string();
    let problem = settings.trust(Vec::new()).unwrap_err();
    assert_eq!(problem.details["setting"], "staff_roots");
    let report = admit(&bytes, Some(&settings), Vec::new()).unwrap_err();
    assert_eq!(ids(&report), vec!["package.unreadable-trust"]);

    settings.package_roots = "not a certificate".to_string();
    let problem = settings.trust(Vec::new()).unwrap_err();
    assert_eq!(problem.details["setting"], "package_roots");
}

#[test]
fn settings_without_roots_trust_nobody_and_keep_the_lists_they_are_given() {
    let world = world();
    let lists = crls_from_pem(&crl(&world.root, &[3])).unwrap();
    let trust = ConfigurationSigning {
        required_approvals: 3,
        ..ConfigurationSigning::default()
    }
    .trust(lists.clone())
    .unwrap();
    assert!(trust.package_roots.is_empty());
    assert!(trust.staff_roots.is_empty());
    assert_eq!(trust.revocation_lists, lists);
    assert_eq!(trust.required_approvals, 3);
}

#[test]
fn the_package_imported_last_is_reported_as_already_imported() {
    let problem = already_imported(8);
    assert_eq!(problem.id.as_deref(), Some("package.already-imported"));
    assert_eq!(problem.code, Code::Rollback);
    assert_eq!(problem.path, "manifest.configuration.revision");
    assert_eq!(problem.details["revision"], "8");
}

/// A package with `manifest` as its manifest, byte for byte, signed by the
/// organization's key.
fn signed_bytes(world: &World, manifest: &[u8]) -> Vec<u8> {
    package(
        "p.zip",
        &members(),
        manifest,
        &sign(&world.key.key, manifest),
        &world.key.chain_pem,
    )
    .unwrap()
    .bytes
}

/// A good package that says `chain` is its signer's certificates.
fn with_chain(world: &World, chain: &str) -> Vec<u8> {
    let manifest = approved(world, 8).to_bytes().unwrap();
    package(
        "p.zip",
        &members(),
        &manifest,
        &sign(&world.key.key, &manifest),
        chain,
    )
    .unwrap()
    .bytes
}

/// The one error a package is refused with.
fn refusal(bytes: &[u8], trust: &PackageTrust) -> Problem {
    let report = verify_package(bytes, trust).unwrap_err();
    let errors: Vec<&Problem> = report.errors().collect();
    assert_eq!(errors.len(), 1, "{errors:?}");
    errors[0].clone()
}

#[test]
fn a_file_that_is_not_a_zip_is_not_verified() {
    let world = world();
    assert_eq!(
        refused(b"not a zip", &trust(&world)),
        vec!["package.unreadable-zip"]
    );
}

#[test]
fn a_manifest_that_cannot_be_read_is_refused() {
    let world = world();
    for manifest in [&b"not json"[..], b"{}", br#"{"format": 1}"#] {
        let problem = refusal(&signed_bytes(&world, manifest), &trust(&world));
        assert_eq!(problem.id.as_deref(), Some("package.unreadable-manifest"));
        assert_eq!(problem.path, MANIFEST_MEMBER);
        assert!(!problem.details["reason"].is_empty());
    }
}

#[test]
fn a_signing_time_that_is_not_a_time_is_refused() {
    let world = world();
    let mut manifest = approved(&world, 8);

    manifest.produced.at = "yesterday".to_string();
    let bytes = signed(&manifest, &world.key, &members());
    let problem = refusal(&bytes, &trust(&world));
    assert_eq!(problem.id.as_deref(), Some("package.invalid-time"));
    assert_eq!(problem.details["value"], "yesterday");

    manifest.produced.at = "1969-12-31T23:59:59Z".to_string();
    let bytes = signed(&manifest, &world.key, &members());
    let problem = refusal(&bytes, &trust(&world));
    assert_eq!(problem.id.as_deref(), Some("package.invalid-time"));
    assert!(problem.message.contains("before 1970"), "{problem:?}");
}

#[test]
fn a_revocation_list_that_cannot_be_read_is_refused_rather_than_skipped() {
    let world = world();
    let unreadable = "package.unreadable-revocation-list";

    // Carried in the package: PEM that doesn't decode, and PEM that decodes
    // to something that is not a revocation list.
    for list in [
        "-----BEGIN X509 CRL-----\n!!!\n-----END X509 CRL-----\n",
        "-----BEGIN X509 CRL-----\nAAAA\n-----END X509 CRL-----\n",
    ] {
        let mut manifest = approved(&world, 8);
        manifest.revocation_lists = vec![list.to_string()];
        let bytes = signed(&manifest, &world.key, &members());
        let problem = refusal(&bytes, &trust(&world));
        assert_eq!(problem.id.as_deref(), Some(unreadable));
        assert_eq!(problem.path, "revocation_lists");
    }

    // Held by the installation.
    let bytes = signed(&approved(&world, 8), &world.key, &members());
    let mut trust = trust(&world);
    trust.revocation_lists = vec![b"not a revocation list".to_vec()];
    assert_eq!(refused(&bytes, &trust), vec![unreadable]);
    assert!(check_revocation_list(b"not a revocation list").is_err());
    assert!(check_revocation_list(
        &crls_from_pem(&crl(&world.root, &[3])).unwrap()[0]
    )
    .is_ok());
}

#[test]
fn a_chain_that_holds_no_certificate_is_refused() {
    let world = world();
    let trust = trust(&world);

    let problem = refusal(&with_chain(&world, "no certificates here"), &trust);
    assert_eq!(problem.id.as_deref(), Some("package.unreadable-chain"));
    assert_eq!(problem.path, CHAIN_MEMBER);
    assert_eq!(problem.details["reason"], "there are no certificates in it");

    let broken =
        "-----BEGIN CERTIFICATE-----\n!!!\n-----END CERTIFICATE-----\n";
    let problem = refusal(&with_chain(&world, broken), &trust);
    assert_eq!(problem.id.as_deref(), Some("package.unreadable-chain"));
    assert!(problem.details["reason"].starts_with("unreadable PEM"));
}

#[test]
fn a_chain_whose_first_certificate_is_not_one_is_not_trusted() {
    let world = world();
    let not_der =
        "-----BEGIN CERTIFICATE-----\nAAAA\n-----END CERTIFICATE-----\n";
    let problem = refusal(&with_chain(&world, not_der), &trust(&world));
    assert_eq!(problem.id.as_deref(), Some("package.untrusted-signer"));
    assert!(
        problem.details["reason"].starts_with("the certificate is unreadable"),
        "{problem:?}"
    );
    assert!(verify_with(
        b"AAAA",
        MANIFEST_ALGORITHMS,
        b"message",
        b"signature"
    )
    .unwrap_err()
    .starts_with("the certificate is unreadable"));
}

#[test]
fn a_root_that_is_not_a_certificate_trusts_nothing() {
    let world = world();
    let bytes = signed(&approved(&world, 8), &world.key, &members());
    let mut trust = trust(&world);
    trust.package_roots = vec![b"not a certificate".to_vec()];
    let problem = refusal(&bytes, &trust);
    assert_eq!(problem.id.as_deref(), Some("package.untrusted-signer"));
    assert!(
        problem.details["reason"].starts_with("a trusted root is unreadable"),
        "{problem:?}"
    );
}

#[test]
fn an_approval_with_no_certificate_behind_it_is_not_trusted() {
    let world = world();
    let problem = check_certificate(
        &[],
        &trust(&world).staff_roots,
        &[],
        unix_time("2026-06-15T08:00:00Z").unwrap(),
        Role::Approver,
    )
    .unwrap_err();
    assert_eq!(problem.id.as_deref(), Some("package.untrusted-approver"));
    assert_eq!(problem.path, "manifest.approvals");
    assert_eq!(problem.details["reason"], "there is no certificate");
}

#[test]
fn a_package_signed_before_its_key_was_valid_is_refused() {
    // The key's certificate starts in 2025.
    let world = world();
    let mut manifest = approved(&world, 8);
    manifest.produced.at = "2024-06-15T08:00:00Z".to_string();
    let bytes = signed(&manifest, &world.key, &members());
    let problem = refusal(&bytes, &trust(&world));
    assert_eq!(problem.id.as_deref(), Some("package.untrusted-signer"));
    assert_eq!(
        problem.details["reason"],
        "it was not valid yet when it was used"
    );
}

#[test]
fn a_certificate_authority_does_not_sign_packages_itself() {
    let world = world();
    let manifest = approved(&world, 8).to_bytes().unwrap();
    let bytes = package(
        "p.zip",
        &members(),
        &manifest,
        &sign(world.root.key(), &manifest),
        &world.root.pem(),
    )
    .unwrap()
    .bytes;
    let problem = refusal(&bytes, &trust(&world));
    assert_eq!(problem.id.as_deref(), Some("package.untrusted-signer"));
    assert_eq!(
        problem.details["reason"],
        "it is a certificate authority, which doesn't sign as a person"
    );
}

#[test]
fn a_key_certified_by_a_certificate_that_may_not_certify_is_refused() {
    let world = world();
    let mut params = CertificateParams::new(Vec::<String>::new()).unwrap();
    params
        .distinguished_name
        .push(DnType::CommonName, "Not An Authority");
    params.key_usages = vec![KeyUsagePurpose::DigitalSignature];
    params.not_before = date_time_ymd(2025, 1, 1);
    params.not_after = date_time_ymd(2035, 1, 1);
    let not_an_authority = CertifiedIssuer::signed_by(
        params,
        KeyPair::generate_for(&PKCS_ECDSA_P256_SHA256).unwrap(),
        &world.root,
    )
    .unwrap();
    let key = leaf(
        "Configuration Signing Key",
        &not_an_authority,
        LeafOptions::default(),
    );
    let bytes = signed(&approved(&world, 8), &key, &members());
    let problem = refusal(&bytes, &trust(&world));
    assert_eq!(problem.id.as_deref(), Some("package.untrusted-signer"));
    assert!(
        problem.details["reason"]
            .starts_with("its certificate path doesn't verify"),
        "{problem:?}"
    );
}

/// 1.3.6.1.5.5.7.3.36, documentSigning.
const DOCUMENT_SIGNING: [u64; 9] = [1, 3, 6, 1, 5, 5, 7, 3, 36];

fn key_for(world: &World, extended: Vec<ExtendedKeyUsagePurpose>) -> Signer {
    leaf(
        "Configuration Signing Key",
        &world.root,
        LeafOptions {
            extended,
            ..LeafOptions::default()
        },
    )
}

#[test]
fn a_certificate_for_signing_documents_or_mail_signs_packages() {
    let world = world();
    for purpose in [
        ExtendedKeyUsagePurpose::EmailProtection,
        ExtendedKeyUsagePurpose::Other(DOCUMENT_SIGNING.to_vec()),
        ExtendedKeyUsagePurpose::Any,
    ] {
        let key = key_for(&world, vec![purpose.clone()]);
        let bytes = signed(&approved(&world, 8), &key, &members());
        assert!(
            verify_package(&bytes, &trust(&world)).is_ok(),
            "{purpose:?}"
        );
    }
}

#[test]
fn a_certificate_for_something_other_than_signing_does_not_sign_packages() {
    let world = world();
    for purpose in [
        ExtendedKeyUsagePurpose::ServerAuth,
        ExtendedKeyUsagePurpose::CodeSigning,
        // 1.3.6.1.5.5.7.3.17, which is not one of the document signing ones.
        ExtendedKeyUsagePurpose::Other(vec![1, 3, 6, 1, 5, 5, 7, 3, 17]),
    ] {
        let key = key_for(&world, vec![purpose.clone()]);
        let bytes = signed(&approved(&world, 8), &key, &members());
        assert_eq!(
            refused(&bytes, &trust(&world)),
            vec!["package.signer-key-usage"],
            "{purpose:?}"
        );
    }
}

/// The reason the second approval of an otherwise good package doesn't
/// count, once `change` has been made to it.
fn second_approval_refused(change: fn(&mut Approval)) -> String {
    let world = world();
    let mut manifest = approved(&world, 8);
    change(&mut manifest.approvals[1]);
    let bytes = signed(&manifest, &world.key, &members());
    let report = verify_package(&bytes, &trust(&world)).unwrap_err();
    assert_eq!(
        ids(&report),
        vec!["package.approval-invalid", "package.too-few-approvals"]
    );
    let invalid = report.errors().next().unwrap();
    assert_eq!(invalid.path, "manifest.approvals[1]");
    assert_eq!(invalid.details["name"], "Security Officer");
    invalid.details["reason"].clone()
}

#[test]
fn an_approval_that_cannot_be_checked_does_not_count() {
    let reason = second_approval_refused(|approval| {
        approval.signed_at = "this morning".to_string()
    });
    assert!(reason.contains("'this morning' is not a time"), "{reason}");

    let reason = second_approval_refused(|approval| {
        approval.certificate_chain = String::new()
    });
    assert_eq!(reason, "there are no certificates in it");

    let reason = second_approval_refused(|approval| {
        approval.signature = "not base64!".to_string()
    });
    assert!(
        reason.starts_with("the signature is not base64"),
        "{reason}"
    );

    // An ECDSA signature said to be RSA.
    let reason = second_approval_refused(|approval| {
        approval.algorithm = SignatureAlgorithm::RsaPkcs1Sha256
    });
    assert_eq!(reason, "the signature does not match the signed bytes");
}

#[test]
fn an_approval_without_a_time_does_not_count_when_the_verifier_has_a_clock() {
    let world = world();
    let mut manifest = approved(&world, 8);
    manifest.approvals[0].signed_at = "this morning".to_string();
    let bytes = signed(&manifest, &world.key, &members());
    let report =
        verify_package_at(&bytes, &trust(&world), at("2026-06-15T09:00:00Z"))
            .unwrap_err();
    let invalid = report.errors().next().unwrap();
    assert_eq!(invalid.id.as_deref(), Some("package.approval-invalid"));
    assert_eq!(invalid.details["name"], "Configuration Manager");
    assert!(invalid.details["reason"].contains("is not a time"));
}

#[test]
fn an_approval_by_a_certificate_that_is_not_for_signing_does_not_count() {
    let world = world();
    let clerk = leaf(
        "Clerk",
        &world.staff_root,
        LeafOptions {
            serial: 12,
            key_usages: vec![KeyUsagePurpose::KeyEncipherment],
            ..LeafOptions::default()
        },
    );
    let mut manifest = unsigned_manifest(8);
    approve(&mut manifest, &world.manager, "Configuration Manager");
    approve(&mut manifest, &clerk, "Clerk");
    let bytes = signed(&manifest, &world.key, &members());
    let report = verify_package(&bytes, &trust(&world)).unwrap_err();
    let invalid = report.errors().next().unwrap();
    assert_eq!(invalid.id.as_deref(), Some("package.approval-invalid"));
    assert_eq!(
        invalid.details["reason"],
        "the approver's certificate is not made for signing"
    );
}

#[test]
fn a_manifest_with_a_number_too_large_to_sign_exactly_is_refused() {
    let world = world();
    let mut trust = trust(&world);
    trust.required_approvals = 0;

    // A revision nobody can have approved: its payload can't be written.
    let bytes = signed(&unsigned_manifest(1 << 53), &world.key, &members());
    assert_eq!(refused(&bytes, &trust), vec!["package.unhashable-content"]);

    // A file size the content digest can't cover.
    let mut manifest = unsigned_manifest(8);
    manifest.content.files[0].size = 1 << 53;
    let bytes = signed(&manifest, &world.key, &members());
    assert_eq!(refused(&bytes, &trust), vec!["package.unhashable-content"]);
}

#[test]
fn a_signing_certificate_that_cannot_be_read_is_not_checked() {
    let world = world();
    let trust = trust(&world);

    let problem = verify_signing_certificate(
        "not a certificate",
        &trust.package_roots,
        &[],
        "2026-06-15T08:00:00Z",
    )
    .unwrap_err();
    assert_eq!(problem.id.as_deref(), Some("package.unreadable-chain"));

    let problem = verify_signing_certificate(
        &world.key.chain_pem,
        &trust.package_roots,
        &[],
        "next week",
    )
    .unwrap_err();
    assert_eq!(problem.id.as_deref(), Some("package.invalid-time"));
    assert_eq!(problem.details["value"], "next week");
}

#[test]
fn a_signing_certificate_revoked_by_a_list_given_is_refused_before_use() {
    let world = world();
    let lists = crls_from_pem(&crl(&world.root, &[world.key.serial])).unwrap();
    let problem = verify_signing_certificate(
        &world.key.chain_pem,
        &trust(&world).package_roots,
        &lists,
        "2026-06-15T08:00:00Z",
    )
    .unwrap_err();
    assert_eq!(problem.id.as_deref(), Some("package.revoked-signer"));
    assert_eq!(problem.path, CHAIN_MEMBER);
}

#[test]
fn the_signature_members_are_named_as_the_manifest_module_names_them() {
    assert_eq!(
        (MANIFEST_MEMBER, SIGNATURE_MEMBER, CHAIN_MEMBER),
        ("manifest.json", "manifest.sig", "manifest.chain.pem")
    );
}
