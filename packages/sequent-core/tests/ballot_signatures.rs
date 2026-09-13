// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! A valid signature must bind the exact ballot, election and serialized
//! content. Each rejection starts from a signature that verifies successfully.

#![cfg(feature = "default_features")]
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![warn(private_interfaces, private_bounds, unnameable_types)]
#![deny(rustdoc::missing_crate_level_docs, rustdoc::broken_intra_doc_links)]
#![deny(
    clippy::missing_docs_in_private_items,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::doc_markdown,
    clippy::unwrap_used,
    clippy::panic,
    clippy::shadow_unrelated,
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::indexing_slicing,
    clippy::future_not_send,
    clippy::arithmetic_side_effects,
    clippy::suspicious,
    clippy::complexity,
    clippy::style,
    clippy::perf,
    clippy::pedantic
)]

use support::TestResult;

mod support;

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

fn signed_multi_ballot() -> TestResult<SignedHashableMultiBallot> {
    let ballot = unsigned_multi_ballot();
    let signature =
        sign_hashable_multi_ballot_with_ephemeral_voter_signing_key(
            BALLOT, ELECTION, &ballot,
        )?;
    Ok(SignedHashableMultiBallot {
        version: ballot.version,
        issue_date: ballot.issue_date,
        contests: ballot.contests,
        config: ballot.config,
        ballot_style_hash: ballot.ballot_style_hash,
        voter_signing_pk: Some(signature.public_key),
        voter_ballot_signature: Some(signature.signature),
    })
}

#[test]
fn multi_ballot_signatures_reject_replay_and_every_changed_signed_field(
) -> TestResult {
    let signed = signed_multi_ballot()?;
    assert!(verify_multi_ballot_signature(BALLOT, ELECTION, &signed)?.is_some());
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
        let mut altered = serde_json::to_value(&signed)?;
        let replacement = if field == "version" {
            json!(TYPES_VERSION + 1)
        } else {
            json!("tampered")
        };
        altered
            .as_object_mut()
            .ok_or("signed ballot must be an object")?
            .insert(field.into(), replacement);
        let altered = serde_json::from_value(altered)?;
        assert!(
            verify_multi_ballot_signature(BALLOT, ELECTION, &altered).is_err(),
            "signature did not bind {field}"
        );
    }
    Ok(())
}

#[test]
fn malformed_signature_material_is_an_error_not_an_unsigned_result(
) -> TestResult {
    let signed = signed_multi_ballot()?;
    for field in ["voter_signing_pk", "voter_ballot_signature"] {
        for malformed in ["", "!invalid-base64", "AAAA"] {
            let mut altered = serde_json::to_value(&signed)?;
            altered
                .as_object_mut()
                .ok_or("signed ballot must be an object")?
                .insert(field.into(), json!(malformed));
            let altered = serde_json::from_value(altered)?;
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
    assert!(
        verify_multi_ballot_signature(BALLOT, ELECTION, &unsigned)?.is_none()
    );
    Ok(())
}

#[test]
fn single_contest_signatures_bind_identity_and_content_too() -> TestResult {
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
    )?;
    let signed = SignedHashableBallot {
        version: ballot.version,
        issue_date: ballot.issue_date,
        contests: ballot.contests,
        config: ballot.config,
        ballot_style_hash: ballot.ballot_style_hash,
        voter_signing_pk: Some(signature.public_key),
        voter_ballot_signature: Some(signature.signature),
    };
    assert!(verify_ballot_signature(BALLOT, ELECTION, &signed)?.is_some());
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
        let mut altered = serde_json::to_value(&signed)?;
        let replacement = match field {
            "version" => json!(TYPES_VERSION + 1),
            "contests" => json!(["tampered"]),
            _ => json!("tampered"),
        };
        altered
            .as_object_mut()
            .ok_or("signed ballot must be an object")?
            .insert(field.into(), replacement);
        assert!(
            verify_ballot_signature(
                BALLOT,
                ELECTION,
                &serde_json::from_value(altered)?
            )
            .is_err(),
            "accepted changed {field}"
        );
    }
    Ok(())
}

