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
    approval_payload, package, ConfigurationRevision, Content, Produced,
    ReportFormat, ReportSetting, CHAIN_MEMBER, MANIFEST_MEMBER,
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
fn trust_settings_that_are_not_pem_are_reported() {
    let problem =
        PackageTrust::from_pem("not a certificate", "", &[], 2).unwrap_err();
    assert_eq!(problem.id.as_deref(), Some("package.unreadable-trust"));
    assert_eq!(problem.details["setting"], "package_roots");
}

#[test]
fn the_signature_members_are_named_as_the_manifest_module_names_them() {
    assert_eq!(
        (MANIFEST_MEMBER, SIGNATURE_MEMBER, CHAIN_MEMBER),
        ("manifest.json", "manifest.sig", "manifest.chain.pem")
    );
}
