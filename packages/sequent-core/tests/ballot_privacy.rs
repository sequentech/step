// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
#![cfg(feature = "default_features")]

use sequent_core::ballot::BallotBoxKey;
use sequent_core::ballot_privacy::{
    verify_ballot_log_proof, BallotPrivacyProof, BallotPrivacyTrust,
    CheckStatus,
};

#[test]
fn absent_entries_wait_but_malformed_entries_fail() {
    let trusted = BallotPrivacyTrust {
        tenant_id: "tenant".into(),
        election_event_id: "event".into(),
        election_id: "election".into(),
        system_public_key: "invalid".into(),
        ballot_box_key: BallotBoxKey {
            key_id: "key".into(),
            public_key: "invalid".into(),
        },
    };
    let mut proof = BallotPrivacyProof {
        ballot_id: "ABCD-EFGH".into(),
        tracking_id: None,
        stage: None,
        pseudonym: "7QKM-2HXD".into(),
        voting_channel: "online".into(),
        received_entry: None,
        cast_entry: None,
    };
    let report = verify_ballot_log_proof(&trusted, &proof);
    assert_eq!(report.checks.binding, CheckStatus::Pending);
    assert_eq!(report.checks.received, CheckStatus::Pending);
    assert_eq!(report.checks.ballot_id, CheckStatus::Pending);
    assert_eq!(report.checks.cast, CheckStatus::Pending);
    proof.received_entry = Some("not base64".into());
    let report = verify_ballot_log_proof(&trusted, &proof);
    assert_eq!(report.checks.binding, CheckStatus::Failed);
    assert_eq!(report.checks.received, CheckStatus::Failed);
    assert_eq!(report.checks.ballot_id, CheckStatus::Failed);
    assert_eq!(report.checks.cast, CheckStatus::Pending);
}

use base64::{engine::general_purpose::STANDARD, Engine};
use borsh::BorshDeserialize;
use electoral_log::messages::message::Message;
use electoral_log::messages::privacy::{
    BallotCastEvidence, BallotReceivedEvidence,
};
use electoral_log::messages::statement::StatementBody;
use sequent_core::ballot::{
    sign_hashable_ballot_with_voter_signing_key, HashableBallot,
    SignedHashableBallot, TYPES_VERSION,
};
use sequent_core::ballot_privacy::pseudonym_hash;
use sequent_core::ballot_receipt::{
    ballot_box_key, sign_cast_receipt, sign_cast_statement,
    sign_received_ballot, CastReceiptStatement, ReceivedStatement,
};
use sequent_core::encrypt::hash_ballot;
use voting_crypto::{
    signing::{SigningKey, VerificationKey},
    RecordSerialize,
};

