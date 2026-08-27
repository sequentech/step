// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Builds and encrypts a synthetic ballot from a `BallotStyle`, mirroring
//! `create_singular_ballot`/`create_multi_ballot` in
//! `beyond/packages/ivr-core/src/execution/phases/ballot_utils.rs:705-793`
//! (ported rather than imported — those functions are `pub(super)`).

use anyhow::{anyhow, Result};
use sequent_core::ballot::{
    sign_hashable_ballot_with_ephemeral_voter_signing_key, BallotStyle, Contest,
    ContestEncryptionPolicy, HashableBallot, SignedHashableBallot, VoterSigningPolicy,
};
use sequent_core::encrypt::{
    encrypt_decoded_contest, encrypt_decoded_multi_contest, hash_ballot, hash_multi_ballot,
};
use sequent_core::multi_ballot::{
    sign_hashable_multi_ballot_with_ephemeral_voter_signing_key, HashableMultiBallot,
    SignedHashableMultiBallot,
};
use sequent_core::plaintext::{
    map_to_decoded_contest, map_to_decoded_multi_contest, DecodedVoteChoice, DecodedVoteContest,
};
use strand::backend::ristretto::RistrettoCtx;

/// The two values `insert_cast_vote` needs on the wire.
pub struct PreparedVote {
    pub ballot_id: String,
    pub content: String,
}

/// One `DecodedVoteContest` per contest in `style`, each selecting the
/// first candidate that isn't an explicit-invalid/blank/write-in/
/// category-list placeholder. `encrypt_decoded_contest` requires exactly
/// one choice per candidate (selected or not) and exactly one contest per
/// entry in `style.contests`.
pub fn build_synthetic_contests(style: &BallotStyle) -> Vec<DecodedVoteContest> {
    style.contests.iter().map(select_first_candidate).collect()
}

fn select_first_candidate(contest: &Contest) -> DecodedVoteContest {
    let selected_id = contest
        .candidates
        .iter()
        .find(|candidate| {
            !candidate.is_explicit_invalid()
                && !candidate.is_explicit_blank()
                && !candidate.is_write_in()
                && !candidate.is_category_list()
        })
        .map(|candidate| candidate.id.clone());

    DecodedVoteContest {
        contest_id: contest.id.clone(),
        is_explicit_invalid: false,
        is_decline_to_vote: false,
        is_blank_ballot: false,
        invalid_errors: vec![],
        invalid_alerts: vec![],
        choices: contest
            .candidates
            .iter()
            .map(|candidate| DecodedVoteChoice {
                id: candidate.id.clone(),
                selected: if Some(&candidate.id) == selected_id.as_ref() {
                    0
                } else {
                    -1
                },
                write_in_text: None,
            })
            .collect(),
    }
}

/// Encrypts, hashes, and (if the ballot style asks for it) signs `contests`
/// against `style`, dispatching on `contest_encryption_policy` the same way
/// `prepare_encrypted_ballot` does in
/// `beyond/packages/ivr-core/src/execution/phases/ballot_loop.rs:434-461`.
pub fn prepare_ballot(
    style: &BallotStyle,
    contests: Vec<DecodedVoteContest>,
) -> Result<PreparedVote> {
    let ctx = RistrettoCtx;
    let encryption_policy = style
        .election_event_presentation
        .as_ref()
        .and_then(|presentation| presentation.contest_encryption_policy.clone())
        .unwrap_or_default();
    let sign = style
        .election_event_presentation
        .as_ref()
        .and_then(|presentation| presentation.voter_signing_policy.clone())
        .unwrap_or_default()
        == VoterSigningPolicy::WITH_SIGNATURE;

    match encryption_policy {
        ContestEncryptionPolicy::SINGLE_CONTEST => {
            prepare_singular_ballot(&ctx, contests, style, sign)
        }
        ContestEncryptionPolicy::MULTIPLE_CONTESTS => {
            prepare_multi_ballot(&ctx, contests, style, sign)
        }
    }
}

fn prepare_singular_ballot(
    ctx: &RistrettoCtx,
    contests: Vec<DecodedVoteContest>,
    style: &BallotStyle,
    sign: bool,
) -> Result<PreparedVote> {
    let auditable = encrypt_decoded_contest::<RistrettoCtx>(ctx, &contests, style)?;
    map_to_decoded_contest::<RistrettoCtx>(&auditable)
        .map_err(|msg| anyhow!("failed to sanity-check the prepared ballot: {msg}"))?;

    let mut signed_hashable = SignedHashableBallot::try_from(&auditable)?;
    let hashable = HashableBallot::try_from(&signed_hashable)?;
    let ballot_id = hash_ballot(&hashable)?;

    if sign {
        let signed = sign_hashable_ballot_with_ephemeral_voter_signing_key(
            &ballot_id,
            &style.election_id,
            &hashable,
        )
        .map_err(|msg| anyhow!("failed to sign ballot: {msg}"))?;
        signed_hashable.voter_signing_pk = Some(signed.public_key);
        signed_hashable.voter_ballot_signature = Some(signed.signature);
    }

    let content = serde_json::to_string(&signed_hashable)?;
    Ok(PreparedVote { ballot_id, content })
}

