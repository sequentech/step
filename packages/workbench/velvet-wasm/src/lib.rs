// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! `velvet-wasm` — wasm-bindgen surface for `velvet-core`.
//!
//! Exposes the minimum surface the workbench needs to run a tally
//! entirely in the browser:
//!
//! * [`tally_decoded_ballots`]   — tally a batch of already-decoded
//!   `DecodedVoteContest` ballots. The workbench has settled on this
//!   single tally entry point: every caller (contest page, ballot
//!   pipeline, standalone tally tool) feeds decoded selections, never
//!   raw `BigUint` plaintexts. Operators wanting to author ballots in
//!   BigUint form decode them through the ballot pipeline first.
//! * [`encode_ballot`]           — turn a `DecodedVoteContest` (selection
//!   structure) into the decimal-`BigUint` string the encrypt path expects.
//! * [`decode_bigint_to_decoded_vote_contest`] — inverse of `encode_ballot`,
//!   the bridge between BigUint plaintexts and the tally input shape.

use num_bigint::BigUint;
use rand_core::{OsRng, TryRngCore};
use sequent_core::ballot::{Contest, HashableBallotContest, Weight};
use sequent_core::ballot_codec::bigint::decode_bigint_from_bytes;
use sequent_core::ballot_codec::vec::decode_array_to_vec;
use sequent_core::ballot_codec::BigUIntCodec;
use sequent_core::plaintext::DecodedVoteContest;
use sequent_core::serialization::base64::{Base64Deserialize, Base64Serialize};
use sequent_core::types::ceremonies::{ScopeOperation, TallyOperation};
use strand::backend::ristretto::RistrettoCtx;
use strand::context::Ctx;
use strand::elgamal::PrivateKey;
use velvet_core::counting::{CountingAlgorithm, InstantRunoff, PluralityAtLarge, Tally};
use wasm_bindgen::prelude::*;

/// Tally a batch of already-decoded ballots against a contest definition.
///
/// `contest_json` — JSON-serialised `sequent_core::ballot::Contest`.
/// `decoded_ballots` — array of JSON-serialised `DecodedVoteContest`
///                    values, one per ballot. Each entry is the same
///                    shape `decode_bigint_to_decoded_vote_contest`
///                    produces and `encode_ballot` consumes.
///
/// Returns the JSON-encoded `ContestResult`. Errors are surfaced as
/// `JsError` so they reject the awaiting JS promise / throw in sync use.
///
/// The workbench used to expose `tally_plaintext_ballots`, which took
/// `BigUint` strings and ran `decode_ballots_from_lines` internally. We
/// removed it so there is exactly one tally entry shape — decoded
/// selections in, `ContestResult` out — and callers who start from
/// BigUints decode them explicitly (via
/// `decode_bigint_to_decoded_vote_contest`) before tallying. This keeps
/// the workbench's call graph simple: every place that tallies handles
/// the same intermediate shape.
#[wasm_bindgen]
pub fn tally_decoded_ballots(
    contest_json: &str,
    decoded_ballots: Vec<String>,
) -> Result<String, JsError> {
    let contest: Contest = serde_json::from_str(contest_json)
        .map_err(|e| JsError::new(&format!("invalid contest JSON: {e}")))?;

    let decoded: Vec<DecodedVoteContest> = decoded_ballots
        .iter()
        .enumerate()
        .map(|(i, s)| {
            serde_json::from_str::<DecodedVoteContest>(s).map_err(|e| {
                JsError::new(&format!(
                    "invalid DecodedVoteContest JSON at index {i}: {e}"
                ))
            })
        })
        .collect::<Result<_, _>>()?;

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
/// the decrypt path emits.
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

// Note: the workbench's encrypt step lives in
// `packages/workbench/app/src/tally.ts`. It chains sequent-core's
// canonical `encrypt_decoded_contest_js` + `to_hashable_ballot_js`
// (the same path the lifted booth's Cast button traverses), so
// `/pipeline` and Cast share one encrypt implementation. A previous
// `encrypt_decoded_vote_contest` lived here as a hand-rolled
// duplicate; it was removed once sequent-core's wasm-bindgen surface
// became reachable from the workbench (see LIFTING.md §A7 and the
// "canonical surface" rule in §I).

/// Decode a decimal-`BigUint` encoded plaintext back into the structured
/// `DecodedVoteContest` selection it came from. Inverse of
/// [`encode_ballot`].
///
/// `contest_json` — JSON-serialised `Contest`.
/// `bigint_str`   — decimal-encoded BigUint, exactly what `encode_ballot`
///                  or `decrypt_ballot_content` produces.
#[wasm_bindgen]
pub fn decode_bigint_to_decoded_vote_contest(
    contest_json: &str,
    bigint_str: &str,
) -> Result<String, JsError> {
    let contest: Contest = serde_json::from_str(contest_json)
        .map_err(|e| JsError::new(&format!("invalid contest JSON: {e}")))?;
    let bigint = BigUint::parse_bytes(bigint_str.trim().as_bytes(), 10)
        .ok_or_else(|| JsError::new("bigint_str is not a decimal BigUint"))?;
    let decoded = contest
        .decode_plaintext_contest_bigint(&bigint)
        .map_err(|e| JsError::new(&format!("decode failed: {e}")))?;
    serde_json::to_string(&decoded)
        .map_err(|e| JsError::new(&format!("serialise decoded failed: {e}")))
}

