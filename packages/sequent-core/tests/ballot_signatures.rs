// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! A valid signature must bind the exact ballot, election and serialized
//! content. Each rejection starts from a signature that verifies successfully.

#![cfg(feature = "default_features")]

use sequent_core::ballot::{
    sign_hashable_ballot_with_ephemeral_voter_signing_key,
    verify_ballot_signature, AuditableBallot, BallotStyle, HashableBallot,
    SignedHashableBallot, TYPES_VERSION,
};
use sequent_core::encrypt::{
    encrypt_decoded_contest, encrypt_decoded_multi_contest,
    recreate_encrypt_cyphertext, DEFAULT_PUBLIC_KEY_RISTRETTO_STR,
};
use sequent_core::fixtures::ballot_codec::get_test_contest;
use sequent_core::multi_ballot::{
    sign_hashable_multi_ballot_with_ephemeral_voter_signing_key,
    verify_multi_ballot_signature, AuditableMultiBallot, HashableMultiBallot,
    RawHashableMultiBallot, SignedHashableMultiBallot,
};
use sequent_core::plaintext::{
    map_to_decoded_contest, map_to_decoded_multi_contest, DecodedVoteChoice,
    DecodedVoteContest,
};
use serde_json::json;
use strand::backend::ristretto::RistrettoCtx;

const BALLOT: &str = "fixture-ballot";
const ELECTION: &str = "fixture-election";

fn unsigned_multi_ballot() -> HashableMultiBallot {
    HashableMultiBallot {
        version: TYPES_VERSION,
        issue_date: "2026-09-12T10:00:00Z".into(),
        contests: "synthetic-serialized-contests".into(),
        config: "fixture-style".into(),
        ballot_style_hash: "synthetic-style-hash".into(),
    }
}

fn signed_multi_ballot() -> SignedHashableMultiBallot {
    let ballot = unsigned_multi_ballot();
    let signature =
        sign_hashable_multi_ballot_with_ephemeral_voter_signing_key(
            BALLOT, ELECTION, &ballot,
        )
        .unwrap();
    SignedHashableMultiBallot {
        version: ballot.version,
        issue_date: ballot.issue_date,
        contests: ballot.contests,
        config: ballot.config,
        ballot_style_hash: ballot.ballot_style_hash,
        voter_signing_pk: Some(signature.public_key),
        voter_ballot_signature: Some(signature.signature),
    }
}

#[test]
fn multi_ballot_signatures_reject_replay_and_every_changed_signed_field() {
    let signed = signed_multi_ballot();
    assert!(verify_multi_ballot_signature(BALLOT, ELECTION, &signed)
        .unwrap()
        .is_some());
    assert!(
        verify_multi_ballot_signature("another-ballot", ELECTION, &signed)
            .is_err()
    );
    assert!(
        verify_multi_ballot_signature(BALLOT, "another-election", &signed)
            .is_err()
    );

    for field in [
        "issue_date",
        "contests",
        "config",
        "ballot_style_hash",
        "version",
    ] {
        let mut altered = serde_json::to_value(&signed).unwrap();
        altered[field] = if field == "version" {
            json!(TYPES_VERSION + 1)
        } else {
            json!("tampered")
        };
        let altered = serde_json::from_value(altered).unwrap();
        assert!(
            verify_multi_ballot_signature(BALLOT, ELECTION, &altered).is_err(),
            "signature did not bind {field}"
        );
    }
}

#[test]
fn malformed_signature_material_is_an_error_not_an_unsigned_result() {
    let signed = signed_multi_ballot();
    for field in ["voter_signing_pk", "voter_ballot_signature"] {
        for malformed in ["", "!invalid-base64", "AAAA"] {
            let mut altered = serde_json::to_value(&signed).unwrap();
            altered[field] = json!(malformed);
            let altered = serde_json::from_value(altered).unwrap();
            assert!(
                verify_multi_ballot_signature(BALLOT, ELECTION, &altered)
                    .is_err(),
                "accepted malformed {field}"
            );
        }
    }

    // The helper reports absence; callers decide whether their protocol permits
    // unsigned ballots. None is deliberately distinct from a verified signature.
    let mut unsigned = signed;
    unsigned.voter_signing_pk = None;
    unsigned.voter_ballot_signature = None;
    assert!(verify_multi_ballot_signature(BALLOT, ELECTION, &unsigned)
        .unwrap()
        .is_none());
}

#[test]
fn single_contest_signatures_bind_identity_and_content_too() {
    let template = unsigned_multi_ballot();
    let ballot = HashableBallot {
        version: template.version,
        issue_date: template.issue_date,
        contests: vec![template.contests],
        config: template.config,
        ballot_style_hash: template.ballot_style_hash,
    };
    let signature = sign_hashable_ballot_with_ephemeral_voter_signing_key(
        BALLOT, ELECTION, &ballot,
    )
    .unwrap();
    let signed = SignedHashableBallot {
        version: ballot.version,
        issue_date: ballot.issue_date,
        contests: ballot.contests,
        config: ballot.config,
        ballot_style_hash: ballot.ballot_style_hash,
        voter_signing_pk: Some(signature.public_key),
        voter_ballot_signature: Some(signature.signature),
    };
    assert!(verify_ballot_signature(BALLOT, ELECTION, &signed)
        .unwrap()
        .is_some());
    assert!(
        verify_ballot_signature("another-ballot", ELECTION, &signed).is_err()
    );
    assert!(
        verify_ballot_signature(BALLOT, "another-election", &signed).is_err()
    );

    for field in [
        "issue_date",
        "contests",
        "config",
        "ballot_style_hash",
        "version",
        "voter_signing_pk",
        "voter_ballot_signature",
    ] {
        let mut altered = serde_json::to_value(&signed).unwrap();
        altered[field] = match field {
            "version" => json!(TYPES_VERSION + 1),
            "contests" => json!(["tampered"]),
            _ => json!("tampered"),
        };
        assert!(
            verify_ballot_signature(
                BALLOT,
                ELECTION,
                &serde_json::from_value(altered).unwrap()
            )
            .is_err(),
            "accepted changed {field}"
        );
    }
}

