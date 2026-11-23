// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

#![cfg(feature = "election_config_signing")]

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use ring::{
    digest::{digest, SHA256},
    rand::SystemRandom,
    signature::{EcdsaKeyPair, KeyPair, ECDSA_P256_SHA256_ASN1_SIGNING},
};
use sequent_core::election_config::webauthn_proof::{
    verify_proof, Action, Binding, Proof, ProofError, Role, Roster, RosterKey,
    ROSTER_FORMAT,
};

const RP: &str = "architect.example.test";
const ORIGIN: &str = "https://architect.example.test";
const CONFIGURATION: &str = "configuration-fixture";
const CONTENT: &str =
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

fn assertion() -> (Proof, Roster) {
    let rng = SystemRandom::new();
    let pkcs8 =
        EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, &rng)
            .unwrap();
    let key = EcdsaKeyPair::from_pkcs8(
        &ECDSA_P256_SHA256_ASN1_SIGNING,
        pkcs8.as_ref(),
        &rng,
    )
    .unwrap();
    let binding = Binding {
        rp_id: RP.into(),
        tenant: "tenant-fixture".into(),
        subject: "approver-fixture".into(),
        action: Action::ApproveRevision,
        configuration: CONFIGURATION.into(),
        revision: 1,
        content_sha256: CONTENT.into(),
        signer: "ephemeral-signing-key".into(),
    };
    let credential = URL_SAFE_NO_PAD.encode(b"synthetic-credential");
    let roster = Roster {
        format: ROSTER_FORMAT.into(),
        rp_id: RP.into(),
        origins: vec![ORIGIN.into()],
        tenant: binding.tenant.clone(),
        keys: vec![RosterKey {
            key: binding.signer.clone(),
            credential_id: credential.clone(),
            subject: binding.subject.clone(),
            role: Role::Approver,
            algorithm: "es256".into(),
            public_key: URL_SAFE_NO_PAD.encode(key.public_key().as_ref()),
            record_sha256: CONTENT.into(),
        }],
    };
    let nonce = [1; 32];
    let client_data = serde_json::to_vec(&serde_json::json!({
        "type": "webauthn.get",
        "challenge": URL_SAFE_NO_PAD.encode(binding.challenge(&nonce)),
        "origin": ORIGIN
    }))
    .unwrap();
    let mut authenticator = digest(&SHA256, RP.as_bytes()).as_ref().to_vec();
    authenticator.push(0x05); // User present and verified; no extension bytes.
    authenticator.extend_from_slice(&1_u32.to_be_bytes());
    let mut signed = authenticator.clone();
    signed.extend_from_slice(digest(&SHA256, &client_data).as_ref());
    let proof = Proof {
        binding,
        nonce: URL_SAFE_NO_PAD.encode(nonce),
        credential_id: credential,
        authenticator_data: URL_SAFE_NO_PAD.encode(authenticator),
        client_data_json: URL_SAFE_NO_PAD.encode(client_data),
        signature: URL_SAFE_NO_PAD
            .encode(key.sign(&rng, &signed).unwrap().as_ref()),
    };
    (
        proof,
        Roster::read(&serde_json::to_string(&roster).unwrap()).unwrap(),
    )
}

#[test]
fn the_receiver_requires_the_exact_relying_party_and_authenticator_layout() {
    let (proof, roster) = assertion();
    let verify = |proof| {
        verify_proof(
            proof,
            &roster,
            Action::ApproveRevision,
            CONFIGURATION,
            1,
            CONTENT,
        )
    };
    assert!(verify(&proof).is_ok());

    let mut wrong_party = proof.clone();
    wrong_party.binding.rp_id = "other.example.test".into();
    assert_eq!(
        verify(&wrong_party),
        Err(ProofError::Binding("relying party"))
    );

    let bytes = URL_SAFE_NO_PAD.decode(&proof.authenticator_data).unwrap();
    let mut truncated = proof.clone();
    truncated.authenticator_data =
        URL_SAFE_NO_PAD.encode(&bytes[..bytes.len() - 1]);
    assert_eq!(
        verify(&truncated),
        Err(ProofError::AuthenticatorData("too short"))
    );

    let mut trailing = proof.clone();
    let mut extra = bytes;
    extra.push(0);
    trailing.authenticator_data = URL_SAFE_NO_PAD.encode(extra);
    assert_eq!(
        verify(&trailing),
        Err(ProofError::AuthenticatorData("trailing bytes"))
    );
}

#[test]
fn unreadable_assertion_fields_remain_distinct_from_signature_failure() {
    let (proof, roster) = assertion();
    let verify = |proof| {
        verify_proof(
            proof,
            &roster,
            Action::ApproveRevision,
            CONFIGURATION,
            1,
            CONTENT,
        )
    };
    let mut encoded_client_data = proof.clone();
    encoded_client_data.client_data_json = "=".into();
    assert_eq!(
        verify(&encoded_client_data),
        Err(ProofError::Encoding("the client data"))
    );

    let mut malformed_json = proof.clone();
    malformed_json.client_data_json = URL_SAFE_NO_PAD.encode(b"{");
    assert!(matches!(
        verify(&malformed_json),
        Err(ProofError::ClientData(_))
    ));

    let mut malformed_challenge = proof.clone();
    let mut client_data: serde_json::Value = serde_json::from_slice(
        &URL_SAFE_NO_PAD.decode(&proof.client_data_json).unwrap(),
    )
    .unwrap();
    client_data["challenge"] = "=".into();
    malformed_challenge.client_data_json =
        URL_SAFE_NO_PAD.encode(serde_json::to_vec(&client_data).unwrap());
    assert_eq!(
        verify(&malformed_challenge),
        Err(ProofError::Encoding("the client data challenge"))
    );

    let mut malformed_nonce = proof.clone();
    malformed_nonce.nonce = "=".into();
    assert_eq!(
        verify(&malformed_nonce),
        Err(ProofError::Encoding("the nonce"))
    );

    let mut wrong_nonce_size = proof.clone();
    wrong_nonce_size.nonce = URL_SAFE_NO_PAD.encode([1; 31]);
    assert_eq!(
        verify(&wrong_nonce_size),
        Err(ProofError::Encoding("the nonce"))
    );

    let mut malformed_authenticator = proof.clone();
    malformed_authenticator.authenticator_data = "//".into();
    assert_eq!(
        verify(&malformed_authenticator),
        Err(ProofError::Encoding("the authenticator data"))
    );

    let mut malformed_signature = proof;
    malformed_signature.signature = "=".into();
    assert_eq!(
        verify(&malformed_signature),
        Err(ProofError::Encoding("the signature"))
    );
}

#[test]
fn malformed_provisioned_keys_are_not_usable() {
    let (proof, mut roster) = assertion();
    roster.keys[0].public_key = "=".into();
    let encoded = serde_json::to_string(&roster).unwrap();
    assert!(matches!(
        Roster::read(&encoded),
        Err(ProofError::Encoding("a roster public key"))
    ));

    // A caller supplying an already decoded roster gets a typed refusal too.
    assert_eq!(
        verify_proof(
            &proof,
            &roster,
            Action::ApproveRevision,
            CONFIGURATION,
            1,
            CONTENT,
        ),
        Err(ProofError::Encoding("the roster public key"))
    );
}
