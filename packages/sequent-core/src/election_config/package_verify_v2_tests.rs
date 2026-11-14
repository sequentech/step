// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Format 2: approvals and the final signature bound to the target tenant,
//! the exact policy digest and enrolled signer IDs; enrollment, roles,
//! distinct people and final-signer separation counted per signer ID; time
//! is the verifier's; RSA-PSS exactly; and format 1 kept as it was.

use rcgen::PKCS_RSA_SHA256;
use ring::signature::{RsaKeyPair, RSA_PKCS1_SHA256, RSA_PSS_SHA256};

use super::*;
use crate::election_config::manifest::{
    approval_payload_v2, ApprovalBinding, SigningBinding,
};
use crate::election_config::signing_policy::{
    EnrolledKey, EnrolledSigner, FinalSignerSeparation, PackageFormat,
    SignerRole,
};

const TENANT: &str = "90505c8a-23a9-4cdf-a26b-4e19f6a097d5";
const APPROVER_RSA: &str = include_str!("package_verify_fixtures/approver-rsa-2048.pk8.pem");
const SIGNER_RSA: &str = include_str!("package_verify_fixtures/signer-rsa-2048.pk8.pem");

fn now() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-11-14T00:00:00Z").unwrap().with_timezone(&Utc)
}

/// A signing key: ECDSA P-256 (rcgen) or RSA (a fixture), with its chain.
struct Key {
    chain_pem: String,
    certificate_pem: String,
    sign: Box<dyn Fn(&[u8]) -> Vec<u8>>,
    algorithm: SignatureAlgorithm,
}

fn ecdsa(name: &str, issuer: &Ca, serial: u64, not_after: (i32, u8, u8)) -> Key {
    let signer = leaf(
        name,
        issuer,
        LeafOptions {
            serial,
            not_after,
            ..LeafOptions::default()
        },
    );
    let der = signer.key.serialize_der();
    Key {
        chain_pem: signer.chain_pem,
        certificate_pem: signer.certificate_pem,
        sign: Box::new(move |message| {
            let rng = SystemRandom::new();
            EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, &der, &rng)
                .unwrap()
                .sign(&rng, message)
                .unwrap()
                .as_ref()
                .to_vec()
        }),
        algorithm: SignatureAlgorithm::EcdsaP256Sha256,
    }
}

fn rsa(name: &str, issuer: &Ca, serial: u64, pem: &'static str, pss: bool) -> Key {
    let key = KeyPair::from_pem_and_sign_algo(pem, &PKCS_RSA_SHA256).unwrap();
    let mut params = CertificateParams::new(Vec::<String>::new()).unwrap();
    params.distinguished_name.push(DnType::CommonName, name);
    params.serial_number = Some(SerialNumber::from(serial));
    params.key_usages = vec![KeyUsagePurpose::DigitalSignature];
    params.not_before = date_time_ymd(2025, 1, 1);
    params.not_after = date_time_ymd(2035, 1, 1);
    let certificate = params.signed_by(&key, issuer).unwrap();
    let der = key.serialize_der();
    Key {
        chain_pem: format!("{}{}", certificate.pem(), issuer.pem()),
        certificate_pem: certificate.pem(),
        sign: Box::new(move |message| {
            let pair = RsaKeyPair::from_pkcs8(&der).unwrap();
            let mut out = vec![0u8; pair.public().modulus_len()];
            let padding: &'static dyn ring::signature::RsaEncoding =
                if pss { &RSA_PSS_SHA256 } else { &RSA_PKCS1_SHA256 };
            pair.sign(padding, &SystemRandom::new(), message, &mut out).unwrap();
            out
        }),
        algorithm: if pss {
            SignatureAlgorithm::RsaPssSha256
        } else {
            SignatureAlgorithm::RsaPkcs1Sha256
        },
    }
}

fn pin(key: &Key) -> String {
    spki_sha256_of_certificate(&key.certificate_pem).unwrap()
}

struct V2 {
    world: World,
    approver_1: Key,
    approver_2: Key,
    custodian: Key,
}

fn v2_world() -> V2 {
    let world = world();
    let approver_1 = ecdsa("DEMO Approver One", &world.staff_root, 21, (2035, 1, 1));
    let approver_2 = ecdsa("DEMO Approver Two", &world.staff_root, 22, (2035, 1, 1));
    let custodian = ecdsa("DEMO Custodian Key", &world.root, 23, (2035, 1, 1));
    V2 {
        world,
        approver_1,
        approver_2,
        custodian,
    }
}

