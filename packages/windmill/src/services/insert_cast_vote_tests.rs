// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use sequent_core::ballot::{AuditableBallot, PublicKeyConfig};
use sequent_core::encrypt::{encrypt_decoded_contest, encrypt_decoded_multi_contest};
use sequent_core::fixtures::ballot_codec::{
    get_test_contest, get_test_decoded_vote_contest, get_writein_ballot_style,
};
use sequent_core::multi_ballot::AuditableMultiBallot;
use sequent_core::serialization::base64::Base64Serialize;
use strand::elgamal::PrivateKey;

const VOTER_ID: &str = "voter";

fn ballot_style() -> BallotStyle {
    BallotStyle {
        contests: vec![get_test_contest()],
        ..get_writein_ballot_style()
    }
}

fn style_with_other_public_key() -> BallotStyle {
    let private_key = PrivateKey::<RistrettoCtx>::gen(&RistrettoCtx);
    BallotStyle {
        public_key: Some(PublicKeyConfig {
            public_key: private_key.pk_element().serialize().unwrap(),
            is_demo: false,
        }),
        ..ballot_style()
    }
}

fn style_with_reordered_candidates() -> BallotStyle {
    let mut ballot_style = ballot_style();
    ballot_style.contests[0].candidates.reverse();
    ballot_style
}

fn published_style(ballot_style: &BallotStyle) -> PublishedBallotStyleHash {
    PublishedBallotStyleHash::from_ballot_eml(&serde_json::to_string(ballot_style).unwrap())
        .unwrap()
}

fn published_styles(ballot_style: &BallotStyle) -> Vec<PublishedBallotStyleHash> {
    vec![published_style(ballot_style)]
}

fn single_ballot(ballot_style: &BallotStyle) -> AuditableBallot {
    encrypt_decoded_contest::<RistrettoCtx>(
        &RistrettoCtx,
        &vec![get_test_decoded_vote_contest()],
        ballot_style,
    )
    .unwrap()
}

fn multi_ballot(ballot_style: &BallotStyle) -> AuditableMultiBallot {
    encrypt_decoded_multi_contest::<RistrettoCtx>(
        &RistrettoCtx,
        &vec![get_test_decoded_vote_contest()],
        ballot_style,
    )
    .unwrap()
}

fn single_input(ballot: &AuditableBallot) -> InsertCastVoteInput {
    let hashable = HashableBallot::try_from(ballot).unwrap();
    InsertCastVoteInput {
        ballot_id: hash_ballot(&hashable).unwrap(),
        election_id: Uuid::new_v4(),
        content: serde_json::to_string(&hashable).unwrap(),
    }
}

fn multi_input(ballot: &AuditableMultiBallot) -> InsertCastVoteInput {
    let hashable = HashableMultiBallot::try_from(ballot).unwrap();
    InsertCastVoteInput {
        ballot_id: hash_multi_ballot(&hashable).unwrap(),
        election_id: Uuid::new_v4(),
        content: serde_json::to_string(&hashable).unwrap(),
    }
}

fn legacy_single_input() -> InsertCastVoteInput {
    let mut ballot = single_ballot(&ballot_style());
    ballot.version = LEGACY_TYPES_VERSION;
    single_input(&ballot)
}

fn check_single(
    input: &InsertCastVoteInput,
    policy: &BallotTrackerPolicy,
    published_ballot_styles: &[PublishedBallotStyleHash],
) -> Result<(), CastVoteError> {
    deserialize_and_check_ballot(&input.content, VOTER_ID, policy, published_ballot_styles)
        .map(|_| ())
}

fn check_multi(
    input: &InsertCastVoteInput,
    published_ballot_styles: &[PublishedBallotStyleHash],
) -> Result<(), CastVoteError> {
    deserialize_and_check_multi_ballot(
        &input.content,
        VOTER_ID,
        &BallotTrackerPolicy::default(),
        published_ballot_styles,
    )
    .map(|_| ())
}

fn assert_style_mismatch(result: Result<(), CastVoteError>) {
    assert!(
        matches!(result, Err(CastVoteError::BallotStyleMismatch(_))),
        "expected a ballot style mismatch, got {result:?}"
    );
}

#[test]
fn ballot_tracker_policy_defaults_to_style_bound() {
    assert_eq!(
        BallotTrackerPolicy::default(),
        BallotTrackerPolicy::STYLE_BOUND
    );
}

#[test]
fn ballot_encrypted_with_the_published_style_is_accepted() {
    let ballot_style = ballot_style();
    let published = published_styles(&ballot_style);

    assert!(check_single(
        &single_input(&single_ballot(&ballot_style)),
        &BallotTrackerPolicy::default(),
        &published,
    )
    .is_ok());
    assert!(check_multi(&multi_input(&multi_ballot(&ballot_style)), &published).is_ok());
}

#[test]
fn ballot_matching_any_published_style_is_accepted() {
    let ballot_style = ballot_style();
    let published = vec![
        published_style(&style_with_other_public_key()),
        published_style(&ballot_style),
    ];

    assert!(check_single(
        &single_input(&single_ballot(&ballot_style)),
        &BallotTrackerPolicy::default(),
        &published,
    )
    .is_ok());
}

#[test]
fn ballot_encrypted_with_another_public_key_is_rejected() {
    let published = published_styles(&ballot_style());
    let other_style = style_with_other_public_key();

    assert_style_mismatch(check_single(
        &single_input(&single_ballot(&other_style)),
        &BallotTrackerPolicy::default(),
        &published,
    ));
    assert_style_mismatch(check_multi(
        &multi_input(&multi_ballot(&other_style)),
        &published,
    ));
}

#[test]
fn ballot_encrypted_with_reordered_candidates_is_rejected() {
    let published = published_styles(&ballot_style());
    let other_style = style_with_reordered_candidates();

    assert_style_mismatch(check_single(
        &single_input(&single_ballot(&other_style)),
        &BallotTrackerPolicy::default(),
        &published,
    ));
    assert_style_mismatch(check_multi(
        &multi_input(&multi_ballot(&other_style)),
        &published,
    ));
}

#[test]
fn ballot_without_a_published_style_is_rejected() {
    let ballot_style = ballot_style();

    assert_style_mismatch(check_single(
        &single_input(&single_ballot(&ballot_style)),
        &BallotTrackerPolicy::default(),
        &[],
    ));
    assert_style_mismatch(check_multi(&multi_input(&multi_ballot(&ballot_style)), &[]));
}

#[test]
fn legacy_ballot_is_rejected_by_default() {
    let published = published_styles(&ballot_style());

    assert_style_mismatch(check_single(
        &legacy_single_input(),
        &BallotTrackerPolicy::default(),
        &published,
    ));
}

#[test]
fn legacy_ballot_is_accepted_when_the_policy_allows_it() {
    assert!(check_single(
        &legacy_single_input(),
        &BallotTrackerPolicy::ALLOW_LEGACY,
        &[],
    )
    .is_ok());
}

#[test]
fn current_ballot_is_checked_when_the_policy_allows_legacy() {
    let published = published_styles(&ballot_style());

    assert_style_mismatch(check_single(
        &single_input(&single_ballot(&style_with_other_public_key())),
        &BallotTrackerPolicy::ALLOW_LEGACY,
        &published,
    ));
}
