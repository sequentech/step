// SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::ballot::*;
use crate::ballot_codec::bigint::BigUIntCodec;
use crate::ballot_codec::raw_ballot::RawBallotCodec;
use crate::encrypt::*;
use crate::interpret_plaintext::{get_layout_properties, get_points};
use crate::plaintext::*;
use crate::serialization::base64::{Base64Deserialize, Base64Serialize};
use crate::util::normalize_vote::normalize_vote_contest;
use strand::backend::ristretto::RistrettoCtx;
use wasm_bindgen::prelude::*;
extern crate console_error_panic_hook;
use serde_wasm_bindgen;
use std::panic;

trait IntoResult<T> {
    fn into_json(self) -> Result<T, JsValue>;
}

impl<T> IntoResult<T> for Result<T, String> {
    fn into_json(self) -> Result<T, JsValue> {
        self.map_err(|err| serde_wasm_bindgen::to_value(&err).unwrap())
    }
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
    #[wasm_bindgen]
    fn postMessage(s: &str);
}

#[wasm_bindgen]
pub fn set_hooks() {
    panic::set_hook(Box::new(console_error_panic_hook::hook));
}

#[allow(clippy::all)]
#[wasm_bindgen]
pub fn to_hashable_ballot_js(
    auditable_ballot_json: JsValue,
) -> Result<JsValue, JsValue> {
    // parse input
    let auditable_ballot_string: String =
        serde_wasm_bindgen::from_value(auditable_ballot_json)
            .map_err(|err| format!("Error reading javascript string: {}", err))
            .into_json()?;
    let auditable_ballot: AuditableBallot<RistrettoCtx> =
        Base64Deserialize::deserialize(auditable_ballot_string)
            .map_err(|err| {
                format!("Error deserializing auditable ballot: {:?}", err)
            })
            .into_json()?;
    let hashable_ballot = HashableBallot::from(&auditable_ballot);

    let hashable_ballot_serialized: String =
        Base64Serialize::serialize(&hashable_ballot)
            .map_err(|err| {
                format!("Error serializing auditable ballot {:?}", err)
            })
            .into_json()?;
    let _deserialized_ballot: HashableBallot<RistrettoCtx> = Base64Deserialize::deserialize(hashable_ballot_serialized.clone())
        .map_err(|err| {
            format!("Error deserializing hashable ballot {:?}", err)
        })
        .into_json()?;
    serde_wasm_bindgen::to_value(&hashable_ballot_serialized)
        .map_err(|err| format!("{:?}", err))
        .into_json()
}

#[allow(clippy::all)]
#[wasm_bindgen]
pub fn hash_auditable_ballot_js(
    auditable_ballot_json: JsValue,
) -> Result<JsValue, JsValue> {
    // parse input
    let auditable_ballot_string: String =
        serde_wasm_bindgen::from_value(auditable_ballot_json)
            .map_err(|err| format!("Error reading javascript string: {}", err))
            .into_json()?;
    let auditable_ballot: AuditableBallot<RistrettoCtx> =
        Base64Deserialize::deserialize(auditable_ballot_string)
            .map_err(|err| {
                format!("Error deserializing auditable ballot: {:?}", err)
            })
            .into_json()?;
    let hashable_ballot = HashableBallot::from(&auditable_ballot);

    // return hash
    let hash_string: String = hash_ballot(&hashable_ballot)
        .map_err(|err| format!("Error hashing ballot: {:?}", err))
        .into_json()?;
    serde_wasm_bindgen::to_value(&hash_string)
        .map_err(|err| format!("Error writing javascript string: {:?}", err))
        .into_json()
}

#[allow(clippy::all)]
#[wasm_bindgen]
pub fn encrypt_decoded_contest_js(
    decoded_contests_json: JsValue,
    election_json: JsValue,
) -> Result<JsValue, JsValue> {
    // parse inputs
    let decoded_contests: Vec<DecodedVoteContest> =
        serde_wasm_bindgen::from_value(decoded_contests_json)
            .map_err(|err| format!("Error parsing decoded contests: {}", err))
            .into_json()?;
    let election: BallotStyle = serde_wasm_bindgen::from_value(election_json)
        .map_err(|err| format!("Error parsing election: {}", err))
        .into_json()?;
    // create context
    let ctx = RistrettoCtx;

    // encrypt ballot
    let auditable_ballot = encrypt_decoded_contest::<RistrettoCtx>(
        &ctx,
        &decoded_contests,
        &election,
    )
    .map_err(|err| format!("Error encrypting decoded contests {:?}", err))
    .into_json()?;

    let auditable_ballot_serialized: String =
        Base64Serialize::serialize(&auditable_ballot)
            .map_err(|err| {
                format!("Error serializing auditable ballot {:?}", err)
            })
            .into_json()?;

    // convert to json output
    serde_wasm_bindgen::to_value(&auditable_ballot_serialized)
        .map_err(|err| {
            format!("Error converting auditable ballot to json {:?}", err)
        })
        .into_json()
}

// before: map_to_decoded_ballot
#[allow(clippy::all)]
#[wasm_bindgen]
pub fn decode_auditable_ballot_js(
    auditable_ballot_json: JsValue,
) -> Result<JsValue, JsValue> {
    let auditable_ballot_string: String =
        serde_wasm_bindgen::from_value(auditable_ballot_json)
            .map_err(|err| {
                format!(
                    "Error parsing auditable ballot javascript string: {}",
                    err
                )
            })
            .into_json()?;
    let auditable_ballot: AuditableBallot<RistrettoCtx> =
        Base64Deserialize::deserialize(auditable_ballot_string)
            .map_err(|err| format!("Error parsing auditable ballot: {:?}", err))
            .into_json()?;
    let plaintext = map_to_decoded_contest(&auditable_ballot).into_json()?;
    // https://crates.io/crates/serde-wasm-bindgen
    serde_wasm_bindgen::to_value(&plaintext)
        .map_err(|err| {
            format!("Error converting decoded ballot to json {:?}", err)
        })
        .into_json()
}