fn enrolled(id: &str, roles: &[SignerRole], keys: &[&Key]) -> EnrolledSigner {
    EnrolledSigner {
        id: id.to_string(),
        name: format!("DEMO {id}"),
        roles: roles.to_vec(),
        keys: keys
            .iter()
            .map(|key| EnrolledKey {
                spki_sha256: pin(key),
                label: String::new(),
            })
            .collect(),
    }
}

fn policy(v2: &V2) -> ConfigurationSigning {
    ConfigurationSigning {
        policy: ConfigurationSigningPolicy::Required,
        package_roots: v2.world.root.pem(),
        staff_roots: v2.world.staff_root.pem(),
        required_approvals: 2,
        minimum_package_format: Some(PackageFormat::V2),
        final_signer_separation: Some(FinalSignerSeparation::Separate),
        signers: vec![
            enrolled("approver-1", &[SignerRole::Approver], &[&v2.approver_1]),
            enrolled("approver-2", &[SignerRole::Approver], &[&v2.approver_2]),
            enrolled("custodian", &[SignerRole::FinalSigner], &[&v2.custodian]),
        ],
    }
}

fn binding(manifest: &Manifest, settings: &ConfigurationSigning, tenant: &str) -> ApprovalBinding {
    ApprovalBinding {
        tenant_id: tenant.to_string(),
        external_id: manifest.configuration.external_id.clone(),
        revision: manifest.configuration.revision,
        content_sha256: manifest.content_sha256.clone(),
        policy_sha256: settings.policy_sha256().unwrap(),
    }
}

fn approve_v2(manifest: &mut Manifest, bind: &ApprovalBinding, key: &Key, signer_id: &str) {
    let payload = approval_payload_v2(bind, signer_id).unwrap();
    manifest.approvals.push(Approval {
        name: format!("DEMO {signer_id}"),
        role: "approver".to_string(),
        signed_at: "2026-11-13T10:00:00Z".to_string(),
        algorithm: key.algorithm,
        certificate_chain: key.chain_pem.clone(),
        signature: STANDARD.encode((key.sign)(payload.as_bytes())),
        signer_id: Some(signer_id.to_string()),
    });
}

/// A format-2 manifest approved by `approvers` (key, signer ID) and bound to
/// `settings` for `tenant`, finally signed by `final_id`.
fn v2_manifest(
    settings: &ConfigurationSigning,
    tenant: &str,
    approvers: &[(&Key, &str)],
    final_id: &str,
) -> Manifest {
    let mut manifest = unsigned_manifest(3);
    manifest.format = ManifestFormat::V2;
    manifest.produced.at = "2026-11-13T12:00:00Z".to_string();
    let bind = binding(&manifest, settings, tenant);
    for (key, id) in approvers {
        approve_v2(&mut manifest, &bind, key, id);
    }
    manifest.signing = Some(SigningBinding {
        tenant_id: tenant.to_string(),
        policy_sha256: bind.policy_sha256,
        final_signer_id: final_id.to_string(),
        role: "final-signer".to_string(),
    });
    manifest
}

fn signed_by(manifest: &Manifest, key: &Key) -> Vec<u8> {
    let bytes = manifest.to_bytes().unwrap();
    package("p.zip", &members(), &bytes, &(key.sign)(&bytes), &key.chain_pem)
        .unwrap()
        .bytes
}

fn genuine(v2: &V2, settings: &ConfigurationSigning) -> Vec<u8> {
    let manifest = v2_manifest(
        settings,
        TENANT,
        &[(&v2.approver_1, "approver-1"), (&v2.approver_2, "approver-2")],
        "custodian",
    );
    signed_by(&manifest, &v2.custodian)
}

fn trust_v2(settings: &ConfigurationSigning) -> PackageTrust {
    settings.trust_for(TENANT, Vec::new()).unwrap()
}

fn refused_v2(bytes: &[u8], trust: &PackageTrust) -> Vec<String> {
    match verify_package_at(bytes, trust, now()) {
        Ok(_) => panic!("expected the format-2 package to be refused"),
        Err(report) => ids(&report),
    }
}