fn ballot_input() -> (BallotStyle, Vec<DecodedVoteContest>) {
    let contest = get_test_contest();
    let vote = DecodedVoteContest {
        contest_id: contest.id.clone(),
        is_explicit_invalid: false,
        is_decline_to_vote: false,
        is_blank_ballot: false,
        invalid_errors: vec![],
        invalid_alerts: vec![],
        choices: contest
            .candidates
            .iter()
            .enumerate()
            .map(|(index, candidate)| DecodedVoteChoice {
                id: candidate.id.clone(),
                selected: if index == 0 { 0 } else { -1 },
                write_in_text: None,
            })
            .collect(),
    };
    let style = serde_json::from_value(json!({
        "id": "fixture-style", "tenant_id": "fixture-tenant",
        "election_event_id": "fixture-event", "election_id": ELECTION,
        "area_id": "fixture-area", "contests": [contest],
        "public_key": {"public_key": DEFAULT_PUBLIC_KEY_RISTRETTO_STR, "is_demo": true}
    })).unwrap();
    (style, vec![vote])
}

#[test]
fn auditable_multi_ballots_preserve_selection_but_strip_private_replication_data(
) {
    let (style, votes) = ballot_input();
    let ballot =
        encrypt_decoded_multi_contest(&RistrettoCtx, &votes, &style).unwrap();
    let decoded =
        map_to_decoded_multi_contest::<RistrettoCtx>(&ballot).unwrap();
    assert_eq!(decoded[0].choices, votes[0].choices);

    let auditable = ballot.deserialize_contests::<RistrettoCtx>().unwrap();
    let hashable = HashableMultiBallot::try_from(&ballot).unwrap();
    let public = hashable.deserialize_contests::<RistrettoCtx>().unwrap();
    assert_eq!(public.contest_ids, vec![votes[0].contest_id.clone()]);
    assert_eq!(public.ciphertext, auditable.choice.ciphertext);
    assert_eq!(public.proof, auditable.proof);
    let raw =
        RawHashableMultiBallot::<RistrettoCtx>::try_from(&hashable).unwrap();
    assert_eq!(raw.contests, public);

    let signed = SignedHashableMultiBallot::try_from(&ballot).unwrap();
    assert_eq!(
        signed.deserialize_contests::<RistrettoCtx>().unwrap(),
        public
    );
    assert_eq!(
        SignedHashableMultiBallot::serialize_contests(&public).unwrap(),
        hashable.contests
    );
    assert_eq!(
        AuditableMultiBallot::serialize_contests(&auditable).unwrap(),
        ballot.contests
    );

    let mut invalid = ballot.clone();
    invalid.version = TYPES_VERSION + 1;
    assert!(HashableMultiBallot::try_from(&invalid).is_err());
    assert!(SignedHashableMultiBallot::try_from(&invalid).is_err());
    invalid = ballot;
    invalid.contests = "!invalid".into();
    assert!(HashableMultiBallot::try_from(&invalid).is_err());
    assert!(map_to_decoded_multi_contest::<RistrettoCtx>(&invalid).is_err());
}

#[test]
fn single_contest_audits_reproduce_ciphertext_from_disclosed_randomness() {
    let (style, votes) = ballot_input();
    let ballot =
        encrypt_decoded_contest(&RistrettoCtx, &votes, &style).unwrap();
    let original = ballot.deserialize_contests::<RistrettoCtx>().unwrap();
    let reproduced =
        recreate_encrypt_cyphertext(&RistrettoCtx, &ballot).unwrap();
    assert_eq!(reproduced[0], original[0].choice);
    // The single-contest wire format uses candidate-id order. The input fixture
    // is deliberately unsorted, so compare with the independently sorted input.
    let mut expected_choices = votes[0].choices.clone();
    expected_choices.sort_by(|left, right| left.id.cmp(&right.id));
    assert_eq!(
        map_to_decoded_contest::<RistrettoCtx>(&ballot).unwrap()[0].choices,
        expected_choices
    );
    assert_eq!(
        AuditableBallot::serialize_contests(&original).unwrap(),
        ballot.contests
    );

    let mut invalid = ballot.clone();
    invalid.config.public_key = None;
    assert!(recreate_encrypt_cyphertext(&RistrettoCtx, &invalid).is_err());
    invalid = ballot.clone();
    invalid.contests.clear();
    assert!(recreate_encrypt_cyphertext(&RistrettoCtx, &invalid).is_err());
    invalid = ballot;
    invalid.contests[0] = "!invalid".into();
    assert!(recreate_encrypt_cyphertext(&RistrettoCtx, &invalid).is_err());
}
