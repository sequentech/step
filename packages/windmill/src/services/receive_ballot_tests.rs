// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use chrono::Duration;
use sequent_core::ballot_receipt::verify_received_ballot;
use voting_crypto::signing::VerificationKey;

fn envelope(public_key: Option<&str>, signature: Option<&str>) -> BallotEnvelope {
    BallotEnvelope {
        config: "style".into(),
        ballot_style_hash: "published-hash".into(),
        voter_signing_pk: public_key.map(str::to_owned),
        voter_ballot_signature: signature.map(str::to_owned),
    }
}

#[test]
fn only_events_with_signed_receipts_receive_ballots_and_never_by_telephone() {
    for channel in [
        VotingStatusChannel::ONLINE,
        VotingStatusChannel::KIOSK,
        VotingStatusChannel::EARLY_VOTING,
        VotingStatusChannel::TELEPHONE,
    ] {
        assert!(!must_be_received(&ReceiptsPolicy::DISABLED, channel));
        assert_eq!(
            must_be_received(&ReceiptsPolicy::SIGNED_BY_BALLOT_BOX, channel),
            channel != VotingStatusChannel::TELEPHONE,
            "{channel:?}"
        );
    }
}

#[test]
fn a_ballot_without_a_complete_voter_signature_is_refused() {
    assert_eq!(
        voter_signature(&envelope(Some("key"), Some("signature"))).unwrap(),
        ("key".to_string(), "signature".to_string())
    );
    for unsigned in [
        envelope(None, None),
        envelope(Some("key"), None),
        envelope(None, Some("signature")),
    ] {
        assert!(matches!(
            voter_signature(&unsigned),
            Err(CastVoteError::BallotVoterSignatureRequired)
        ));
    }
}

#[test]
fn a_ballot_of_another_ballot_style_is_refused() {
    let ballot = envelope(None, None);

    check_ballot_style_hash(&ballot, "published-hash").unwrap();
    assert!(matches!(
        check_ballot_style_hash(&ballot, "another-hash"),
        Err(CastVoteError::BallotStyleMismatch(message))
            if message.contains("published-hash") && message.contains("another-hash")
    ));
}

#[test]
fn a_ballot_style_that_does_not_parse_is_an_internal_error() {
    assert!(matches!(
        hash_published_ballot_eml("{}"),
        Err(CastVoteError::CheckStatusInternalFailed(_))
    ));
}

#[test]
fn the_envelope_reads_single_and_multi_contest_ballots() {
    for contests in [r#"["contest"]"#, r#""contests""#] {
        let content = format!(
            r#"{{"version":1,"issue_date":"today","contests":{contests},"config":"style",
                "ballot_style_hash":"hash","voter_signing_pk":"key","voter_ballot_signature":null}}"#
        );
        let envelope: BallotEnvelope = deserialize_str(&content).unwrap();

        assert_eq!(envelope.config, "style");
        assert_eq!(envelope.ballot_style_hash, "hash");
        assert_eq!(envelope.voter_signing_pk.as_deref(), Some("key"));
        assert_eq!(envelope.voter_ballot_signature, None);
    }
}

#[test]
fn the_received_time_is_signed_to_the_millisecond() {
    let time = Utc.timestamp_nanos(1_841_367_600_007_654_321);

    assert_eq!(
        format_signed_time(&truncate_to_milliseconds(time).unwrap()),
        "2028-05-08T03:00:00.007Z"
    );
}

#[test]
fn signed_times_keep_milliseconds_and_utc() {
    assert_eq!(
        format_signed_time(&Utc.timestamp_millis_opt(1_841_367_600_007).unwrap()),
        "2028-05-08T03:00:00.007Z"
    );
    assert_eq!(
        format_signed_time(&Utc.timestamp_millis_opt(1_841_367_600_000).unwrap()),
        "2028-05-08T03:00:00.000Z"
    );
}

