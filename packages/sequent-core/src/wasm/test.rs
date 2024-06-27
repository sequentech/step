// SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::ballot::*;
use crate::ballot_codec::bigint::BigUIntCodec;
use crate::ballot_codec::raw_ballot::RawBallotCodec;
use crate::encrypt;
use crate::encrypt::*;
use crate::fixtures::ballot_codec::*;
use crate::interpret_plaintext::{
    check_is_blank, get_layout_properties, get_points,
};
use crate::plaintext::*;
//use crate::serialization::base64::Base64Deserialize;
use crate::util::normalize_vote::normalize_vote_contest;
use strand::backend::ristretto::RistrettoCtx;
use wasm_bindgen::prelude::*;
extern crate console_error_panic_hook;
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen;
use serde_wasm_bindgen::Serializer;
use std::collections::HashMap;
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
    let auditable_ballot: AuditableBallot =
        serde_wasm_bindgen::from_value(auditable_ballot_json).map_err(
            |err| format!("Error reading javascript auditable ballot: {}", err),
        )?;
    // test deserializing auditable ballot contests
    let _auditable_ballot_contests = auditable_ballot
        .deserialize_contests::<RistrettoCtx>()
        .map_err(|err| {
            format!("Error deserializing auditable ballot contests: {:?}", err)
        })
        .into_json()?;
    let deserialized_ballot: HashableBallot =
        HashableBallot::try_from(&auditable_ballot).map_err(|err| {
            format!(
                "Error converting auditable ballot to hashable ballot: {:?}",
                err
            )
        })?;

    // test deserializing hashable ballot contests
    let _hashable_ballot_contests = deserialized_ballot
        .deserialize_contests::<RistrettoCtx>()
        .map_err(|err| {
            format!("Error deserializing hashable ballot contests: {:?}", err)
        })
        .into_json()?;
    let serializer = Serializer::json_compatible();
    deserialized_ballot
        .serialize(&serializer)
        .map_err(|err| format!("{:?}", err))
        .into_json()
}

#[allow(clippy::all)]
#[wasm_bindgen]
pub fn hash_auditable_ballot_js(
    auditable_ballot_json: JsValue,
) -> Result<JsValue, JsValue> {
    // parse input
    let auditable_ballot: AuditableBallot =
        serde_wasm_bindgen::from_value(auditable_ballot_json)
            .map_err(|err| {
                format!("Error deserializing auditable ballot: {err}",)
            })
            .into_json()?;
    let hashable_ballot =
        HashableBallot::try_from(&auditable_ballot).map_err(|err| {
            format!(
                "Error converting auditable ballot into hashable ballot: {err}",
            )
        })?;

    // return hash
    let hash_string: String = hash_ballot(&hashable_ballot)
        .map_err(|err| format!("Error hashing ballot: {err}",))
        .into_json()?;
    serde_wasm_bindgen::to_value(&hash_string)
        .map_err(|err| format!("Error writing javascript string: {err}",))
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

    // convert to json output
    let serializer = Serializer::json_compatible();
    auditable_ballot
        .serialize(&serializer)
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
    let auditable_ballot: AuditableBallot =
        serde_wasm_bindgen::from_value(auditable_ballot_json)
            .map_err(|err| {
                format!(
                    "Error parsing auditable ballot javascript string: {}",
                    err
                )
            })
            .into_json()?;
    let plaintext = map_to_decoded_contest::<RistrettoCtx>(&auditable_ballot)
        .into_json()?;
    // https://crates.io/crates/serde-wasm-bindgen
    let serializer = Serializer::json_compatible();
    plaintext
        .serialize(&serializer)
        .map_err(|err| {
            format!("Error converting decoded ballot to json {:?}", err)
        })
        .into_json()
}


#[wasm_bindgen]
pub fn sort_array_by_presentation(
  array: JsValue,
  election_presentation: Option<ElectionPresentation>,
) -> Result<Vec<Contest>> {
    let contest_arr: Vec<Contest> =
        serde_wasm_bindgen::from_value(contests)
            .map_err(|err| {
                format!(
                    "Error parsing auditable ballot javascript string: {}",
                    err
                )
            })?;
            // .into_json()?;
    let election_presentation_val: Option<ElectionPresentation> =
        serde_wasm_bindgen::from_value(election_presentation)
            .map_err(|err| {
                format!(
                    "Error parsing auditable ballot javascript string: {}",
                    err
                )
            })?;
            // .into_json()?;

			//sort contests depending on election_presentation specification

			match election_presentation_val.as_ref().and_then(|ep| ep.contests_order.clone()) {
    Some("alphabetical") => contest_arr.sort_by(|a, b| a.name.cmp(&b.name)),
    Some("custom") => contest_arr.sort_by(|a, b| a.presentation.sort_order.cmp(&b.presentation.sort_order)),
    Some("random") => contest_arr.shuffle(&mut thread_rng()),
    _ => {}
  }
}