fn prepare_multi_ballot(
    ctx: &RistrettoCtx,
    contests: Vec<DecodedVoteContest>,
    style: &BallotStyle,
    sign: bool,
) -> Result<PreparedVote> {
    let auditable = encrypt_decoded_multi_contest::<RistrettoCtx>(ctx, &contests, style)?;
    map_to_decoded_multi_contest::<RistrettoCtx>(&auditable)
        .map_err(|msg| anyhow!("failed to sanity-check the prepared ballot: {msg}"))?;

    let mut signed_hashable = SignedHashableMultiBallot::try_from(&auditable)?;
    let hashable = HashableMultiBallot::try_from(&signed_hashable)?;
    let ballot_id = hash_multi_ballot(&hashable)?;

    if sign {
        let signed = sign_hashable_multi_ballot_with_ephemeral_voter_signing_key(
            &ballot_id,
            &style.election_id,
            &hashable,
        )
        .map_err(|msg| anyhow!("failed to sign ballot: {msg}"))?;
        signed_hashable.voter_signing_pk = Some(signed.public_key);
        signed_hashable.voter_ballot_signature = Some(signed.signature);
    }

    let content = serde_json::to_string(&signed_hashable)?;
    Ok(PreparedVote { ballot_id, content })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A real, single-contest, plurality-at-large `BallotStyle`
    /// (`contest_encryption_policy: single-contest`, `voter_signing_policy:
    /// no-signature`), lifted from
    /// `beyond/packages/ivr-core/src/adapters/mock/fixtures/ballot_styles.json`
    /// (its first entry's `ballot_eml`, decoded).
    fn fixture_style() -> BallotStyle {
        serde_json::from_str(include_str!("testdata/ballot_style.json")).unwrap()
    }

    #[test]
    fn synthetic_contests_select_exactly_one_candidate_per_contest() {
        let style = fixture_style();
        let contests = build_synthetic_contests(&style);

        assert_eq!(contests.len(), style.contests.len());
        for (contest, decoded) in style.contests.iter().zip(&contests) {
            assert_eq!(decoded.contest_id, contest.id);
            assert_eq!(decoded.choices.len(), contest.candidates.len());
            assert_eq!(
                decoded
                    .choices
                    .iter()
                    .filter(|choice| choice.selected >= 0)
                    .count(),
                1,
                "exactly one candidate should be selected"
            );
            assert!(decoded
                .choices
                .iter()
                .all(|choice| choice.selected == -1 || choice.selected == 0));
        }
    }

    #[test]
    fn a_synthetic_ballot_encrypts_hashes_and_round_trips() {
        let style = fixture_style();
        let contests = build_synthetic_contests(&style);

        let prepared = prepare_ballot(&style, contests).expect("ballot preparation should succeed");

        assert!(!prepared.ballot_id.is_empty());
        assert!(
            prepared.ballot_id.chars().all(|c| c.is_ascii_hexdigit()),
            "ballot_id should be hex, got {}",
            prepared.ballot_id
        );

        // `content` must be exactly what `insert_cast_vote` sends on the
        // wire: valid JSON, round-tripping through `SignedHashableBallot`.
        let round_tripped: SignedHashableBallot =
            serde_json::from_str(&prepared.content).expect("content should be valid JSON");
        let round_tripped_content = serde_json::to_string(&round_tripped).unwrap();
        let reparsed: SignedHashableBallot = serde_json::from_str(&round_tripped_content).unwrap();
        assert_eq!(
            serde_json::to_value(&reparsed).unwrap(),
            serde_json::to_value(&round_tripped).unwrap()
        );
    }

    #[test]
    fn re_encrypting_the_same_choices_yields_different_ballot_ids() {
        // ElGamal encryption re-randomizes on every call, so preparing the
        // "same" vote twice must NOT produce identical ciphertexts or
        // hashes — a repeat here would mean randomness isn't actually being
        // used, which is a correctness bug (and a privacy one).
        let style = fixture_style();

        let first = prepare_ballot(&style, build_synthetic_contests(&style)).unwrap();
        let second = prepare_ballot(&style, build_synthetic_contests(&style)).unwrap();

        assert_ne!(first.ballot_id, second.ballot_id);
        assert_ne!(first.content, second.content);
    }
}