const PSEUDONYM: &str = "7QKM-2HXD";
const PSEUDONYM_HASH: &str = "1573bf00e3ac06ad5d81019102939444c4883b181e8acd25399461cb4fa33c5ebe67c517439f61a22029d9e0c968aa75af3e8dca872b6bc844ddb3eebfb66e0e";
fn key(seed: u8) -> SigningKey {
    let mut bytes = vec![
        0x30, 0x2e, 0x02, 0x01, 0x00, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70,
        0x04, 0x22, 0x04, 0x20,
    ];
    bytes.extend([seed; 32]);
    SigningKey::from_b64_string(&STANDARD.encode(bytes)).unwrap()
}
fn encode(message: &Message) -> String {
    STANDARD.encode(message.record_serialize().unwrap())
}
fn decode(encoded: &str) -> Message {
    Message::try_from_slice(&STANDARD.decode(encoded).unwrap()).unwrap()
}
fn fixture() -> (BallotPrivacyTrust, BallotPrivacyProof) {
    fixture_for_format(false)
}
fn fixture_for_format(multi: bool) -> (BallotPrivacyTrust, BallotPrivacyProof) {
    let system = key(3);
    let voter = key(2);
    let ballot_box = key(1);
    let published =
        ballot_box_key(&VerificationKey::from_sk(&ballot_box).unwrap())
            .unwrap();
    let (hash, signed, encrypted) = if multi {
        use sequent_core::ballot::BallotStyle;
        use sequent_core::encrypt::{
            encrypt_decoded_multi_contest, hash_multi_ballot,
        };
        use sequent_core::fixtures::ballot_codec::{
            get_test_contest, get_test_decoded_vote_contest,
            get_writein_ballot_style,
        };
        use sequent_core::multi_ballot::{
            sign_hashable_multi_ballot_with_voter_signing_key,
            HashableMultiBallot, SignedHashableMultiBallot,
        };
        let style = BallotStyle {
            contests: vec![get_test_contest()],
            ..get_writein_ballot_style()
        };
        let encrypted = encrypt_decoded_multi_contest(
            &cryptography::context::RistrettoCtx,
            &vec![get_test_decoded_vote_contest()],
            &style,
        )
        .unwrap();
        let hashable = HashableMultiBallot::try_from(&encrypted).unwrap();
        let hash = hash_multi_ballot(&hashable).unwrap();
        let signed = sign_hashable_multi_ballot_with_voter_signing_key(
            &voter, &hash, "election", &hashable,
        )
        .unwrap();
        let mut encrypted =
            SignedHashableMultiBallot::try_from(&encrypted).unwrap();
        encrypted.voter_signing_pk = Some(signed.public_key.clone());
        encrypted.voter_ballot_signature = Some(signed.signature.clone());
        (hash, signed, serde_json::to_string(&encrypted).unwrap())
    } else {
        let ballot = HashableBallot {
            version: TYPES_VERSION,
            issue_date: "2028-05-08T03:00:00Z".into(),
            contests: vec![],
            config: "style".into(),
            ballot_style_hash: "style-hash".into(),
        };
        let hash = hash_ballot(&ballot).unwrap();
        let signed = sign_hashable_ballot_with_voter_signing_key(
            &voter, &hash, "election", &ballot,
        )
        .unwrap();
        let encrypted = SignedHashableBallot {
            version: ballot.version,
            issue_date: ballot.issue_date,
            contests: ballot.contests,
            config: ballot.config,
            ballot_style_hash: ballot.ballot_style_hash,
            voter_signing_pk: Some(signed.public_key.clone()),
            voter_ballot_signature: Some(signed.signature.clone()),
        };
        (hash, signed, serde_json::to_string(&encrypted).unwrap())
    };
    let received = sign_received_ballot(
        &ballot_box,
        ReceivedStatement {
            tenant_id: "tenant".into(),
            election_event_id: "event".into(),
            election_id: "election".into(),
            ballot_hash: hash,
            voter_signing_pk: signed.public_key.clone(),
            voter_ballot_signature: signed.signature,
            received_at: "2028-05-08T03:00:00.000Z".into(),
            key_id: published.key_id.clone(),
        },
    )
    .unwrap();
    let cast_signature =
        sign_cast_statement(&voter, "election", &received.ballot_id).unwrap();
    let cast = sign_cast_receipt(
        &ballot_box,
        CastReceiptStatement {
            election_event_id: "event".into(),
            election_id: "election".into(),
            ballot_id: received.ballot_id.clone(),
            received_at: received.statement.received_at.clone(),
            cast_at: "2028-05-08T03:00:02.000Z".into(),
            key_id: published.key_id.clone(),
            cast_signature,
        },
    )
    .unwrap();
    let received_message = Message::ballot_received_message(
        PSEUDONYM,
        BallotReceivedEvidence {
            tenant_id: "tenant".into(),
            election_event_id: "event".into(),
            election_id: "election".into(),
            pseudonym_hash: PSEUDONYM_HASH.into(),
            voter_signing_pk: signed.public_key,
            received_ballot_json: serde_json::to_string(&received).unwrap(),
            encrypted_ballot: encrypted,
        },
        &system,
    )
    .unwrap();
    let cast_message = Message::privacy_cast_vote_message(
        PSEUDONYM,
        BallotCastEvidence {
            tenant_id: "tenant".into(),
            election_event_id: "event".into(),
            election_id: "election".into(),
            pseudonym_hash: PSEUDONYM_HASH.into(),
            ballot_id: received.ballot_id.clone(),
            cast_receipt_json: serde_json::to_string(&cast).unwrap(),
            voting_channel: "online".into(),
        },
        &system,
    )
    .unwrap();
    (
        BallotPrivacyTrust {
            tenant_id: "tenant".into(),
            election_event_id: "event".into(),
            election_id: "election".into(),
            system_public_key: VerificationKey::from_sk(&system)
                .unwrap()
                .to_b64_string()
                .unwrap(),
            ballot_box_key: published,
        },
        BallotPrivacyProof {
            ballot_id: received.ballot_id,
            tracking_id: None,
            stage: None,
            pseudonym: PSEUDONYM.into(),
            voting_channel: "online".into(),
            received_entry: Some(encode(&received_message)),
            cast_entry: Some(encode(&cast_message)),
        },
    )
}

