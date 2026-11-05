// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use chrono::TimeZone;
use sequent_core::ballot_receipt::{
    sign_cast_statement, sign_received_ballot, verify_cast_receipt, ReceivedStatement,
};
use voting_crypto::signing::VerificationKey;

const ELECTION_ID: &str = "f2f1065e-b784-46d1-b81a-c71bfeb9ad55";
const ELECTION_EVENT_ID: &str = "33f18502-a67c-4853-8333-a58630663559";

struct Fixture {
    ballot_box_sk: SigningKey,
    voter_sk: SigningKey,
    received_ballot: ReceivedBallot,
}

impl Fixture {
    fn new() -> Self {
        let ballot_box_sk = SigningKey::generate().unwrap();
        let voter_sk = SigningKey::generate().unwrap();
        let received_at = Utc.timestamp_millis_opt(1_841_367_600_000).unwrap();
        let received = sign_received_ballot(
            &ballot_box_sk,
            ReceivedStatement {
                tenant_id: "90505c8a-23a9-4cdf-a26b-4e19f6a097d5".into(),
                election_event_id: ELECTION_EVENT_ID.into(),
                election_id: ELECTION_ID.into(),
                ballot_hash: "ballot-hash".into(),
                voter_signing_pk: VerificationKey::from_sk(&voter_sk)
                    .unwrap()
                    .to_b64_string()
                    .unwrap(),
                voter_ballot_signature: voter_sk.sign(b"ballot").unwrap().to_b64_string().unwrap(),
                received_at: format_signed_time(&received_at),
                key_id: published_key(&ballot_box_sk).unwrap().key_id,
            },
        )
        .unwrap();

        Self {
            ballot_box_sk,
            voter_sk,
            received_ballot: ReceivedBallot {
                id: Uuid::new_v4().to_string(),
                election_event_id: ELECTION_EVENT_ID.into(),
                election_id: ELECTION_ID.into(),
                area_id: Uuid::new_v4().to_string(),
                voter_id: "voter".into(),
                ballot_id: received.ballot_id,
                ballot_hash: received.statement.ballot_hash,
                content: "ciphertext".into(),
                voter_signing_pk: received.statement.voter_signing_pk,
                voter_ballot_signature: received.statement.voter_ballot_signature,
                received_at,
                key_id: received.statement.key_id,
                received_signature: received.received_signature,
                status: ReceivedStatus::Received,
                cast_at: None,
                cast_signature: None,
                cast_receipt_signature: None,
                vitrine: None,
            },
        }
    }

    fn cast_signature(&self) -> String {
        sign_cast_statement(&self.voter_sk, ELECTION_ID, &self.received_ballot.ballot_id).unwrap()
    }

    /// The ballot as the ballot box keeps it once cast with `cast_signature`.
    fn cast_with(mut self, cast_signature: &str) -> Self {
        let cast_at = Utc.timestamp_millis_opt(1_841_367_845_678).unwrap();
        let receipt = sign_cast_receipt(
            &self.ballot_box_sk,
            receipt_statement(
                &self.received_ballot,
                &cast_at,
                &self.received_ballot.key_id,
                cast_signature,
            ),
        )
        .unwrap();
        self.received_ballot.status = ReceivedStatus::Cast;
        self.received_ballot.cast_at = Some(cast_at);
        self.received_ballot.cast_signature = Some(cast_signature.into());
        self.received_ballot.cast_receipt_signature = Some(receipt.cast_receipt_signature);
        self
    }
}

#[test]
fn the_key_that_signed_the_ballot_casts_it() {
    let f = Fixture::new();
    let cast_signature = f.cast_signature();

    assert_eq!(
        check_cast_signature(&f.received_ballot, &cast_signature).unwrap(),
        cast_signature
    );
    assert_eq!(
        cast_or_stored_receipt(&f.received_ballot, &cast_signature).unwrap(),
        None
    );
}

