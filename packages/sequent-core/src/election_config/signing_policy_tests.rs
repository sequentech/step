// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use crate::election_config::package_verify::{
    ConfigurationSigning, ConfigurationSigningPolicy,
};

fn pin(n: u8) -> String {
    format!("{:02x}", n).repeat(32)
}

fn signer(id: &str, roles: &[SignerRole], pins: &[String]) -> EnrolledSigner {
    EnrolledSigner {
        id: id.to_string(),
        name: format!("DEMO {id}"),
        roles: roles.to_vec(),
        keys: pins
            .iter()
            .map(|p| EnrolledKey {
                spki_sha256: p.clone(),
                label: String::new(),
            })
            .collect(),
    }
}

fn v2_policy() -> ConfigurationSigning {
    ConfigurationSigning {
        policy: ConfigurationSigningPolicy::Required,
        required_approvals: 2,
        minimum_package_format: Some(PackageFormat::V2),
        final_signer_separation: Some(FinalSignerSeparation::Separate),
        signers: vec![
            signer("approver-1", &[SignerRole::Approver], &[pin(1)]),
            signer("approver-2", &[SignerRole::Approver], &[pin(2), pin(3)]),
            signer("custodian", &[SignerRole::FinalSigner], &[pin(4)]),
        ],
        ..ConfigurationSigning::default()
    }
}

fn ids(problems: &[PolicyProblem]) -> Vec<&'static str> {
    problems.iter().map(|p| p.id).collect()
}

#[test]
fn a_well_formed_v2_policy_has_no_problems() {
    assert!(
        check_policy(&v2_policy()).is_empty(),
        "{:?}",
        check_policy(&v2_policy())
    );
}

#[test]
fn a_legacy_record_without_minimum_or_signers_stays_format_1() {
    let legacy = ConfigurationSigning {
        policy: ConfigurationSigningPolicy::Required,
        required_approvals: 2,
        ..ConfigurationSigning::default()
    };
    assert!(check_policy(&legacy).is_empty());
    assert_eq!(legacy.effective_minimum_format(), PackageFormat::V1);
    // A legacy record reads back exactly: no v2 field appears in its JSON.
    let json = serde_json::to_value(&legacy).unwrap();
    assert!(json.get("minimum_package_format").is_none());
    assert!(json.get("final_signer_separation").is_none());
    let read: ConfigurationSigning = serde_json::from_value(
        serde_json::json!({"policy": "required", "required_approvals": 2}),
    )
    .unwrap();
    assert_eq!(read, legacy);
}

#[test]
fn signers_without_an_explicit_minimum_format_are_refused() {
    let mut policy = v2_policy();
    policy.minimum_package_format = None;
    assert!(ids(&check_policy(&policy))
        .contains(&"configuration-signing.minimum-format-required"));
    // Separation alone, too.
    let separation_only = ConfigurationSigning {
        policy: ConfigurationSigningPolicy::Required,
        final_signer_separation: Some(FinalSignerSeparation::Allowed),
        ..ConfigurationSigning::default()
    };
    assert!(ids(&check_policy(&separation_only))
        .contains(&"configuration-signing.minimum-format-required"));
}

#[test]
fn optional_signatures_with_format_2_required_are_contradictory() {
    let mut policy = v2_policy();
    policy.policy = ConfigurationSigningPolicy::Optional;
    assert!(ids(&check_policy(&policy))
        .contains(&"configuration-signing.optional-with-v2"));
}

#[test]
fn a_key_belongs_to_one_signer_only() {
    let mut policy = v2_policy();
    policy.signers[1].keys.push(EnrolledKey {
        spki_sha256: pin(1),
        label: "taken from approver-1".to_string(),
    });
    assert!(ids(&check_policy(&policy))
        .contains(&"configuration-signing.shared-key"));
    assert!(Enrollment::from_signers(&policy.signers).is_err());
}

#[test]
fn ids_are_unique_well_formed_and_signers_complete() {
    let mut policy = v2_policy();
    policy.signers.push(signer(
        "approver-1",
        &[SignerRole::Approver],
        &[pin(9)],
    ));
    policy
        .signers
        .push(signer("Bad ID!", &[SignerRole::Approver], &[pin(10)]));
    policy.signers.push(signer("no-roles", &[], &[pin(11)]));
    policy
        .signers
        .push(signer("no-keys", &[SignerRole::Approver], &[]));
    policy.signers.push(signer(
        "bad-pin",
        &[SignerRole::Approver],
        &["ABCD".to_string()],
    ));
    let found = ids(&check_policy(&policy));
    for id in [
        "configuration-signing.duplicate-signer",
        "configuration-signing.signer-id",
        "configuration-signing.signer-roles",
        "configuration-signing.signer-keys",
        "configuration-signing.key-pin",
    ] {
        assert!(found.contains(&id), "{id} missing from {found:?}");
    }
}