#[test]
fn a_v2_package_approved_by_two_enrolled_people_and_signed_by_the_custodian_verifies() {
    let v2 = v2_world();
    let settings = policy(&v2);
    let verified = verify_package_at(&genuine(&v2, &settings), &trust_v2(&settings), now()).unwrap();
    assert_eq!(verified.manifest.format, ManifestFormat::V2);
    assert_eq!(verified.signer_id.as_deref(), Some("custodian"));
    assert_eq!(verified.approver_ids, vec!["approver-1", "approver-2"]);
}

#[test]
fn rsa_pss_approvals_and_final_signature_verify_with_exact_parameters() {
    let v2 = v2_world();
    let approver_rsa = rsa("DEMO RSA Approver", &v2.world.staff_root, 31, APPROVER_RSA, true);
    let custodian_rsa = rsa("DEMO RSA Custodian", &v2.world.root, 32, SIGNER_RSA, true);
    let mut settings = policy(&v2);
    settings.signers[1] = enrolled("approver-2", &[SignerRole::Approver], &[&approver_rsa]);
    settings.signers[2] = enrolled("custodian", &[SignerRole::FinalSigner], &[&custodian_rsa]);
    let manifest = v2_manifest(
        &settings,
        TENANT,
        &[(&v2.approver_1, "approver-1"), (&approver_rsa, "approver-2")],
        "custodian",
    );
    let verified =
        verify_package_at(&signed_by(&manifest, &custodian_rsa), &trust_v2(&settings), now()).unwrap();
    assert_eq!(verified.approver_ids.len(), 2);
}

#[test]
fn rsa_pkcs1_v1_5_is_refused_in_format_2() {
    let v2 = v2_world();
    let approver_pkcs1 = rsa("DEMO PKCS1 Approver", &v2.world.staff_root, 33, APPROVER_RSA, false);
    let custodian_pkcs1 = rsa("DEMO PKCS1 Custodian", &v2.world.root, 34, SIGNER_RSA, false);
    let mut settings = policy(&v2);
    settings.signers[1] = enrolled("approver-2", &[SignerRole::Approver], &[&approver_pkcs1]);
    // An approval with PKCS#1 v1.5 doesn't count.
    let manifest = v2_manifest(
        &settings,
        TENANT,
        &[(&v2.approver_1, "approver-1"), (&approver_pkcs1, "approver-2")],
        "custodian",
    );
    let found = refused_v2(&signed_by(&manifest, &v2.custodian), &trust_v2(&settings));
    assert!(found.contains(&"package.approval-algorithm".to_string()), "{found:?}");
    // A final signature with PKCS#1 v1.5 doesn't verify.
    settings.signers[1] = enrolled("approver-2", &[SignerRole::Approver], &[&v2.approver_2]);
    settings.signers[2] = enrolled("custodian", &[SignerRole::FinalSigner], &[&custodian_pkcs1]);
    let manifest = v2_manifest(
        &settings,
        TENANT,
        &[(&v2.approver_1, "approver-1"), (&v2.approver_2, "approver-2")],
        "custodian",
    );
    let found = refused_v2(&signed_by(&manifest, &custodian_pkcs1), &trust_v2(&settings));
    assert!(found.contains(&"package.bad-signature".to_string()), "{found:?}");
}

#[test]
fn rsa_keys_outside_2048_to_4096_bits_are_refused() {
    let small = include_bytes!("package_verify_fixtures/rsa-1024.spki.der");
    let large = include_bytes!("package_verify_fixtures/rsa-6144.spki.der");
    assert!(rsa_modulus_allowed(small).is_err());
    assert!(rsa_modulus_allowed(large).is_err());
    let ok = x509_parser::prelude::X509Certificate::from_der(
        &certificates_from_pem(&rsa("x", &world().staff_root, 1, APPROVER_RSA, true).certificate_pem)
            .unwrap()[0],
    )
    .unwrap()
    .1
    .public_key()
    .raw
    .to_vec();
    assert!(rsa_modulus_allowed(&ok).is_ok());
}

