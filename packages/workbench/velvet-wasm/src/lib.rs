// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! `velvet-wasm` — wasm-bindgen surface for `velvet-core`.
//!
//! Exposes the minimum surface the workbench needs to run a tally
//! entirely in the browser:
//!
//! * [`tally_plaintext_ballots`] — decode + tally a batch of ballots.
//! * [`encode_ballot`]           — turn a `DecodedVoteContest` (selection
//!   structure) into the decimal-`BigUint` string the decoder expects.
//! * [`get_sample_contest_json`] / [`get_sample_ballots_json`] —
//!   developer fixtures that mirror the in-tree sequent-core test
//!   contest, used to bootstrap the UI before a real contest editor
//!   exists.

use rand_core::{OsRng, TryRngCore};
use sequent_core::ballot::{Contest, HashableBallotContest, Weight};
use sequent_core::ballot_codec::bigint::decode_bigint_from_bytes;
use sequent_core::ballot_codec::vec::decode_array_to_vec;
use sequent_core::ballot_codec::BigUIntCodec;
use sequent_core::plaintext::{DecodedVoteChoice, DecodedVoteContest};
use sequent_core::serialization::base64::{Base64Deserialize, Base64Serialize};
use sequent_core::types::ceremonies::{ScopeOperation, TallyOperation};
use strand::backend::ristretto::RistrettoCtx;
use strand::context::Ctx;
use strand::elgamal::PrivateKey;
use velvet_core::counting::{CountingAlgorithm, InstantRunoff, PluralityAtLarge, Tally};
use velvet_core::decode::decode_ballots_from_lines;
use wasm_bindgen::prelude::*;

/// Tally a batch of plaintext ballots against a contest definition.
///
/// `contest_json` — JSON-serialised `sequent_core::ballot::Contest`.
/// `ballots`      — array of decimal-encoded `BigUint` strings, one per ballot.
///
/// Returns the JSON-encoded `ContestResult`. Errors are surfaced as
/// `JsError` so they reject the awaiting JS promise / throw in sync use.
#[wasm_bindgen]
pub fn tally_plaintext_ballots(
    contest_json: &str,
    ballots: Vec<String>,
) -> Result<String, JsError> {
    let contest: Contest = serde_json::from_str(contest_json)
        .map_err(|e| JsError::new(&format!("invalid contest JSON: {e}")))?;

    let decoded = decode_ballots_from_lines(ballots.iter(), &contest)
        .map_err(|e| JsError::new(&format!("decode failed: {e}")))?;

    let weight: Weight = Weight::default();
    let ballots_with_weights: Vec<_> =
        decoded.into_iter().map(|v| (v, weight.clone())).collect();
    let census = ballots_with_weights.len() as u64;

    let tally = Tally::from_ballots(
        &contest,
        ScopeOperation::Contest(TallyOperation::ProcessBallotsAll),
        ballots_with_weights,
        census,
        0,
        vec![],
        vec![],
    )
    .map_err(|e| JsError::new(&format!("tally setup failed: {e}")))?;

    // Use OsRng for IRV tiebreaks. `&mut dyn RngCore` is what the trait
    // wants; `OsRng` (rand_core 0.9) impls `TryRngCore`, but in a
    // browser the underlying OS RNG never fails, so `.unwrap_err()`
    // (which adapts `TryRngCore` → `RngCore` via the `UnwrapErr`
    // newtype) is sound here.
    let mut rng = OsRng.unwrap_err();

    let counting_alg = contest
        .counting_algorithm
        .ok_or_else(|| JsError::new("contest is missing counting_algorithm"))?;

    let result = match counting_alg {
        sequent_core::types::ceremonies::CountingAlgType::PluralityAtLarge => {
            PluralityAtLarge::new(tally).tally(&mut rng)
        }
        sequent_core::types::ceremonies::CountingAlgType::InstantRunoff => {
            InstantRunoff::new(tally).tally(&mut rng)
        }
        other => {
            return Err(JsError::new(&format!(
                "counting algorithm not supported in workbench yet: {other:?}"
            )))
        }
    }
    .map_err(|e| JsError::new(&format!("tally failed: {e}")))?;

    serde_json::to_string(&result)
        .map_err(|e| JsError::new(&format!("serialise failed: {e}")))
}

/// Encode a `DecodedVoteContest` (the structured "which candidates did
/// the voter pick, in what order") into the decimal-`BigUint` string
/// the decoder / `tally_plaintext_ballots` expects.
///
/// `contest_json`              — JSON-serialised `Contest`.
/// `decoded_vote_contest_json` — JSON-serialised `DecodedVoteContest`.
#[wasm_bindgen]
pub fn encode_ballot(
    contest_json: &str,
    decoded_vote_contest_json: &str,
) -> Result<String, JsError> {
    let contest: Contest = serde_json::from_str(contest_json)
        .map_err(|e| JsError::new(&format!("invalid contest JSON: {e}")))?;
    let decoded: DecodedVoteContest = serde_json::from_str(decoded_vote_contest_json)
        .map_err(|e| JsError::new(&format!("invalid decoded ballot JSON: {e}")))?;

    contest
        .encode_plaintext_contest_bigint(&decoded)
        .map(|bi| bi.to_str_radix(10))
        .map_err(|e| JsError::new(&format!("encode failed: {e}")))
}