#[test]
fn verifies_real_voter_receipt_and_authority_signatures() {
    let (trusted, proof) = fixture();
    let checks = verify_ballot_log_proof(&trusted, &proof).checks;
    assert_eq!(checks.binding, CheckStatus::Passed);
    assert_eq!(checks.received, CheckStatus::Passed);
    assert_eq!(checks.ballot_id, CheckStatus::Passed);
    assert_eq!(checks.cast, CheckStatus::Passed);
}

#[test]
fn browser_json_contract_preserves_scope_and_optional_evidence() {
    let trust: BallotPrivacyTrust = serde_json::from_value(serde_json::json!({
        "tenant_id": "tenant", "election_event_id": "event", "election_id": "election",
        "system_public_key": "published-system-key",
        "ballot_box_key": {"key_id": "box-key-id", "public_key": "published-box-key"}
    })).unwrap();
    let proof: BallotPrivacyProof = serde_json::from_value(serde_json::json!({
        "ballot_id": "FTBE-MHRX", "pseudonym": PSEUDONYM, "voting_channel": "online"
    })).unwrap();
    assert_eq!(trust.tenant_id, "tenant");
    assert_eq!(trust.ballot_box_key.key_id, "box-key-id");
    let json = serde_json::to_value(&proof).unwrap();
    assert_eq!(json["received_entry"], serde_json::Value::Null);
    assert_eq!(json["cast_entry"], serde_json::Value::Null);
    assert_eq!(json["pseudonym"], PSEUDONYM);
    let mut missing_scope = serde_json::to_value(&trust).unwrap();
    missing_scope.as_object_mut().unwrap().remove("tenant_id");
    assert!(
        serde_json::from_value::<BallotPrivacyTrust>(missing_scope).is_err()
    );
    let report = verify_ballot_log_proof(&trust, &proof);
    assert_eq!(
        serde_json::to_value(report).unwrap(),
        serde_json::json!({
            "checks": {"binding": "pending", "received": "pending", "ballotId": "pending", "cast": "pending"}
        })
    );
}

#[test]
fn browser_report_uses_camel_case_ballot_id_and_explicit_check_states() {
    use sequent_core::ballot_privacy::BallotPrivacyReport;
    for (wire, status) in [
        ("passed", CheckStatus::Passed),
        ("failed", CheckStatus::Failed),
        ("pending", CheckStatus::Pending),
    ] {
        let wire = serde_json::json!({"checks": {"binding": wire, "received": wire, "ballotId": wire, "cast": wire}});
        let report: BallotPrivacyReport =
            serde_json::from_value(wire.clone()).unwrap();
        assert_eq!(report.checks.binding, status);
        assert_eq!(report.checks.ballot_id, status);
        assert_eq!(serde_json::to_value(report).unwrap(), wire);
    }
    for invalid in [
        serde_json::json!({"binding": "passed", "received": "passed", "ballot_id": "passed", "cast": "passed"}),
        serde_json::json!({"binding": "unknown", "received": "passed", "ballotId": "passed", "cast": "passed"}),
        serde_json::json!({"binding": "passed", "received": "passed", "ballotId": "passed"}),
    ] {
        assert!(serde_json::from_value::<BallotPrivacyReport>(
            serde_json::json!({"checks": invalid})
        )
        .is_err());
    }
}

#[test]
fn a_valid_cast_without_its_received_entry_is_pending_and_oversized_evidence_fails_closed(
) {
    let (trusted, mut proof) = fixture();
    proof.received_entry = None;
    let checks = verify_ballot_log_proof(&trusted, &proof).checks;
    assert_eq!(checks.binding, CheckStatus::Pending);
    assert_eq!(checks.cast, CheckStatus::Pending);
    proof.received_entry = Some("A".repeat(2 * 1024 * 1024 + 1));
    let checks = verify_ballot_log_proof(&trusted, &proof).checks;
    assert_eq!(checks.binding, CheckStatus::Failed);
    assert_eq!(checks.received, CheckStatus::Failed);
    assert_eq!(checks.cast, CheckStatus::Failed);
}

