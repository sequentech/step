// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use crate::postgres::received_ballot::StoredReceivedBallot;
use chrono::{TimeZone, Utc};
use sequent_core::ballot_receipt::{
    sign_cast_receipt, sign_cast_statement, sign_received_ballot, verify_cast_receipt,
    ReceivedStatement,
};
use strand::signature::{StrandSignaturePk, StrandSignatureSk};

const ELECTION_ID: &str = "f2f1065e-b784-46d1-b81a-c71bfeb9ad55";

struct Fixture {
    ballot_box_sk: StrandSignatureSk,
    voter_sk: StrandSignatureSk,
    received_ballot: ReceivedBallotToCast,
}

impl Fixture {
    fn new() -> Self {
        let ballot_box_sk = StrandSignatureSk::generate().unwrap();
        let voter_sk = StrandSignatureSk::generate().unwrap();
        let received = sign_received_ballot(
            &ballot_box_sk,
            ReceivedStatement {
                tenant_id: "90505c8a-23a9-4cdf-a26b-4e19f6a097d5".into(),
                election_event_id: "33f18502-a67c-4853-8333-a58630663559".into(),
                election_id: ELECTION_ID.into(),
                ballot_hash: "ballot-hash".into(),
                voter_signing_pk: StrandSignaturePk::from_sk(&voter_sk)
                    .unwrap()
                    .to_der_b64_string()
                    .unwrap(),
                voter_ballot_signature: voter_sk.sign(b"ballot").unwrap().to_b64_string().unwrap(),
                received_at: "2028-05-08T03:00:00.000Z".into(),
                key_id: published_key(&ballot_box_sk).unwrap().key_id,
            },
        )
        .unwrap();

        Self {
            ballot_box_sk,
            voter_sk,
            received_ballot: ReceivedBallotToCast {
                stored: StoredReceivedBallot {
                    id: Uuid::new_v4(),
                    status: ReceivedBallotStatus::Received,
                    received,
                },
                content: "ciphertext".into(),
                cast: None,
            },
        }
    }

    fn cast_signature(&self) -> String {
        sign_cast_statement(
            &self.voter_sk,
            ELECTION_ID,
            &self.received_ballot.stored.received.ballot_id,
        )
        .unwrap()
    }

    /// The ballot as the ballot box keeps it once cast with `cast_signature`.
    fn cast_with(mut self, cast_signature: &str) -> Self {
        let cast_at = Utc.timestamp_millis_opt(1_841_367_845_678).unwrap();
        let mut receipt = stored_receipt(
            &self.received_ballot,
            &StoredCast {
                cast_at,
                cast_signature: cast_signature.into(),
                cast_receipt_signature: String::new(),
            },
        );
        receipt = sign_cast_receipt(&self.ballot_box_sk, receipt.statement).unwrap();
        self.received_ballot.stored.status = ReceivedBallotStatus::Cast;
        self.received_ballot.cast = Some(StoredCast {
            cast_at,
            cast_signature: cast_signature.into(),
            cast_receipt_signature: receipt.cast_receipt_signature,
        });
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
    let another_voter = StrandSignatureSk::generate().unwrap();
    let ballot_id = f.received_ballot.stored.received.ballot_id.clone();

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
            f.received_ballot
                .stored
                .received
                .statement
                .voter_ballot_signature
                .clone(),
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
    assert_eq!(
        receipt.statement.ballot_id,
        f.received_ballot.stored.received.ballot_id
    );
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
    f.received_ballot.cast = None;
    assert!(matches!(
        cast_or_stored_receipt(&f.received_ballot, &cast_signature),
        Err(CastVoteError::BallotAlreadyCast)
    ));
}

#[test]
fn an_audited_ballot_is_never_cast() {
    let mut f = Fixture::new();
    f.received_ballot.stored.status = ReceivedBallotStatus::Audited;

    assert!(matches!(
        cast_or_stored_receipt(&f.received_ballot, &f.cast_signature()),
        Err(CastVoteError::BallotAudited)
    ));
}