#[test]
fn an_unenrolled_approver_or_custodian_is_refused() {
    let v2 = v2_world();
    let stranger = ecdsa("DEMO Stranger", &v2.world.staff_root, 41, (2035, 1, 1));
    let settings = policy(&v2);
    // A valid certificate under the trusted root, but its key is not enrolled.
    let manifest = v2_manifest(
        &settings,
        TENANT,
        &[(&v2.approver_1, "approver-1"), (&stranger, "approver-2")],
        "custodian",
    );
    let found = refused_v2(&signed_by(&manifest, &v2.custodian), &trust_v2(&settings));
    assert!(found.contains(&"package.approver-id-mismatch".to_string()), "{found:?}");
    assert!(found.contains(&"package.too-few-approvals".to_string()), "{found:?}");

    let other_key = ecdsa("DEMO Other Custodian", &v2.world.root, 42, (2035, 1, 1));
    let manifest = v2_manifest(
        &settings,
        TENANT,
        &[(&v2.approver_1, "approver-1"), (&v2.approver_2, "approver-2")],
        "custodian",
    );
    let found = refused_v2(&signed_by(&manifest, &other_key), &trust_v2(&settings));
    assert!(found.contains(&"package.final-signer-not-enrolled".to_string()), "{found:?}");
}

#[test]
fn roles_are_the_policys_not_the_packages() {
    let v2 = v2_world();
    // A key under the approvers' root, enrolled only to make final
    // signatures: its approval doesn't count.
    let final_only = ecdsa("DEMO Final Only", &v2.world.staff_root, 71, (2035, 1, 1));
    let mut settings = policy(&v2);
    settings.signers.push(enrolled("final-only", &[SignerRole::FinalSigner], &[&final_only]));
    let manifest = v2_manifest(
        &settings,
        TENANT,
        &[(&v2.approver_1, "approver-1"), (&final_only, "final-only")],
        "custodian",
    );
    let found = refused_v2(&signed_by(&manifest, &v2.custodian), &trust_v2(&settings));
    assert!(found.contains(&"package.approver-role".to_string()), "{found:?}");

    // A key under the package root, enrolled only to approve: its final
    // signature doesn't count.
    let approve_only = ecdsa("DEMO Approve Only", &v2.world.root, 72, (2035, 1, 1));
    let mut settings = policy(&v2);
    settings.signers.push(enrolled("approve-only", &[SignerRole::Approver], &[&approve_only]));
    let manifest = v2_manifest(
        &settings,
        TENANT,
        &[(&v2.approver_1, "approver-1"), (&v2.approver_2, "approver-2")],
        "approve-only",
    );
    let found = refused_v2(&signed_by(&manifest, &approve_only), &trust_v2(&settings));
    assert!(found.contains(&"package.final-signer-role".to_string()), "{found:?}");
}

#[test]
fn an_approval_must_name_the_signer_its_key_belongs_to() {
    let v2 = v2_world();
    let settings = policy(&v2);
    // approver-1's key, claiming to be approver-2.
    let manifest = v2_manifest(
        &settings,
        TENANT,
        &[(&v2.approver_1, "approver-1"), (&v2.approver_1, "approver-2")],
        "custodian",
    );
    let found = refused_v2(&signed_by(&manifest, &v2.custodian), &trust_v2(&settings));
    assert!(found.contains(&"package.approver-id-mismatch".to_string()), "{found:?}");
}

#[test]
fn two_certificates_of_one_person_count_once_and_fail_closed() {
    let v2 = v2_world();
    let reissued = ecdsa("DEMO Approver One (new key)", &v2.world.staff_root, 51, (2035, 1, 1));
    let mut settings = policy(&v2);
    settings.signers[0] =
        enrolled("approver-1", &[SignerRole::Approver], &[&v2.approver_1, &reissued]);
    // Both of approver-1's keys approve; approver-2 does not.
    let manifest = v2_manifest(
        &settings,
        TENANT,
        &[(&v2.approver_1, "approver-1"), (&reissued, "approver-1")],
        "custodian",
    );
    let found = refused_v2(&signed_by(&manifest, &v2.custodian), &trust_v2(&settings));
    assert!(found.contains(&"package.duplicate-signer".to_string()), "{found:?}");
    assert!(found.contains(&"package.too-few-approvals".to_string()), "{found:?}");
    // The reissued key alone stands for the same person.
    let manifest = v2_manifest(
        &settings,
        TENANT,
        &[(&reissued, "approver-1"), (&v2.approver_2, "approver-2")],
        "custodian",
    );
    verify_package_at(&signed_by(&manifest, &v2.custodian), &trust_v2(&settings), now()).unwrap();
}

