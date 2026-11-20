// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The offline check of WebAuthn bundles, with assertions made here by
//! software P-256 keys standing in for security keys. These are test
//! fixtures only: no key or roster here is trusted anywhere.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use ring::digest::{digest, SHA256};
use ring::rand::SystemRandom;
use ring::signature::{EcdsaKeyPair, KeyPair, ECDSA_P256_SHA256_ASN1_SIGNING};

use super::*;

const RP: &str = "architect.ems.example";
const ORIGIN: &str = "https://architect.ems.example";
const CONFIGURATION: &str = "00000000-0000-0000-0000-000000000007";
const CONTENT: &str =
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const EXTERNAL_ID: &str = "ov-2028";

fn manifest() -> ManifestRevision<'static> {
    ManifestRevision {
        external_id: EXTERNAL_ID,
        revision: 3,
        content_sha256: CONTENT,
    }
}

fn b64(bytes: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(bytes)
}

/// A software key standing in for one official's security key.
struct Official {
    subject: &'static str,
    key: String,
    role: Role,
    credential: Vec<u8>,
    pair: EcdsaKeyPair,
}

fn official(subject: &'static str, role: Role) -> Official {
    let rng = SystemRandom::new();
    let pkcs8 =
        EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, &rng)
            .unwrap();
    let pair = EcdsaKeyPair::from_pkcs8(
        &ECDSA_P256_SHA256_ASN1_SIGNING,
        pkcs8.as_ref(),
        &rng,
    )
    .unwrap();
    Official {
        subject,
        key: format!("key-{subject}"),
        role,
        credential: format!("credential-{subject}").into_bytes(),
        pair,
    }
}

fn roster(officials: &[&Official]) -> Roster {
    let text = serde_json::json!({
        "format": ROSTER_FORMAT,
        "rp_id": RP,
        "origins": [ORIGIN],
        "tenant": "t",
        "keys": officials.iter().map(|o| serde_json::json!({
            "key": o.key,
            "credential_id": b64(&o.credential),
            "subject": o.subject,
            "role": o.role,
            "algorithm": "es256",
            "public_key": b64(o.pair.public_key().as_ref()),
            "record_sha256": "b".repeat(64),
        })).collect::<Vec<_>>(),
    })
    .to_string();
    Roster::read(&text).unwrap()
}

/// What a browser and key would give, with each part open to change.
struct Ceremony {
    kind: &'static str,
    origin: &'static str,
    flags: u8,
    rp: &'static str,
    extra: &'static str,
}

const HONEST: Ceremony = Ceremony {
    kind: "webauthn.get",
    origin: ORIGIN,
    flags: 0x05,
    rp: RP,
    extra: "",
};

fn assert_with(
    official: &Official,
    binding: Binding,
    ceremony: &Ceremony,
) -> Proof {
    let nonce = [9u8; 32];
    let challenge = binding.challenge(&nonce);
    let client_data = format!(
        "{{\"type\":\"{}\",\"challenge\":\"{}\",\"origin\":\"{}\"{}}}",
        ceremony.kind,
        b64(&challenge),
        ceremony.origin,
        ceremony.extra
    );
    let mut authenticator_data =
        digest(&SHA256, ceremony.rp.as_bytes()).as_ref().to_vec();
    authenticator_data.push(ceremony.flags);
    authenticator_data.extend_from_slice(&7u32.to_be_bytes());
    let mut message = authenticator_data.clone();
    message.extend_from_slice(digest(&SHA256, client_data.as_bytes()).as_ref());
    let signature = official.pair.sign(&SystemRandom::new(), &message).unwrap();
    Proof {
        binding,
        nonce: b64(&nonce),
        credential_id: b64(&official.credential),
        authenticator_data: b64(&authenticator_data),
        client_data_json: b64(client_data.as_bytes()),
        signature: b64(signature.as_ref()),
    }
}

fn binding(official: &Official, action: Action, content: &str) -> Binding {
    Binding {
        rp_id: RP.to_string(),
        tenant: "t".to_string(),
        subject: official.subject.to_string(),
        action,
        configuration: CONFIGURATION.to_string(),
        revision: 3,
        content_sha256: content.to_string(),
        signer: official.key.clone(),
    }
}

fn approve(official: &Official) -> Proof {
    assert_with(
        official,
        binding(official, Action::ApproveRevision, CONTENT),
        &HONEST,
    )
}