// ----------------------------------------------------------------------------
// Workbench keypair + decryption
//
// Production never holds a single per-election ElGamal secret key: the
// election public key is the product of trustee shares and decryption
// happens in the threshold mixnet pipeline. The workbench has no
// trustees, so to *exercise* the encrypt → decrypt → decode → tally
// loop end-to-end inside the browser we generate a one-off single-party
// keypair, seed it into the ballot style as `public_key`, and use the
// matching secret here to invert the production encrypt step.
//
// The keypair lives in the workbench's persisted state (alongside the
// captured cast votes); it is *not* the in-tree
// `DEFAULT_PUBLIC_KEY_RISTRETTO_STR` constant — that public key has no
// matching secret anywhere in this repo by design.
// ----------------------------------------------------------------------------

/// Generate a fresh Ristretto ElGamal keypair.
///
/// Returns a JSON string `{"pk_b64":"…","sk_b64":"…"}` (the caller is
/// expected to `JSON.parse` it). We deliberately avoid
/// `serde_wasm_bindgen::to_value` here: by default it serialises
/// `serde_json::Value::Object` as a JS `Map`, not a plain object, so
/// `obj.pk_b64` reads `undefined` on the JS side and the keypair
/// vanishes on the way to the workbench store.
///
/// `pk_b64` is the base64-no-pad encoding of the strand/borsh-serialised
/// public key element — the exact same format as
/// `DEFAULT_PUBLIC_KEY_RISTRETTO_STR`, so it is a drop-in replacement
/// for the fixture's `ballot_style.public_key`. `sk_b64` is the
/// base64-no-pad encoding of the borsh-serialised `PrivateKey`.
#[wasm_bindgen]
pub fn generate_keypair() -> Result<String, JsError> {
    let ctx = RistrettoCtx;
    let sk: PrivateKey<RistrettoCtx> = PrivateKey::gen(&ctx);
    let pk_b64 = Base64Serialize::serialize(sk.pk_element())
        .map_err(|e| JsError::new(&format!("serialise pk failed: {e:?}")))?;
    let sk_b64 = Base64Serialize::serialize(&sk)
        .map_err(|e| JsError::new(&format!("serialise sk failed: {e:?}")))?;
    Ok(serde_json::json!({ "pk_b64": pk_b64, "sk_b64": sk_b64 }).to_string())
}

/// Decrypt a single contest's ciphertext out of the JSON the portal
/// stores in `castVote.content` and return the decoded plaintext as a
/// decimal-`BigUint` string — the exact same byte that `encode_ballot`
/// would produce from the matching `DecodedVoteContest`.
///
/// `content_json` — `castVote.content` JSON. This is a
/// `sequent_core::ballot::HashableBallot` (not `AuditableBallot`):
/// `{version, issue_date, config: <ballot_style_id>, contests:
/// Vec<String>, ballot_style_hash}`. We only need `contests` — each
/// entry is base64-no-pad of a borsh-serialised
/// `HashableBallotContest<RistrettoCtx>` = `{contest_id, ciphertext,
/// proof}`. Note: `AuditableBallotContest` is a *different* layout
/// (`{contest_id, choice: ReplicationChoice{ciphertext, plaintext,
/// randomness}, proof}`) and deserialising the wrong one here yields
/// a confusing "Failed to decode scalar" error.
/// `sk_b64`       — workbench-generated private key, base64-no-pad of
/// the borsh-serialised `PrivateKey<RistrettoCtx>`.
/// `contest_id`   — the contest within the ballot to decrypt.
#[wasm_bindgen]
pub fn decrypt_ballot_content(
    content_json: &str,
    sk_b64: &str,
    contest_id: &str,
) -> Result<String, JsError> {
    let ctx = RistrettoCtx;
    let sk: PrivateKey<RistrettoCtx> = Base64Deserialize::deserialize(sk_b64.to_string())
        .map_err(|e| JsError::new(&format!("invalid sk: {e:?}")))?;
    let value: serde_json::Value = serde_json::from_str(content_json)
        .map_err(|e| JsError::new(&format!("invalid ballot JSON: {e}")))?;
    let contest_blobs = value
        .get("contests")
        .and_then(|v| v.as_array())
        .ok_or_else(|| JsError::new("ballot JSON missing `contests` array"))?;
    let mut target: Option<HashableBallotContest<RistrettoCtx>> = None;
    for blob in contest_blobs {
        let s = blob.as_str().ok_or_else(|| {
            JsError::new("ballot contest entry is not a string")
        })?;
        let contest: HashableBallotContest<RistrettoCtx> =
            Base64Deserialize::deserialize(s.to_string()).map_err(|e| {
                JsError::new(&format!("deserialise ballot contest failed: {e:?}"))
            })?;
        if contest.contest_id == contest_id {
            target = Some(contest);
            break
        }
    }
    let target = target.ok_or_else(|| {
        JsError::new(&format!("contest {contest_id} not present in ballot"))
    })?;
    let element = sk.decrypt(&target.ciphertext);
    // The encrypt path is BigUint -> `encode_bigint_to_bytes` (LE
    // bytes, length-determined by the BigUint) -> `encode_vec_to_array`
    // (pack into [u8;30] with byte 0 holding the length). To recover
    // the original BigUint we must strip the length prefix first;
    // running `decode_bigint_from_bytes` on the raw 30-byte array
    // produces `length + plaintext*256` (e.g. BigUint=4 round-trips
    // to 1025 = 1 + 4*256), which then fails to decode back into a
    // valid `DecodedVoteContest`.
    let plaintext_array: [u8; 30] = ctx.decode(&element);
    let plaintext_bytes = decode_array_to_vec(&plaintext_array);
    let bigint = decode_bigint_from_bytes(&plaintext_bytes)
        .map_err(|e| JsError::new(&format!("bigint decode failed: {e}")))?;
    Ok(bigint.to_str_radix(10))
}