#[test]
fn a_stored_ballot_reads_back_as_the_receipt_the_ballot_box_signed() {
    let signing_key = SigningKey::generate().unwrap();
    let voter_key = SigningKey::generate().unwrap();
    let voter_signing_pk = VerificationKey::from_sk(&voter_key)
        .unwrap()
        .to_b64_string()
        .unwrap();
    let voter_ballot_signature = voter_key.sign(b"ballot").unwrap().to_b64_string().unwrap();
    let (tenant, event, election) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
    let scope = ReceivedBallotScope {
        tenant_id: &tenant,
        election_event_id: &event,
        election_id: &election,
        ballot_hash: "ballot-hash",
    };
    let received_at = truncate_to_milliseconds(Utc::now()).unwrap();
    let received = sign_received(
        &signing_key,
        &scope,
        &voter_signing_pk,
        &voter_ballot_signature,
        &received_at,
    )
    .unwrap();
    let stored = StoredReceivedBallot {
        id: Uuid::new_v4().to_string(),
        election_event_id: event.to_string(),
        election_id: election.to_string(),
        area_id: Uuid::new_v4().to_string(),
        voter_id: "voter".into(),
        ballot_id: received.ballot_id.clone(),
        ballot_hash: "ballot-hash".into(),
        content: "ciphertext".into(),
        voter_signing_pk,
        voter_ballot_signature,
        received_at,
        key_id: received.statement.key_id.clone(),
        received_signature: received.received_signature.clone(),
        status: electoral_log::adapters::ballot_box_received::ReceivedStatus::Received,
        cast_at: None,
        cast_signature: None,
        cast_receipt_signature: None,
        vitrine: None,
    };

    let read_back = stored_receipt(&tenant, stored);
    assert_eq!(read_back, received);
    verify_received_ballot(&published_key(&signing_key).unwrap(), &read_back).unwrap();
}

#[test]
fn the_ballot_box_signs_a_receipt_voters_can_check_with_its_published_key() {
    let signing_key = SigningKey::generate().unwrap();
    let voter_key = SigningKey::generate().unwrap();
    let voter_signing_pk = VerificationKey::from_sk(&voter_key)
        .unwrap()
        .to_b64_string()
        .unwrap();
    let voter_ballot_signature = voter_key.sign(b"ballot").unwrap().to_b64_string().unwrap();
    let (tenant, event, election) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
    let scope = ReceivedBallotScope {
        tenant_id: &tenant,
        election_event_id: &event,
        election_id: &election,
        ballot_hash: "ballot-hash",
    };
    let received_at = truncate_to_milliseconds(Utc::now()).unwrap();

    let received = sign_received(
        &signing_key,
        &scope,
        &voter_signing_pk,
        &voter_ballot_signature,
        &received_at,
    )
    .unwrap();
    let later = sign_received(
        &signing_key,
        &scope,
        &voter_signing_pk,
        &voter_ballot_signature,
        &(received_at + Duration::milliseconds(1)),
    )
    .unwrap();

    let key = published_key(&signing_key).unwrap();
    assert_eq!(
        verify_received_ballot(&key, &received).unwrap(),
        received.ballot_id
    );
    assert_eq!(received.statement.tenant_id, tenant.to_string());
    assert_eq!(received.statement.election_event_id, event.to_string());
    assert_eq!(received.statement.election_id, election.to_string());
    assert_eq!(received.statement.ballot_hash, "ballot-hash");
    assert_eq!(received.statement.key_id, key.key_id);
    assert_ne!(received.ballot_id, later.ballot_id);
}

#[test]
fn a_malformed_voter_key_cannot_be_signed_for() {
    let signing_key = SigningKey::generate().unwrap();
    let (tenant, event, election) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
    let scope = ReceivedBallotScope {
        tenant_id: &tenant,
        election_event_id: &event,
        election_id: &election,
        ballot_hash: "ballot-hash",
    };

    assert!(matches!(
        sign_received(
            &signing_key,
            &scope,
            "not a key",
            "not a signature",
            &Utc::now()
        ),
        Err(CastVoteError::BallotSignFailed(_))
    ));
}

#[test]
fn private_received_envelope_drops_unsigned_identity_fields_without_changing_hash() {
    use sequent_core::ballot::{HashableBallot, SignedHashableBallot};
    use sequent_core::encrypt::hash_ballot;
    use sequent_core::types::vote_privacy::BallotIdentity;
    let raw=serde_json::json!({"version":sequent_core::ballot::TYPES_VERSION,
        "issue_date":"2028-05-08T03:00:00Z","contests":[],"config":"published-style", "ballot_style_hash":"published-hash",
        "voter_signing_pk":"public-key","voter_ballot_signature":"signature",
        "voter_id":"private-account","ip":"198.51.100.7","user_agent":"private device"}).to_string();
    let before: SignedHashableBallot = serde_json::from_str(&raw).unwrap();
    let stored = storage_ballot_content(&raw, BallotIdentity::Pseudonym, false).unwrap();
    let after: SignedHashableBallot = serde_json::from_str(&stored).unwrap();
    assert_eq!(before, after);
    assert_eq!(
        hash_ballot(&HashableBallot::try_from(&before).unwrap()).unwrap(),
        hash_ballot(&HashableBallot::try_from(&after).unwrap()).unwrap()
    );
    for forbidden in ["voter_id", "198.51.100.7", "private-account", "user_agent"] {
        assert!(!stored.contains(forbidden));
    }
    assert_eq!(
        storage_ballot_content(&raw, BallotIdentity::VoterId, false).unwrap(),
        raw
    );
    let mut multi: serde_json::Value = serde_json::from_str(&raw).unwrap();
    multi["contests"] = serde_json::json!("opaque encrypted contests");
    let clean =
        storage_ballot_content(&multi.to_string(), BallotIdentity::Pseudonym, true).unwrap();
    let typed: sequent_core::multi_ballot::SignedHashableMultiBallot =
        serde_json::from_str(&clean).unwrap();
    assert_eq!(typed.contests, "opaque encrypted contests");
    assert!(!clean.contains("private-account"));
}

