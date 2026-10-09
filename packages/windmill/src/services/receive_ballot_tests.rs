// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use sequent_core::ballot_receipt::verify_received_ballot;
use strand::signature::StrandSignaturePk;

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
        format_received_at(&truncate_to_milliseconds(time).unwrap()),
        "2028-05-08T03:00:00.007Z"
    );
}

#[test]
fn the_ballot_box_signs_a_receipt_voters_can_check_with_its_published_key() {
    let signing_key = StrandSignatureSk::generate().unwrap();
    let voter_key = StrandSignatureSk::generate().unwrap();
    let voter_signing_pk = StrandSignaturePk::from_sk(&voter_key)
        .unwrap()
        .to_der_b64_string()
        .unwrap();
    let voter_ballot_signature = voter_key.sign(b"ballot").unwrap().to_b64_string().unwrap();
    let (tenant, event, election) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
    let scope = ReceivedBallotScope {
        tenant_id: &tenant,
        election_event_id: &event,
        election_id: &election,
        voter_id: "voter",
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
    let signing_key = StrandSignatureSk::generate().unwrap();
    let (tenant, event, election) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
    let scope = ReceivedBallotScope {
        tenant_id: &tenant,
        election_event_id: &event,
        election_id: &election,
        voter_id: "voter",
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