#[wasm_bindgen]
pub fn get_layout_properties_from_contest_js(
    val: JsValue,
) -> Result<JsValue, JsValue> {
    let contest: Contest = serde_wasm_bindgen::from_value(val)
        .map_err(|err| format!("Error parsing contest: {}", err))
        .into_json()?;
    let properties = get_layout_properties(&contest);

    let serializer = Serializer::json_compatible();
    properties
        .serialize(&serializer)
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

    let serializer = Serializer::json_compatible();
    points
        .serialize(&serializer)
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

    let serializer = Serializer::json_compatible();
    modified_decoded_contest
        .serialize(&serializer)
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

#[wasm_bindgen]
pub fn generate_sample_auditable_ballot_js() -> Result<JsValue, JsValue> {
    let ctx = RistrettoCtx;
    let ballot_style = get_writein_ballot_style();
    let decoded_contest = get_writein_plaintext();
    let auditable_ballot = encrypt::encrypt_decoded_contest::<RistrettoCtx>(
        &ctx,
        &vec![decoded_contest.clone()],
        &ballot_style,
    )
    .unwrap();

    let serializer = Serializer::json_compatible();
    auditable_ballot
        .serialize(&serializer)
        .map_err(|err| {
            format!("Error converting auditable ballot to json {:?}", err)
        })
        .into_json()
}

#[wasm_bindgen]
pub fn check_is_blank_js(
    decoded_contest_json: JsValue,
) -> Result<JsValue, JsValue> {
    let decoded_contest: DecodedVoteContest =
        serde_wasm_bindgen::from_value(decoded_contest_json)
            .map_err(|err| format!("Error parsing decoded contest: {}", err))
            .into_json()?;
    let is_blank = check_is_blank(decoded_contest);

    serde_wasm_bindgen::to_value(&is_blank)
        .map_err(|err| {
            format!("Error converting boolean is_blank to json {:?}", err)
        })
        .into_json()
}

#[wasm_bindgen]
pub fn check_voting_not_allowed_next(
    contests: JsValue,
    decoded_contests: JsValue,
) -> Result<JsValue, JsValue> {
    let all_contests: Vec<Contest> = serde_wasm_bindgen::from_value(contests)
        .map_err(|err| {
        JsValue::from_str(&format!("Error parsing contests: {}", err))
    })?;
    let all_decoded_contests: HashMap<String, DecodedVoteContest> =
        serde_wasm_bindgen::from_value(decoded_contests).map_err(|err| {
            JsValue::from_str(&format!(
                "Error parsing decoded contests: {}",
                err
            ))
        })?;

    let voting_not_allowed = all_contests.iter().any(|contest| {
        let default_policy = InvalidVotePolicy::default();
        let policy = contest
            .presentation
            .as_ref()
            .and_then(|p| p.invalid_vote_policy.as_ref())
            .unwrap_or(&default_policy);
        if let Some(decoded_contest) = all_decoded_contests.get(&contest.id) {
            let invalid_errors: Vec<InvalidPlaintextError> =
                decoded_contest.invalid_errors.clone();
            invalid_errors.iter().any(|error| {
                matches!(
                    error.error_type,
                    InvalidPlaintextErrorType::Explicit
                        | InvalidPlaintextErrorType::EncodingError
                )
            }) || (invalid_errors.len() > 0
                && *policy == InvalidVotePolicy::NOT_ALLOWED)
        } else {
            false
        }
    });

    Ok(JsValue::from_bool(voting_not_allowed))
}

#[wasm_bindgen]
pub fn check_voting_error_dialog(
    contests: JsValue,
    decoded_contests: JsValue,
) -> Result<JsValue, JsValue> {
    let all_contests: Vec<Contest> = serde_wasm_bindgen::from_value(contests)
        .map_err(|err| {
        JsValue::from_str(&format!("Error parsing contests: {}", err))
    })?;
    let all_decoded_contests: HashMap<String, DecodedVoteContest> =
        serde_wasm_bindgen::from_value(decoded_contests).map_err(|err| {
            JsValue::from_str(&format!(
                "Error parsing decoded contests: {}",
                err
            ))
        })?;

    let show_voting_alert = all_contests.iter().any(|contest| {
        let default_policy = InvalidVotePolicy::default();
        let policy = contest
            .presentation
            .as_ref()
            .and_then(|p| p.invalid_vote_policy.as_ref())
            .unwrap_or(&default_policy);
        if let Some(decoded_contest) = all_decoded_contests.get(&contest.id) {
            let invalid_errors: Vec<InvalidPlaintextError> =
                decoded_contest.invalid_errors.clone();
            let explicit_invalid = decoded_contest.is_explicit_invalid;
            (invalid_errors.len() > 0 && *policy != InvalidVotePolicy::ALLOWED)
                || (*policy
                    == InvalidVotePolicy::WARN_INVALID_IMPLICIT_AND_EXPLICIT
                    && explicit_invalid)
        } else {
            false
        }
    });

    Ok(JsValue::from_bool(show_voting_alert))
}