mod auditable_ballot_publication {
    use super::*;
    use sequent_core::ballot::{AuditableBallot, HashableBallot, SignedHashableBallot};
    use sequent_core::encrypt::{
        encrypt_decoded_contest, encrypt_decoded_multi_contest, hash_ballot, hash_multi_ballot,
    };
    use sequent_core::fixtures::ballot_codec::{
        get_test_contest, get_test_decoded_vote_contest, get_writein_ballot_style,
    };
    use sequent_core::multi_ballot::{
        AuditableMultiBallot, HashableMultiBallot, SignedHashableMultiBallot,
    };
    use voting_crypto::RistrettoCtx;

    const SINGLE: ContestEncryptionPolicy = ContestEncryptionPolicy::SINGLE_CONTEST;
    const MULTI: ContestEncryptionPolicy = ContestEncryptionPolicy::MULTIPLE_CONTESTS;

    fn style() -> BallotStyle {
        BallotStyle {
            contests: vec![get_test_contest()],
            ..get_writein_ballot_style()
        }
    }

    fn voter_key() -> String {
        VerificationKey::from_sk(&SigningKey::generate().unwrap())
            .unwrap()
            .to_b64_string()
            .unwrap()
    }

    /// A ballot the voter signed, as the portal keeps it.
    fn single_ballot() -> AuditableBallot {
        let mut ballot = encrypt_decoded_contest(
            &RistrettoCtx,
            &vec![get_test_decoded_vote_contest()],
            &style(),
        )
        .unwrap();
        ballot.voter_signing_pk = Some(voter_key());
        ballot.voter_ballot_signature = Some("signature".into());
        let signed = SignedHashableBallot::try_from(&ballot).unwrap();
        ballot.ballot_hash = hash_ballot(&HashableBallot::try_from(&signed).unwrap()).unwrap();
        ballot
    }

    fn multi_ballot() -> AuditableMultiBallot {
        let mut ballot = encrypt_decoded_multi_contest(
            &RistrettoCtx,
            &vec![get_test_decoded_vote_contest()],
            &style(),
        )
        .unwrap();
        ballot.voter_signing_pk = Some(voter_key());
        ballot.voter_ballot_signature = Some("signature".into());
        let signed = SignedHashableMultiBallot::try_from(&ballot).unwrap();
        ballot.ballot_hash =
            hash_multi_ballot(&HashableMultiBallot::try_from(&signed).unwrap()).unwrap();
        ballot
    }

    /// The ballot box's copy of a ballot: what the portal sent to it.
    fn stored(content: String, ballot_hash: &str, voter_signing_pk: &str) -> StoredReceivedBallot {
        StoredReceivedBallot {
            id: Uuid::new_v4().to_string(),
            election_event_id: Uuid::new_v4().to_string(),
            election_id: Uuid::new_v4().to_string(),
            area_id: Uuid::new_v4().to_string(),
            voter_id: "voter".into(),
            ballot_id: "FTBE-MHRX".into(),
            ballot_hash: ballot_hash.into(),
            content,
            voter_signing_pk: voter_signing_pk.into(),
            voter_ballot_signature: "signature".into(),
            received_at: Utc::now(),
            key_id: "box-key".into(),
            received_signature: "receipt".into(),
            status: electoral_log::adapters::ballot_box_received::ReceivedStatus::Audited,
            cast_at: None,
            cast_signature: None,
            cast_receipt_signature: None,
            vitrine: None,
        }
    }

    fn stored_single(ballot: &AuditableBallot) -> StoredReceivedBallot {
        let signed = SignedHashableBallot::try_from(ballot).unwrap();
        stored(
            serde_json::to_string(&signed).unwrap(),
            &ballot.ballot_hash,
            ballot.voter_signing_pk.as_deref().unwrap(),
        )
    }