fn ballot_input() -> TestResult<(BallotStyle, Vec<DecodedVoteContest>)> {
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
    }))?;
    Ok((style, vec![vote]))
}

#[test]
fn auditable_multi_ballots_preserve_selection_but_strip_private_replication_data(
) -> TestResult {
    let (style, votes) = ballot_input()?;
    let ballot = encrypt_decoded_multi_contest(&RistrettoCtx, &votes, &style)?;
    let decoded = map_to_decoded_multi_contest::<RistrettoCtx>(&ballot)?;
    assert_eq!(
        decoded
            .first()
            .ok_or("expected one contest in the ballot fixture")?
            .choices,
        votes
            .first()
            .ok_or("expected one contest in the ballot fixture")?
            .choices
    );

    let auditable = ballot.deserialize_contests::<RistrettoCtx>()?;
    let hashable = HashableMultiBallot::try_from(&ballot)?;
    let public = hashable.deserialize_contests::<RistrettoCtx>()?;
    assert_eq!(
        public.contest_ids,
        vec![votes
            .first()
            .ok_or("expected one contest in the ballot fixture")?
            .contest_id
            .clone()]
    );
    assert_eq!(public.ciphertext, auditable.choice.ciphertext);
    assert_eq!(public.proof, auditable.proof);
    let raw = RawHashableMultiBallot::<RistrettoCtx>::try_from(&hashable)?;
    assert_eq!(raw.contests, public);

    let signed = SignedHashableMultiBallot::try_from(&ballot)?;
    assert_eq!(signed.deserialize_contests::<RistrettoCtx>()?, public);
    assert_eq!(
        SignedHashableMultiBallot::serialize_contests(&public)?,
        hashable.contests
    );
    assert_eq!(
        AuditableMultiBallot::serialize_contests(&auditable)?,
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
    Ok(())
}

#[test]
fn single_contest_audits_reproduce_ciphertext_from_disclosed_randomness(
) -> TestResult {
    let (style, votes) = ballot_input()?;
    let ballot = encrypt_decoded_contest(&RistrettoCtx, &votes, &style)?;
    let original = ballot.deserialize_contests::<RistrettoCtx>()?;
    let reproduced = recreate_encrypt_cyphertext(&RistrettoCtx, &ballot)?;
    assert_eq!(
        reproduced
            .first()
            .ok_or("expected one contest in the ballot fixture")?,
        &original
            .first()
            .ok_or("expected one contest in the ballot fixture")?
            .choice
    );
    // The single-contest wire format uses candidate-id order. The input fixture
    // is deliberately unsorted, so compare with the independently sorted input.
    let mut expected_choices = votes
        .first()
        .ok_or("expected one contest in the ballot fixture")?
        .choices
        .clone();
    expected_choices.sort_by(|left, right| left.id.cmp(&right.id));
    assert_eq!(
        map_to_decoded_contest::<RistrettoCtx>(&ballot)?
            .first()
            .ok_or("decoded contest is missing")?
            .choices,
        expected_choices
    );
    assert_eq!(
        AuditableBallot::serialize_contests(&original)?,
        ballot.contests
    );

    let mut invalid = ballot.clone();
    invalid.config.public_key = None;
    assert!(recreate_encrypt_cyphertext(&RistrettoCtx, &invalid).is_err());
    invalid = ballot.clone();
    invalid.contests.clear();
    assert!(recreate_encrypt_cyphertext(&RistrettoCtx, &invalid).is_err());
    invalid = ballot;
    *invalid
        .contests
        .first_mut()
        .ok_or("serialized contest is missing")? = "!invalid".into();
    assert!(recreate_encrypt_cyphertext(&RistrettoCtx, &invalid).is_err());
    Ok(())
}