#[wasm_bindgen]
pub fn get_ballot_style_from_auditable_ballot_js(
    auditable_ballot_json: JsValue,
) -> Result<JsValue, JsValue> {
    let auditable_ballot_string: String =
        serde_wasm_bindgen::from_value(auditable_ballot_json)
            .map_err(|err| {
                format!(
                    "Error parsing auditable ballot javascript string: {}",
                    err
                )
            })
            .into_json()?;
    let auditable_ballot: AuditableBallot<RistrettoCtx> =
        Base64Deserialize::deserialize(auditable_ballot_string)
            .map_err(|err| format!("Error parsing auditable ballot: {:?}", err))
            .into_json()?;
    serde_wasm_bindgen::to_value(&auditable_ballot.config)
        .map_err(|err| {
            format!("Error converting decoded ballot to json {:?}", err)
        })
        .into_json()
}

#[wasm_bindgen]
pub fn get_layout_properties_from_contest_js(
    val: JsValue,
) -> Result<JsValue, JsValue> {
    let contest: Contest = serde_wasm_bindgen::from_value(val)
        .map_err(|err| format!("Error parsing contest: {}", err))
        .into_json()?;
    let properties = get_layout_properties(&contest);
    serde_wasm_bindgen::to_value(&properties)
        .map_err(|err| format!("{:?}", err))
        .into_json()
}

#[wasm_bindgen]
pub fn get_candidate_points_js(
    contest_val: JsValue,
    candidate_val: JsValue,
) -> Result<JsValue, JsValue> {
    let contest: Contest = serde_wasm_bindgen::from_value(contest_val)
        .map_err(|err| format!("Error parsing contest: {}", err))
        .into_json()?;
    let candidate: DecodedVoteChoice =
        serde_wasm_bindgen::from_value(candidate_val)
            .map_err(|err| format!("Error parsing vote choice: {}", err))
            .into_json()?;
    let points = get_points(&contest, &candidate);
    serde_wasm_bindgen::to_value(&points)
        .map_err(|err| format!("{:?}", err))
        .into_json()
}

#[wasm_bindgen]
pub fn test_contest_reencoding_js(
    decoded_contest_json: JsValue,
    ballot_style_json: JsValue,
) -> Result<JsValue, JsValue> {
    // parse inputs
    let decoded_contest: DecodedVoteContest =
        serde_wasm_bindgen::from_value(decoded_contest_json)
            .map_err(|err| format!("Error parsing decoded contest: {}", err))
            .into_json()?;
    let ballot_style: BallotStyle =
        serde_wasm_bindgen::from_value(ballot_style_json)
            .map_err(|err| format!("Error parsing election: {}", err))
            .into_json()?;

    let contest = ballot_style
        .contests
        .iter()
        .find(|contest| contest.id == decoded_contest.contest_id)
        .ok_or_else(|| {
            format!(
                "Can't find contest with id {} on ballot style",
                decoded_contest.contest_id
            )
        })
        .into_json()?;
    let bigint = contest
        .encode_plaintext_contest_bigint(&decoded_contest)
        .into_json()?;
    let modified_decoded_contest = contest
        .decode_plaintext_contest_bigint(&bigint)
        .into_json()?;

    let invalid_candidate_ids = contest.get_invalid_candidate_ids();

    let input_compare = normalize_vote_contest(
        &decoded_contest,
        contest.get_counting_algorithm().as_str(),
        true,
        &invalid_candidate_ids,
    );
    let output_compare = normalize_vote_contest(
        &modified_decoded_contest,
        contest.get_counting_algorithm().as_str(),
        true,
        &invalid_candidate_ids,
    );
    if input_compare != output_compare {
        return Err(format!(
            "Consistency check failed. Input =! Output, {:?} != {:?}",
            input_compare, output_compare
        ))
        .into_json();
    }

    serde_wasm_bindgen::to_value(&modified_decoded_contest)
        .map_err(|err| {
            format!("Error converting decoded contest to json {:?}", err)
        })
        .into_json()
}

#[wasm_bindgen]
pub fn get_write_in_available_characters_js(
    decoded_contest_json: JsValue,
    ballot_style_json: JsValue,
) -> Result<JsValue, JsValue> {
    // parse inputs
    let decoded_contest: DecodedVoteContest =
        serde_wasm_bindgen::from_value(decoded_contest_json)
            .map_err(|err| format!("Error parsing decoded contest: {}", err))
            .into_json()?;
    let ballot_style: BallotStyle =
        serde_wasm_bindgen::from_value(ballot_style_json)
            .map_err(|err| format!("Error parsing election: {}", err))
            .into_json()?;

    let contest = ballot_style
        .contests
        .iter()
        .find(|contest| contest.id == decoded_contest.contest_id)
        .ok_or_else(|| {
            format!(
                "Can't find contest with id {} on ballot style",
                decoded_contest.contest_id
            )
        })
        .into_json()?;
    let num_available_chars = contest
        .available_write_in_characters(&decoded_contest)
        .into_json()?;

    serde_wasm_bindgen::to_value(&num_available_chars)
        .map_err(|err| {
            format!("Error converting decoded contest to json {:?}", err)
        })
        .into_json()
}