#[test]
fn the_final_signer_may_not_have_approved_unless_the_policy_allows_it() {
    let v2 = v2_world();
    let mut settings = policy(&v2);
    // One CA for both, so approver-1's key can make a final signature at all.
    settings.package_roots = format!("{}{}", v2.world.root.pem(), v2.world.staff_root.pem());
    settings.signers[0].roles.push(SignerRole::FinalSigner);
    let manifest = v2_manifest(
        &settings,
        TENANT,
        &[(&v2.approver_1, "approver-1"), (&v2.approver_2, "approver-2")],
        "approver-1",
    );
    let found = refused_v2(&signed_by(&manifest, &v2.approver_1), &trust_v2(&settings));
    assert!(found.contains(&"package.final-signer-approved".to_string()), "{found:?}");

    settings.final_signer_separation = Some(FinalSignerSeparation::Allowed);
    let manifest = v2_manifest(
        &settings,
        TENANT,
        &[(&v2.approver_1, "approver-1"), (&v2.approver_2, "approver-2")],
        "approver-1",
    );
    verify_package_at(&signed_by(&manifest, &v2.approver_1), &trust_v2(&settings), now()).unwrap();
}

#[test]
fn a_package_for_another_tenant_or_policy_is_refused() {
    let v2 = v2_world();
    let settings = policy(&v2);
    let manifest = v2_manifest(
        &settings,
        "00000000-0000-0000-0000-000000000000",
        &[(&v2.approver_1, "approver-1"), (&v2.approver_2, "approver-2")],
        "custodian",
    );
    let found = refused_v2(&signed_by(&manifest, &v2.custodian), &trust_v2(&settings));
    assert!(found.contains(&"package.wrong-tenant".to_string()), "{found:?}");

    // Approved under the policy, then the tenant's policy changed.
    let bytes = genuine(&v2, &settings);
    let mut changed = settings.clone();
    changed.required_approvals = 1;
    let found = refused_v2(&bytes, &trust_v2(&changed));
    assert!(found.contains(&"package.policy-changed".to_string()), "{found:?}");
}

#[test]
fn an_approval_from_another_revision_does_not_count() {
    let v2 = v2_world();
    let settings = policy(&v2);
    let mut manifest = v2_manifest(
        &settings,
        TENANT,
        &[(&v2.approver_1, "approver-1"), (&v2.approver_2, "approver-2")],
        "custodian",
    );
    // approver-2's approval of revision 2, replayed into revision 3.
    let mut older = unsigned_manifest(2);
    older.format = ManifestFormat::V2;
    let bind = binding(&older, &settings, TENANT);
    approve_v2(&mut older, &bind, &v2.approver_2, "approver-2");
    manifest.approvals[1] = older.approvals[0].clone();
    let found = refused_v2(&signed_by(&manifest, &v2.custodian), &trust_v2(&settings));
    assert!(found.contains(&"package.approval-invalid".to_string()), "{found:?}");
    assert!(found.contains(&"package.too-few-approvals".to_string()), "{found:?}");
}

#[test]
fn format_1_is_refused_where_the_tenant_requires_format_2() {
    let v2 = v2_world();
    let settings = policy(&v2);
    // A genuine format-1 package, valid under the same roots.
    let world = &v2.world;
    let v1 = signed(&approved(world, 8), &world.key, &members());
    let found = refused_v2(&v1, &trust_v2(&settings));
    assert!(found.contains(&"package.downgrade-refused".to_string()), "{found:?}");
}

#[test]
fn format_1_still_imports_where_the_tenant_allows_it() {
    let world = world();
    let v1 = signed(&approved(&world, 8), &world.key, &members());
    let legacy = ConfigurationSigning {
        policy: ConfigurationSigningPolicy::Required,
        package_roots: world.root.pem(),
        staff_roots: world.staff_root.pem(),
        required_approvals: 2,
        ..ConfigurationSigning::default()
    };
    let verified = verify_package_at(&v1, &legacy.trust_for(TENANT, Vec::new()).unwrap(), now()).unwrap();
    assert_eq!(verified.manifest.format, ManifestFormat::V1);
    assert_eq!(verified.signer_id, None);
}