// ----------------------------------------------------------------------------
// Workbench dev fixtures
//
// These helpers reuse the in-tree sequent-core fixture so the JS
// workbench can bootstrap a working tally without yet knowing how to
// construct a Contest from scratch. As the workbench grows a real
// contest editor + ballot composer they will become redundant.
// ----------------------------------------------------------------------------

/// JSON-encoded sample `Contest` (plurality-at-large, max_votes=3, three
/// fixture candidates). Sourced from
/// `sequent_core::fixtures::ballot_codec::get_test_contest`.
#[wasm_bindgen]
pub fn get_sample_contest_json() -> Result<String, JsError> {
    let contest = sequent_core::fixtures::ballot_codec::get_test_contest();
    serde_json::to_string(&contest)
        .map_err(|e| JsError::new(&format!("serialise sample contest failed: {e}")))
}

/// JSON-encoded sample `DecodedVoteContest` representing a single
/// vote for the first candidate of the sample contest. Suitable as
/// input to `encode_ballot` together with `get_sample_contest_json`,
/// and intended as a starting point users can tweak in the UI.
#[wasm_bindgen]
pub fn get_sample_decoded_vote_contest_json() -> Result<String, JsError> {
    let contest = sequent_core::fixtures::ballot_codec::get_test_contest();
    let cand_a = contest.candidates[0].id.clone();
    let decoded = build_decoded_vote_contest(&contest, &[cand_a.as_str()]);
    serde_json::to_string(&decoded).map_err(|e| {
        JsError::new(&format!("serialise sample decoded ballot failed: {e}"))
    })
}

/// JSON-encoded array of three sample plaintext ballots: A, A, B (where
/// A is the first candidate, B the second). Suitable as input to
/// `tally_plaintext_ballots` together with `get_sample_contest_json`.
#[wasm_bindgen]
pub fn get_sample_ballots_json() -> Result<String, JsError> {
    let contest = sequent_core::fixtures::ballot_codec::get_test_contest();
    let cand_a = contest.candidates[0].id.clone();
    let cand_b = contest.candidates[1].id.clone();

    let ballots = vec![
        encode_selection(&contest, &[cand_a.as_str()])?,
        encode_selection(&contest, &[cand_a.as_str()])?,
        encode_selection(&contest, &[cand_b.as_str()])?,
    ];

    serde_json::to_string(&ballots)
        .map_err(|e| JsError::new(&format!("serialise sample ballots failed: {e}")))
}

/// Build a `DecodedVoteContest` for `contest` where `selected` lists
/// the candidate ids picked in preference order (others get -1).
fn build_decoded_vote_contest(
    contest: &Contest,
    selected: &[&str],
) -> DecodedVoteContest {
    let choices: Vec<DecodedVoteChoice> = contest
        .candidates
        .iter()
        .map(|c| {
            let pos = selected
                .iter()
                .position(|id| *id == c.id)
                .map(|i| i as i64)
                .unwrap_or(-1);
            DecodedVoteChoice {
                id: c.id.clone(),
                selected: pos,
                write_in_text: None,
            }
        })
        .collect();

    DecodedVoteContest {
        contest_id: contest.id.clone(),
        choices,
        is_explicit_invalid: false,
        invalid_errors: vec![],
        invalid_alerts: vec![],
    }
}

/// Encode a single ballot given the candidate ids selected in preference
/// order. Internal helper for `get_sample_ballots_json`.
fn encode_selection(contest: &Contest, selected: &[&str]) -> Result<String, JsError> {
    let decoded = build_decoded_vote_contest(contest, selected);
    contest
        .encode_plaintext_contest_bigint(&decoded)
        .map(|bi| bi.to_str_radix(10))
        .map_err(|e| JsError::new(&format!("encode failed: {e}")))
}