    fn stored_multi(ballot: &AuditableMultiBallot) -> StoredReceivedBallot {
        let signed = SignedHashableMultiBallot::try_from(ballot).unwrap();
        stored(
            serde_json::to_string(&signed).unwrap(),
            &ballot.ballot_hash,
            ballot.voter_signing_pk.as_deref().unwrap(),
        )
    }

    #[test]
    fn the_stored_ballots_auditable_ballot_is_published_with_its_tracking_id() {
        let ballot = single_ballot();
        let received = stored_single(&ballot);
        let published =
            check_auditable_ballot(&serde_json::to_string(&ballot).unwrap(), &received, &SINGLE)
                .unwrap();
        let read_back: AuditableBallot = serde_json::from_str(&published.content).unwrap();
        assert_eq!(read_back, ballot);
        assert_eq!(
            published.tracking_id,
            sequent_core::ballot_receipt::tracking_id(
                &ballot.ballot_hash,
                ballot.voter_signing_pk.as_deref()
            )
            .unwrap()
        );
    }

    #[test]
    fn a_multi_contest_ballot_is_checked_in_its_own_format() {
        let ballot = multi_ballot();
        let received = stored_multi(&ballot);
        let auditable = serde_json::to_string(&ballot).unwrap();
        let published = check_auditable_ballot(&auditable, &received, &MULTI).unwrap();
        let read_back: AuditableMultiBallot = serde_json::from_str(&published.content).unwrap();
        assert_eq!(read_back, ballot);
        assert!(matches!(
            check_auditable_ballot(&auditable, &received, &SINGLE),
            Err(AuditableBallotRefusal::Unreadable(_))
        ));
    }

    #[test]
    fn what_the_portal_added_is_not_published() {
        let ballot = single_ballot();
        let received = stored_single(&ballot);
        let mut sent = serde_json::to_value(&ballot).unwrap();
        sent["voter_id"] = "private-account".into();
        let published = check_auditable_ballot(&sent.to_string(), &received, &SINGLE).unwrap();
        assert!(!published.content.contains("private-account"));
    }

    #[test]
    fn another_ballot_is_not_published() {
        let ballot = single_ballot();
        let received = stored_single(&ballot);
        let other = single_ballot();
        assert_eq!(
            check_auditable_ballot(&serde_json::to_string(&other).unwrap(), &received, &SINGLE),
            Err(AuditableBallotRefusal::NotTheStoredBallot)
        );
    }

    #[test]
    fn other_ciphertexts_under_the_same_hash_are_not_published() {
        let ballot = single_ballot();
        let received = stored_single(&ballot);
        // Another encryption of the same choices, claiming the stored ballot's hash and key.
        let mut other = encrypt_decoded_contest(
            &RistrettoCtx,
            &vec![get_test_decoded_vote_contest()],
            &style(),
        )
        .unwrap();
        other.voter_signing_pk = ballot.voter_signing_pk.clone();
        other.voter_ballot_signature = ballot.voter_ballot_signature.clone();
        other.ballot_hash = ballot.ballot_hash.clone();
        assert_ne!(other.contests, ballot.contests);
        assert_eq!(
            check_auditable_ballot(&serde_json::to_string(&other).unwrap(), &received, &SINGLE),
            Err(AuditableBallotRefusal::NotTheStoredBallot)
        );
    }

    #[test]
    fn a_ballot_whose_hash_is_not_its_own_is_not_published() {
        let mut ballot = single_ballot();
        ballot.ballot_hash = "0".repeat(64);
        let received = stored_single(&ballot);
        assert_eq!(
            check_auditable_ballot(&serde_json::to_string(&ballot).unwrap(), &received, &SINGLE),
            Err(AuditableBallotRefusal::NotTheStoredBallot)
        );
    }

    #[test]
    fn a_ballot_of_another_voter_key_is_not_published() {
        let ballot = single_ballot();
        let mut received = stored_single(&ballot);
        received.voter_signing_pk = voter_key();
        assert_eq!(
            check_auditable_ballot(&serde_json::to_string(&ballot).unwrap(), &received, &SINGLE),
            Err(AuditableBallotRefusal::NotTheStoredBallot)
        );
    }

    #[test]
    fn an_unreadable_ballot_is_not_published() {
        let received = stored_single(&single_ballot());
        for sent in ["", "not json", "{}", r#"{"ballot_hash":"ballot-hash"}"#] {
            assert!(
                matches!(
                    check_auditable_ballot(sent, &received, &SINGLE),
                    Err(AuditableBallotRefusal::Unreadable(_))
                ),
                "{sent}"
            );
        }
    }
}