#[test]
fn canonical_pseudonym_hash_has_an_independent_vector() {
    assert_eq!(pseudonym_hash(PSEUDONYM).as_deref(), Some(PSEUDONYM_HASH));
    for text in [
        "7qkm-2hxd",
        "7QKM2HXD",
        "7QKM 2HXD",
        "7QKM-2HXD ",
        "7QKM-2HXDx",
    ] {
        assert_eq!(pseudonym_hash(text), None, "{text}");
    }
}

#[test]
fn changed_signed_channel_fails_only_cast() {
    let (trusted, mut proof) = fixture();
    let mut message = decode(proof.cast_entry.as_deref().unwrap());
    if let StatementBody::PrivacyCastVote(ref mut evidence) =
        message.statement.body
    {
        evidence.voting_channel = "kiosk".into();
    }
    proof.cast_entry = Some(encode(&message));
    proof.voting_channel = "kiosk".into();
    let checks = verify_ballot_log_proof(&trusted, &proof).checks;
    assert_eq!(checks.binding, CheckStatus::Passed);
    assert_eq!(checks.received, CheckStatus::Passed);
    assert_eq!(checks.ballot_id, CheckStatus::Passed);
    assert_eq!(checks.cast, CheckStatus::Failed);
}

#[test]
fn display_metadata_cannot_substitute_a_pseudonym_or_channel() {
    let (trusted, proof) = fixture();
    let mut other = proof.clone();
    other.pseudonym = "ABCD-EFGH".into();
    assert_eq!(
        verify_ballot_log_proof(&trusted, &other).checks.binding,
        CheckStatus::Failed
    );
    let mut other = proof.clone();
    other.voting_channel = "kiosk".into();
    assert_eq!(
        verify_ballot_log_proof(&trusted, &other).checks.cast,
        CheckStatus::Failed
    );
    let mut other = proof;
    other.ballot_id = "ABCD-EFGH".into();
    assert_eq!(
        verify_ballot_log_proof(&trusted, &other).checks.ballot_id,
        CheckStatus::Failed
    );
}

#[test]
fn published_keys_and_tenant_event_election_are_independent_trust_inputs() {
    let (trusted, proof) = fixture();
    for field in ["tenant", "event", "election", "system", "box"] {
        let mut wrong = trusted.clone();
        match field {
            "tenant" => wrong.tenant_id = "another".into(),
            "event" => wrong.election_event_id = "another".into(),
            "election" => wrong.election_id = "another".into(),
            "system" => {
                wrong.system_public_key = VerificationKey::from_sk(&key(8))
                    .unwrap()
                    .to_b64_string()
                    .unwrap()
            }
            _ => {
                wrong.ballot_box_key =
                    ballot_box_key(&VerificationKey::from_sk(&key(8)).unwrap())
                        .unwrap()
            }
        }
        let checks = verify_ballot_log_proof(&wrong, &proof).checks;
        assert_eq!(checks.cast, CheckStatus::Failed, "{field}");
        assert_ne!(
            (checks.binding, checks.received),
            (CheckStatus::Passed, CheckStatus::Passed),
            "{field}"
        );
    }
}

#[test]
fn a_received_entry_can_verify_while_cast_is_still_pending() {
    let (trusted, mut proof) = fixture();
    proof.cast_entry = None;
    let checks = verify_ballot_log_proof(&trusted, &proof).checks;
    assert_eq!(checks.binding, CheckStatus::Passed);
    assert_eq!(checks.received, CheckStatus::Passed);
    assert_eq!(checks.cast, CheckStatus::Pending);
    proof.cast_entry = Some("".into());
    assert_eq!(
        verify_ballot_log_proof(&trusted, &proof).checks.cast,
        CheckStatus::Failed
    );
}