#[test]
fn a_v2_policy_must_be_satisfiable() {
    let mut too_few = v2_policy();
    too_few.required_approvals = 3;
    assert!(ids(&check_policy(&too_few))
        .contains(&"configuration-signing.too-few-approvers"));
    let mut no_final = v2_policy();
    no_final.signers.retain(|s| s.id != "custodian");
    assert!(ids(&check_policy(&no_final))
        .contains(&"configuration-signing.no-final-signer"));
    let no_signers = ConfigurationSigning {
        signers: Vec::new(),
        ..v2_policy()
    };
    assert!(ids(&check_policy(&no_signers))
        .contains(&"configuration-signing.no-signers"));
}

#[test]
fn the_policy_digest_ignores_order_and_changes_with_what_matters() {
    let policy = v2_policy();
    let digest = policy_sha256(&policy).unwrap();
    // Same policy, other order: same digest.
    let mut reordered = v2_policy();
    reordered.signers.reverse();
    reordered.signers[1].keys.reverse();
    assert_eq!(policy_sha256(&reordered).unwrap(), digest);
    // Anything that decides what imports changes it.
    let mut changes: Vec<ConfigurationSigning> = Vec::new();
    let mut c = v2_policy();
    c.required_approvals = 1;
    changes.push(c);
    let mut c = v2_policy();
    c.signers[0].keys[0].spki_sha256 = pin(20);
    changes.push(c);
    let mut c = v2_policy();
    c.signers[2].roles.push(SignerRole::Approver);
    changes.push(c);
    let mut c = v2_policy();
    c.final_signer_separation = Some(FinalSignerSeparation::Allowed);
    changes.push(c);
    let mut c = v2_policy();
    c.minimum_package_format = Some(PackageFormat::V1);
    changes.push(c);
    let mut c = v2_policy();
    c.signers[0].id = "approver-x".to_string();
    changes.push(c);
    for (index, changed) in changes.iter().enumerate() {
        assert_ne!(
            policy_sha256(changed).unwrap(),
            digest,
            "change {index} kept the digest"
        );
    }
    // A key label is a note, not policy.
    let mut labelled = v2_policy();
    labelled.signers[0].keys[0].label = "laptop".to_string();
    assert_eq!(policy_sha256(&labelled).unwrap(), digest);
}

#[test]
fn enrollment_finds_the_signer_of_each_key() {
    let policy = v2_policy();
    let enrollment = Enrollment::from_signers(&policy.signers).unwrap();
    assert_eq!(enrollment.signer_of(&pin(3)).unwrap().id, "approver-2");
    assert_eq!(enrollment.signer_of(&pin(2)).unwrap().id, "approver-2");
    assert!(enrollment.signer_of(&pin(99)).is_none());
}

#[test]
fn separation_needs_a_final_signer_with_enough_other_approvers() {
    // Exactly two approvers, and the only final signer is one of them:
    // a package could never have two approvals plus a separate final signer.
    let mut policy = v2_policy();
    policy.signers.retain(|s| s.id != "custodian");
    policy.signers[0].roles.push(SignerRole::FinalSigner);
    assert!(ids(&check_policy(&policy))
        .contains(&"configuration-signing.separation-unsatisfiable"));
    // The same dual-role signer is fine when separation is allowed...
    policy.final_signer_separation = Some(FinalSignerSeparation::Allowed);
    assert!(!ids(&check_policy(&policy))
        .contains(&"configuration-signing.separation-unsatisfiable"));
    // ...or when a third approver leaves two others.
    let mut three = v2_policy();
    three.signers.retain(|s| s.id != "custodian");
    three.signers[0].roles.push(SignerRole::FinalSigner);
    three.signers.push(signer(
        "approver-3",
        &[SignerRole::Approver],
        &[pin(30)],
    ));
    assert!(
        check_policy(&three).is_empty(),
        "{:?}",
        check_policy(&three)
    );
}

#[test]
fn the_policy_digest_ignores_root_order_wrapping_and_repeats() {
    use rcgen::{
        BasicConstraints, CertificateParams, DnType, IsCa, KeyPair,
        PKCS_ECDSA_P256_SHA256,
    };
    let root = |name: &str| {
        let mut params = CertificateParams::new(Vec::<String>::new()).unwrap();
        params.distinguished_name.push(DnType::CommonName, name);
        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        params
            .self_signed(
                &KeyPair::generate_for(&PKCS_ECDSA_P256_SHA256).unwrap(),
            )
            .unwrap()
            .pem()
    };
    let (a, b) = (root("Root A"), root("Root B"));
    let mut one = v2_policy();
    one.package_roots = format!("{a}{b}");
    let mut other = v2_policy();
    other.package_roots = format!("\n{b}\n\n{a}{b}");
    assert_eq!(policy_sha256(&one).unwrap(), policy_sha256(&other).unwrap());
    let mut fewer = v2_policy();
    fewer.package_roots = a.clone();
    assert_ne!(policy_sha256(&one).unwrap(), policy_sha256(&fewer).unwrap());
}