#[test]
fn a_cast_signature_by_another_key_or_for_another_ballot_is_refused() {
    let f = Fixture::new();
    let another_voter = SigningKey::generate().unwrap();
    let ballot_id = f.received_ballot.ballot_id.clone();

    for (case, signature) in [
        (
            "another key",
            sign_cast_statement(&another_voter, ELECTION_ID, &ballot_id).unwrap(),
        ),
        (
            "another ballot",
            sign_cast_statement(&f.voter_sk, ELECTION_ID, "0000-0000").unwrap(),
        ),
        (
            "another election",
            sign_cast_statement(&f.voter_sk, "another-election", &ballot_id).unwrap(),
        ),
        (
            "the ballot's signature",
            f.received_ballot.voter_ballot_signature.clone(),
        ),
        ("not a signature", "not base64".to_string()),
        ("empty", String::new()),
    ] {
        assert!(
            matches!(
                check_cast_signature(&f.received_ballot, &signature),
                Err(CastVoteError::BallotCastSignatureFailed(_))
            ),
            "{case}"
        );
    }
}

#[test]
fn casting_again_with_the_same_signature_returns_the_stored_receipt() {
    let f = Fixture::new();
    let cast_signature = f.cast_signature();
    let f = f.cast_with(&cast_signature);

    let receipt = cast_or_stored_receipt(&f.received_ballot, &cast_signature)
        .unwrap()
        .unwrap();

    verify_cast_receipt(&published_key(&f.ballot_box_sk).unwrap(), &receipt).unwrap();
    assert_eq!(receipt.statement.cast_at, "2028-05-08T03:04:05.678Z");
    assert_eq!(receipt.statement.cast_signature, cast_signature);
    assert_eq!(receipt.statement.ballot_id, f.received_ballot.ballot_id);
    assert_eq!(receipt.statement.received_at, "2028-05-08T03:00:00.000Z");
}

#[test]
fn a_ballot_already_cast_is_not_cast_again() {
    let f = Fixture::new();
    let cast_signature = f.cast_signature();
    let another_signature = f
        .voter_sk
        .sign(b"another")
        .unwrap()
        .to_b64_string()
        .unwrap();
    let mut f = f.cast_with(&another_signature);

    assert!(matches!(
        cast_or_stored_receipt(&f.received_ballot, &cast_signature),
        Err(CastVoteError::BallotAlreadyCast)
    ));

    // Cast before the ballot box signed receipts: there is none to return.
    f.received_ballot.cast_receipt_signature = None;
    assert!(matches!(
        cast_or_stored_receipt(&f.received_ballot, &cast_signature),
        Err(CastVoteError::BallotAlreadyCast)
    ));
}

#[test]
fn an_audited_ballot_is_never_cast() {
    let mut f = Fixture::new();
    f.received_ballot.status = ReceivedStatus::Audited;

    assert!(matches!(
        cast_or_stored_receipt(&f.received_ballot, &f.cast_signature()),
        Err(CastVoteError::BallotAudited)
    ));
}

#[test]
fn a_cast_takes_the_received_ballot_and_signs_a_receipt_for_it() {
    let f = Fixture::new();
    let cast_signature = f.cast_signature();
    let (input, received_cast) = received_cast(
        f.received_ballot.clone(),
        cast_signature.clone(),
        f.ballot_box_sk.clone(),
    )
    .unwrap();

    assert_eq!(input.ballot_id, "ballot-hash");
    assert_eq!(input.election_id.to_string(), ELECTION_ID);
    assert_eq!(input.content, "ciphertext");

    let (cast_at, receipt) = sign_receipt(&received_cast).unwrap();
    verify_cast_receipt(&published_key(&f.ballot_box_sk).unwrap(), &receipt).unwrap();
    assert_eq!(receipt.statement.cast_at, format_signed_time(&cast_at));
    assert_eq!(receipt.statement.cast_signature, cast_signature);
    assert_eq!(cast_at.timestamp_subsec_nanos() % 1_000_000, 0);
}

#[test]
fn only_the_key_that_received_the_ballot_signs_its_cast_receipt() {
    let f = Fixture::new();
    let another_key = SigningKey::generate().unwrap();

    assert!(matches!(
        received_cast(f.received_ballot.clone(), f.cast_signature(), another_key),
        Err(CastVoteError::BallotSignFailed(_))
    ));
}