#[test]
fn a_format_1_manifest_carrying_format_2_fields_is_refused() {
    let world = world();
    let mut manifest = approved(&world, 8);
    manifest.signing = Some(SigningBinding {
        tenant_id: TENANT.to_string(),
        policy_sha256: "00".repeat(32),
        final_signer_id: "custodian".to_string(),
        role: "final-signer".to_string(),
    });
    let legacy = ConfigurationSigning {
        policy: ConfigurationSigningPolicy::Required,
        package_roots: world.root.pem(),
        staff_roots: world.staff_root.pem(),
        required_approvals: 2,
        ..ConfigurationSigning::default()
    };
    let bytes = signed(&manifest, &world.key, &members());
    let found = refused_v2(&bytes, &legacy.trust_for(TENANT, Vec::new()).unwrap());
    assert!(found.contains(&"package.mixed-format".to_string()), "{found:?}");
}

#[test]
fn a_v2_package_needs_an_enrolling_policy_and_the_verifiers_clock() {
    let v2 = v2_world();
    let settings = policy(&v2);
    let bytes = genuine(&v2, &settings);
    // A tenant without enrolled signers can't check a format-2 package.
    let legacy = ConfigurationSigning {
        policy: ConfigurationSigningPolicy::Required,
        package_roots: v2.world.root.pem(),
        staff_roots: v2.world.staff_root.pem(),
        required_approvals: 2,
        ..ConfigurationSigning::default()
    };
    let found = refused_v2(&bytes, &legacy.trust_for(TENANT, Vec::new()).unwrap());
    assert!(found.contains(&"package.no-signing-policy".to_string()), "{found:?}");
    // Without a clock, nothing to check certificates at.
    match verify_package(&bytes, &trust_v2(&settings)) {
        Ok(_) => panic!("a format-2 package needs the verifier's clock"),
        Err(report) => assert!(ids(&report).contains(&"package.needs-clock".to_string())),
    }
}

#[test]
fn certificates_are_checked_at_the_verifiers_time_not_the_claimed_one() {
    let v2 = v2_world();
    // Valid when it claims to have approved (2026-11-13), expired by now.
    let expired = ecdsa("DEMO Approver Expiring", &v2.world.staff_root, 61, (2026, 11, 13));
    let mut settings = policy(&v2);
    settings.signers[1] = enrolled("approver-2", &[SignerRole::Approver], &[&expired]);
    let mut manifest = v2_manifest(
        &settings,
        TENANT,
        &[(&v2.approver_1, "approver-1"), (&expired, "approver-2")],
        "custodian",
    );
    manifest.approvals[1].signed_at = "2026-11-12T10:00:00Z".to_string();
    let found = refused_v2(&signed_by(&manifest, &v2.custodian), &trust_v2(&settings));
    assert!(found.contains(&"package.approval-invalid".to_string()), "{found:?}");
}

#[test]
fn an_unsigned_file_is_refused_by_a_v2_tenant_whatever_its_signature_policy() {
    let v2 = v2_world();
    let settings = policy(&v2);
    let unsigned = crate::election_config::archive::zip(&members()).unwrap();
    match admit_for_tenant_at(&unsigned, Some(&settings), TENANT, Vec::new(), now()) {
        Ok(_) => panic!("an unsigned file must not bypass a format-2 policy"),
        Err(report) => assert!(ids(&report).contains(&"package.unsigned".to_string())),
    }
    // A contradictory optional + v2 record that somehow got stored is
    // refused before that, as an invalid policy (see the malformed-policy test).
    let mut contradictory = settings.clone();
    contradictory.policy = ConfigurationSigningPolicy::Optional;
    match admit_for_tenant_at(&unsigned, Some(&contradictory), TENANT, Vec::new(), now()) {
        Ok(_) => panic!("an unsigned file must not bypass a contradictory policy"),
        Err(report) => assert!(ids(&report)
            .contains(&"configuration-signing.invalid-stored-policy".to_string())),
    }
}

#[test]
fn tampering_after_signing_is_still_caught_in_format_2() {
    let v2 = v2_world();
    let settings = policy(&v2);
    let manifest = v2_manifest(
        &settings,
        TENANT,
        &[(&v2.approver_1, "approver-1"), (&v2.approver_2, "approver-2")],
        "custodian",
    );
    let bytes = manifest.to_bytes().unwrap();
    let mut other = members();
    other[1].bytes = b"{\"version\":5}".to_vec();
    let tampered = package("p.zip", &other, &bytes, &(v2.custodian.sign)(&bytes), &v2.custodian.chain_pem)
        .unwrap()
        .bytes;
    assert!(!refused_v2(&tampered, &trust_v2(&settings)).is_empty());
}