/// A bundle of `approvers`' approvals, sealed by `signer`.
fn bundle(approvers: &[&Official], signer: &Official) -> Bundle {
    let mut bundle = Bundle {
        format: BUNDLE_FORMAT.to_string(),
        configuration: CONFIGURATION.to_string(),
        external_id: EXTERNAL_ID.to_string(),
        revision: 3,
        content_sha256: CONTENT.to_string(),
        approvals: approvers.iter().map(|o| approve(o)).collect(),
        final_signature: approve(signer),
    };
    let statement = bundle.statement().sha256();
    bundle.final_signature = assert_with(
        signer,
        binding(signer, Action::SignPackage, &statement),
        &HONEST,
    );
    bundle
}

struct World {
    a: Official,
    b: Official,
    c: Official,
    signer: Official,
}

fn world() -> World {
    World {
        a: official("u-a", Role::Approver),
        b: official("u-b", Role::Approver),
        c: official("u-c", Role::Approver),
        signer: official("u-signer", Role::FinalSigner),
    }
}

impl World {
    fn roster(&self) -> Roster {
        roster(&[&self.a, &self.b, &self.c, &self.signer])
    }
}

#[test]
fn verifies_two_distinct_approvals_sealed_by_a_separate_final_signer() {
    let w = world();
    let verified =
        verify_bundle(&bundle(&[&w.a, &w.b], &w.signer), &w.roster(), 2)
            .unwrap();
    assert_eq!(
        verified.approvers,
        vec!["u-a".to_string(), "u-b".to_string()]
    );
    assert_eq!(verified.final_signer, "u-signer");
}

#[test]
fn refuses_too_few_approvals_or_the_same_approver_twice() {
    let w = world();
    assert!(matches!(
        verify_bundle(&bundle(&[&w.a], &w.signer), &w.roster(), 2),
        Err(ProofError::TooFewApprovals {
            needed: 2,
            given: 1
        })
    ));
    assert!(matches!(
        verify_bundle(&bundle(&[&w.a, &w.a], &w.signer), &w.roster(), 2),
        Err(ProofError::Duplicate(_))
    ));
    assert!(matches!(
        verify_bundle(&bundle(&[], &w.signer), &w.roster(), 0),
        Err(ProofError::TooFewApprovals {
            needed: 1,
            given: 0
        })
    ));
}

#[test]
fn refuses_an_approval_or_a_seal_by_the_wrong_role() {
    let w = world();
    // The final signer can't approve; an approver can't seal.
    assert!(matches!(
        verify_bundle(&bundle(&[&w.a, &w.signer], &w.c), &w.roster(), 2),
        Err(ProofError::WrongKey(_))
    ));
    assert!(matches!(
        verify_bundle(&bundle(&[&w.a, &w.b], &w.c), &w.roster(), 2),
        Err(ProofError::WrongKey(_))
    ));
}

#[test]
fn refuses_a_seal_by_someone_who_approved() {
    // One person on the roster as both is refused by the roster itself.
    let a = official("u-a", Role::Approver);
    let mut also = official("u-a", Role::FinalSigner);
    also.key = "key-u-a-signer".to_string();
    let text = serde_json::to_string(&roster(&[&a])).unwrap();
    let mut doubled: Roster = serde_json::from_str(&text).unwrap();
    doubled.keys.push(roster(&[&also]).keys[0].clone());
    assert!(matches!(
        Roster::read(&serde_json::to_string(&doubled).unwrap()),
        Err(ProofError::Roster(_))
    ));
}

#[test]
fn refuses_a_seal_over_other_approvals_than_the_bundle_carries() {
    let w = world();
    let mut sealed = bundle(&[&w.a, &w.b], &w.signer);
    // Swap an approval after sealing: the statement no longer matches.
    sealed.approvals[1] = approve(&w.c);
    assert!(matches!(
        verify_bundle(&sealed, &w.roster(), 2),
        Err(ProofError::Binding("content"))
    ));
}

#[test]
fn refuses_a_proof_for_another_revision_content_or_configuration() {
    let w = world();
    let roster = w.roster();
    let proof = approve(&w.a);
    let check = |configuration: &str, revision: i64, content: &str| {
        verify_proof(
            &proof,
            &roster,
            Action::ApproveRevision,
            configuration,
            revision,
            content,
        )
    };
    assert!(check(CONFIGURATION, 3, CONTENT).is_ok());
    assert_eq!(
        check(CONFIGURATION, 4, CONTENT),
        Err(ProofError::Binding("revision"))
    );
    assert_eq!(
        check(CONFIGURATION, 0, CONTENT),
        Err(ProofError::Binding("revision"))
    );
    assert_eq!(
        check("another", 3, CONTENT),
        Err(ProofError::Binding("revision"))
    );
    assert_eq!(
        check(CONFIGURATION, 3, &"c".repeat(64)),
        Err(ProofError::Binding("content"))
    );
    assert_eq!(
        verify_proof(
            &proof,
            &roster,
            Action::SignPackage,
            CONFIGURATION,
            3,
            CONTENT
        ),
        Err(ProofError::Binding("action"))
    );
}