#[test]
fn the_tracking_id_shown_is_the_one_the_signed_ballot_gives() {
    use electoral_log::messages::statement::StatementBody;
    let (trusted, mut proof) = fixture();
    let message = decode(proof.received_entry.as_ref().unwrap());
    let StatementBody::BallotReceived(evidence) = message.statement.body else {
        panic!("a received entry")
    };
    let receipt: sequent_core::ballot_receipt::ReceivedBallot =
        serde_json::from_str(&evidence.received_ballot_json).unwrap();
    let computed = sequent_core::ballot_receipt::tracking_id(
        &receipt.statement.ballot_hash,
        Some(&receipt.statement.voter_signing_pk),
    )
    .unwrap();
    proof.tracking_id = Some(computed.clone());
    proof.stage = Some("cast".into());
    let checks = verify_ballot_log_proof(&trusted, &proof).checks;
    assert_eq!(checks.ballot_id, CheckStatus::Passed);
    assert_eq!(checks.cast, CheckStatus::Passed);
    let other = if computed.starts_with('0') { "1" } else { "0" };
    proof.tracking_id = Some(format!("{other}{}", &computed[1..]));
    let checks = verify_ballot_log_proof(&trusted, &proof).checks;
    assert_eq!(checks.ballot_id, CheckStatus::Failed);
    assert_eq!(checks.received, CheckStatus::Passed);
}

#[test]
fn authority_signature_does_not_replace_the_voter_ballot_check() {
    let (trusted, mut proof) = fixture();
    let message = decode(proof.received_entry.as_deref().unwrap());
    let StatementBody::BallotReceived(mut evidence) = message.statement.body
    else {
        panic!("fixture");
    };
    let mut encrypted: serde_json::Value =
        serde_json::from_str(&evidence.encrypted_ballot).unwrap();
    encrypted["issue_date"] = "2028-05-09T03:00:00Z".into();
    evidence.encrypted_ballot = serde_json::to_string(&encrypted).unwrap();
    proof.received_entry = Some(encode(
        &Message::ballot_received_message(PSEUDONYM, evidence, &key(3))
            .unwrap(),
    ));
    let checks = verify_ballot_log_proof(&trusted, &proof).checks;
    assert_eq!(checks.binding, CheckStatus::Passed);
    assert_eq!(checks.received, CheckStatus::Failed);
}

#[test]
fn canonical_entry_bytes_reject_trailing_data() {
    let (trusted, mut proof) = fixture();
    let mut bytes = STANDARD
        .decode(proof.received_entry.as_deref().unwrap())
        .unwrap();
    bytes.push(0);
    proof.received_entry = Some(STANDARD.encode(bytes));
    assert_eq!(
        verify_ballot_log_proof(&trusted, &proof).checks.binding,
        CheckStatus::Failed
    );
}

#[test]
fn verifies_the_actual_multi_contest_ballot_signature() {
    let (trusted, proof) = fixture_for_format(true);
    let checks = verify_ballot_log_proof(&trusted, &proof).checks;
    assert_eq!(checks.binding, CheckStatus::Passed);
    assert_eq!(checks.received, CheckStatus::Passed);
    assert_eq!(checks.ballot_id, CheckStatus::Passed);
    assert_eq!(checks.cast, CheckStatus::Passed);
}

/// Generate the synthetic browser fixture using the production canonical
/// encoder. The JSON contains only the fixed test keys and encrypted fixture.
#[test]
#[ignore = "writes the browser fixture named by BALLOT_PRIVACY_FIXTURE_PATH"]
fn export_browser_fixture() {
    let (trusted, mut proof) = fixture();
    for entry in [&mut proof.received_entry, &mut proof.cast_entry] {
        let mut message = decode(entry.as_deref().unwrap());
        message.statement.head.timestamp = 1_841_367_600;
        let signed = Message::sign(
            message.statement,
            None,
            &key(3),
            PSEUDONYM,
            &key(3),
            None,
            Some(PSEUDONYM.into()),
            message.election_id,
            None,
            message.ballot_id,
        )
        .unwrap();
        *entry = Some(encode(&signed));
    }
    let verified = verify_ballot_log_proof(&trusted, &proof);
    assert_eq!(verified.checks.received, CheckStatus::Passed);
    assert_eq!(verified.checks.cast, CheckStatus::Passed);
    let path = std::env::var("BALLOT_PRIVACY_FIXTURE_PATH")
        .expect("fixture destination");
    std::fs::write(
        path,
        serde_json::to_vec_pretty(
            &serde_json::json!({"trusted":trusted,"proof":proof}),
        )
        .unwrap(),
    )
    .unwrap();
}