#[test]
fn a_malformed_stored_policy_refuses_every_file_signed_or_not() {
    let v2 = v2_world();
    let world = &v2.world;
    // Signers stored without a minimum format (a save refuses it; a stored
    // record must not be read with legacy defaults either).
    let mut malformed = policy(&v2);
    malformed.minimum_package_format = None;
    malformed.policy = ConfigurationSigningPolicy::Optional;
    let unsigned = crate::election_config::archive::zip(&members()).unwrap();
    let v1 = signed(&approved(world, 8), &world.key, &members());
    for (what, bytes) in [("unsigned", &unsigned), ("format 1", &v1)] {
        match admit_for_tenant_at(bytes, Some(&malformed), TENANT, Vec::new(), now()) {
            Ok(_) => panic!("{what} admitted under a malformed stored policy"),
            Err(report) => assert!(
                ids(&report).contains(&"configuration-signing.invalid-stored-policy".to_string()),
                "{what}: {:?}",
                ids(&report)
            ),
        }
    }
    assert!(malformed.trust_for(TENANT, Vec::new()).is_err());
    // Optional with format 2 stored: refused the same way.
    let mut contradictory = policy(&v2);
    contradictory.policy = ConfigurationSigningPolicy::Optional;
    assert!(admit_for_tenant_at(&unsigned, Some(&contradictory), TENANT, Vec::new(), now()).is_err());
    // A truly legacy record (no format-2 field) keeps its old behaviour:
    // optional admits an unsigned file.
    let legacy = ConfigurationSigning {
        policy: ConfigurationSigningPolicy::Optional,
        ..ConfigurationSigning::default()
    };
    assert!(matches!(
        admit_for_tenant_at(&unsigned, Some(&legacy), TENANT, Vec::new(), now()),
        Ok(Admission::Unsigned)
    ));
}

#[test]
fn a_v2_approval_labelled_with_another_role_does_not_count() {
    let v2 = v2_world();
    let settings = policy(&v2);
    let mut manifest = v2_manifest(
        &settings,
        TENANT,
        &[(&v2.approver_1, "approver-1"), (&v2.approver_2, "approver-2")],
        "custodian",
    );
    manifest.approvals[1].role = "final-signer".to_string();
    let found = refused_v2(&signed_by(&manifest, &v2.custodian), &trust_v2(&settings));
    assert!(found.contains(&"package.approval-role-label".to_string()), "{found:?}");
    assert!(found.contains(&"package.too-few-approvals".to_string()), "{found:?}");
}

#[test]
fn a_key_withdrawn_from_the_current_policy_is_refused() {
    let v2 = v2_world();
    let settings = policy(&v2);
    let bytes = genuine(&v2, &settings);
    // The installer withdraws approver-2's key (and, separately, the signer).
    let mut withdrawn = settings.clone();
    withdrawn.signers[1].keys.clear();
    withdrawn.signers[1].keys.push(EnrolledKey {
        spki_sha256: "ab".repeat(32),
        label: "replacement".to_string(),
    });
    let found = refused_v2(&bytes, &trust_v2(&withdrawn));
    // The approvals bound the old policy's digest, so under the current one
    // none of them verifies, before enrollment is even consulted.
    assert!(found.contains(&"package.policy-changed".to_string()), "{found:?}");
    assert!(found.contains(&"package.approval-invalid".to_string()), "{found:?}");
    assert!(found.contains(&"package.too-few-approvals".to_string()), "{found:?}");
}

#[test]
fn a_configured_revocation_list_revokes_a_v2_approver() {
    let v2 = v2_world();
    let settings = policy(&v2);
    let bytes = genuine(&v2, &settings);
    // The tenant holds a CRL from the approvers' CA listing approver-2 (serial 22).
    let lists = crls_from_pem(&crl(&v2.world.staff_root, &[22])).unwrap();
    let trust = settings.trust_for(TENANT, lists).unwrap();
    let found = refused_v2(&bytes, &trust);
    assert!(found.contains(&"package.approval-invalid".to_string()), "{found:?}");
    // Without that list the same package verifies: a missing list is not
    // treated as revocation (no freshness is claimed).
    verify_package_at(&bytes, &trust_v2(&settings), now()).unwrap();
}

#[path = "standalone_signing_flow_tests.rs"]
mod standalone;