#[test]
fn refuses_a_binding_edited_after_signing() {
    let w = world();
    let roster = w.roster();
    let mut proof = approve(&w.a);
    // Claiming another revision changes the challenge the signature covered.
    proof.binding.revision = 4;
    assert!(matches!(
        verify_proof(
            &proof,
            &roster,
            Action::ApproveRevision,
            CONFIGURATION,
            4,
            CONTENT
        ),
        Err(ProofError::ClientData(_))
    ));
}

#[test]
fn refuses_what_the_browser_or_key_should_not_have_said() {
    let w = world();
    let roster = w.roster();
    let cases: [(&str, Ceremony); 9] = [
        (
            "a registration",
            Ceremony {
                kind: "webauthn.create",
                ..HONEST
            },
        ),
        (
            "another origin",
            Ceremony {
                origin: "https://evil.example",
                ..HONEST
            },
        ),
        (
            "a cross-origin frame",
            Ceremony {
                extra: ",\"crossOrigin\":true",
                ..HONEST
            },
        ),
        (
            "a top origin",
            Ceremony {
                extra: ",\"topOrigin\":\"https://evil.example\"",
                ..HONEST
            },
        ),
        (
            "another relying party",
            Ceremony {
                rp: "evil.example",
                ..HONEST
            },
        ),
        (
            "no user verification",
            Ceremony {
                flags: 0x01,
                ..HONEST
            },
        ),
        (
            "no user presence",
            Ceremony {
                flags: 0x04,
                ..HONEST
            },
        ),
        (
            "a backup-eligible key",
            Ceremony {
                flags: 0x0d,
                ..HONEST
            },
        ),
        (
            "attested data in an assertion",
            Ceremony {
                flags: 0x45,
                ..HONEST
            },
        ),
    ];
    for (what, ceremony) in cases {
        let proof = assert_with(
            &w.a,
            binding(&w.a, Action::ApproveRevision, CONTENT),
            &ceremony,
        );
        assert!(
            verify_proof(
                &proof,
                &roster,
                Action::ApproveRevision,
                CONFIGURATION,
                3,
                CONTENT
            )
            .is_err(),
            "{what} was taken"
        );
    }
}

#[test]
fn refuses_a_signature_by_another_key_or_over_other_bytes() {
    let w = world();
    let roster = w.roster();
    let impostor = official("u-a", Role::Approver);
    let mut stolen = assert_with(
        &impostor,
        binding(&w.a, Action::ApproveRevision, CONTENT),
        &HONEST,
    );
    stolen.credential_id = b64(&w.a.credential);
    assert_eq!(
        verify_proof(
            &stolen,
            &roster,
            Action::ApproveRevision,
            CONFIGURATION,
            3,
            CONTENT
        ),
        Err(ProofError::Signature)
    );
    let mut tampered = approve(&w.a);
    let mut data = URL_SAFE_NO_PAD
        .decode(&tampered.authenticator_data)
        .unwrap();
    data[36] ^= 1; // the signature counter
    tampered.authenticator_data = b64(&data);
    assert_eq!(
        verify_proof(
            &tampered,
            &roster,
            Action::ApproveRevision,
            CONFIGURATION,
            3,
            CONTENT
        ),
        Err(ProofError::Signature)
    );
}

#[test]
fn trusts_only_the_roster_it_was_given() {
    let w = world();
    let sealed = bundle(&[&w.a, &w.b], &w.signer);
    // A roster without one approver: the bundle can't add them.
    assert!(matches!(
        verify_bundle(&sealed, &roster(&[&w.a, &w.signer]), 2),
        Err(ProofError::UnknownKey(_))
    ));
    // Another relying party or tenant in the roster.
    let mut elsewhere = w.roster();
    elsewhere.tenant = "t2".to_string();
    assert!(matches!(
        verify_bundle(&sealed, &elsewhere, 2),
        Err(ProofError::Binding("tenant"))
    ));
    let mut other_origin = w.roster();
    other_origin.origins = vec!["https://other.example".to_string()];
    assert!(matches!(
        verify_bundle(&sealed, &other_origin, 2),
        Err(ProofError::ClientData(_))
    ));
}

#[test]
fn reads_only_a_well_formed_roster() {
    let w = world();
    let good = serde_json::to_value(w.roster()).unwrap();
    let mut cases = Vec::new();
    let mut wrong_format = good.clone();
    wrong_format["format"] = "sequent.webauthn-roster.v2".into();
    cases.push(wrong_format);
    let mut rs256 = good.clone();
    rs256["keys"][0]["algorithm"] = "rs256".into();
    cases.push(rs256);
    let mut short_key = good.clone();
    short_key["keys"][0]["public_key"] = b64(&[4u8; 33]).into();
    cases.push(short_key);
    let mut unknown_field = good.clone();
    unknown_field["trusted"] = true.into();
    cases.push(unknown_field);
    let mut no_origin = good.clone();
    no_origin["origins"] = serde_json::json!([]);
    cases.push(no_origin);
    for case in cases {
        assert!(Roster::read(&case.to_string()).is_err(), "{case}");
    }
    assert!(Roster::read(&good.to_string()).is_ok());
}

#[test]
fn refuses_a_bundle_of_another_format() {
    let w = world();
    let mut sealed = bundle(&[&w.a, &w.b], &w.signer);
    sealed.format = "sequent.webauthn-bundle.v0".to_string();
    assert_eq!(
        verify_bundle(&sealed, &w.roster(), 2),
        Err(ProofError::Format(BUNDLE_FORMAT))
    );
}

#[test]
fn verifies_a_bundle_for_exactly_the_revision_a_package_delivers() {
    let w = world();
    let sealed = bundle(&[&w.a, &w.b], &w.signer);
    let verified =
        verify_bundle_for_manifest(&sealed, &w.roster(), 2, manifest())
            .unwrap();
    assert_eq!(verified.final_signer, "u-signer");
    let other_content = "c".repeat(64);
    for (what, other, expected) in [
        (
            "another configuration",
            ManifestRevision {
                external_id: "ov-2032",
                ..manifest()
            },
            ProofError::Binding("package's configuration"),
        ),
        (
            "another revision",
            ManifestRevision {
                revision: 4,
                ..manifest()
            },
            ProofError::Binding("package's revision"),
        ),
        (
            "other content",
            ManifestRevision {
                content_sha256: &other_content,
                ..manifest()
            },
            ProofError::Binding("package's content"),
        ),
    ] {
        assert_eq!(
            verify_bundle_for_manifest(&sealed, &w.roster(), 2, other),
            Err(expected),
            "{what}"
        );
    }
}

#[test]
fn refuses_a_bundle_whose_package_identity_was_edited_after_sealing() {
    let w = world();
    let mut sealed = bundle(&[&w.a, &w.b], &w.signer);
    // Renamed to fit another package: the seal covered the old name.
    sealed.external_id = "ov-2032".to_string();
    let renamed = ManifestRevision {
        external_id: "ov-2032",
        ..manifest()
    };
    assert_eq!(
        verify_bundle_for_manifest(&sealed, &w.roster(), 2, renamed),
        Err(ProofError::Binding("content"))
    );
}

#[test]
fn refuses_approvals_or_a_seal_replayed_from_another_revision() {
    let w = world();
    // An approval of revision 2, replayed into revision 3's bundle.
    let mut stale = bundle(&[&w.a, &w.b], &w.signer);
    let mut earlier = binding(&w.b, Action::ApproveRevision, CONTENT);
    earlier.revision = 2;
    stale.approvals[1] = assert_with(&w.b, earlier, &HONEST);
    assert_eq!(
        verify_bundle(&stale, &w.roster(), 2),
        Err(ProofError::Binding("revision"))
    );
    // A seal of another bundle (other approvals), replayed onto this one.
    let mut resealed = bundle(&[&w.a, &w.b], &w.signer);
    resealed.final_signature = bundle(&[&w.a, &w.c], &w.signer).final_signature;
    assert_eq!(
        verify_bundle(&resealed, &w.roster(), 2),
        Err(ProofError::Binding("content"))
    );
}

#[test]
fn reads_only_the_roster_the_verifier_was_provisioned_with() {
    let w = world();
    let text = serde_json::to_string(&w.roster()).unwrap();
    let pinned = Roster::sha256(&text);
    assert_eq!(Roster::read_pinned(&text, &pinned).unwrap(), w.roster());
    assert_eq!(
        Roster::read_pinned(&text, &pinned.to_ascii_uppercase()).unwrap(),
        w.roster()
    );
    // A roster with one more key, or any other bytes: not the provisioned one.
    let mut grown = w.roster();
    grown
        .keys
        .push(roster(&[&official("u-z", Role::Approver)]).keys[0].clone());
    let grown = serde_json::to_string(&grown).unwrap();
    assert!(matches!(
        Roster::read_pinned(&grown, &pinned),
        Err(ProofError::Roster(_))
    ));
    assert!(matches!(
        Roster::read_pinned(&format!("{text} "), &pinned),
        Err(ProofError::Roster(_))
    ));
}
