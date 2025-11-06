// SPDX-FileCopyrightText: 2022-2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::ballot::*;
use crate::ballot::{
    sign_hashable_ballot_with_ephemeral_voter_signing_key,
    verify_ballot_signature,
};
use crate::ballot_codec::bigint::BigUIntCodec;
use crate::ballot_codec::multi_ballot::*;
use crate::ballot_codec::raw_ballot::RawBallotCodec;
use crate::encrypt;
use crate::encrypt::*;
use crate::fixtures::ballot_codec::*;
use crate::interpret_plaintext::{
    check_is_blank, get_layout_properties, get_points,
};
use crate::multi_ballot::*;
use crate::plaintext::*;
use crate::serialization::deserialize_with_path::deserialize_value;
use crate::services::generate_urls::get_auth_url;
use crate::services::generate_urls::AuthAction;
use crate::util::normalize_vote::*;
use strand::backend::ristretto::RistrettoCtx;
use wasm_bindgen::prelude::*;
extern crate console_error_panic_hook;
use crate::util::voting_screen::{
    check_voting_error_dialog_util, check_voting_not_allowed_next_util,
};
use rand::seq::SliceRandom;
use rand::thread_rng;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_wasm_bindgen;
use serde_wasm_bindgen::Serializer;
use std::collections::HashMap;
use std::panic;

// use base64;
// use borsh::{from_slice, to_vec, BorshDeserialize, BorshSerialize};

// // A wrapper for Base64-encoded data
// #[derive(Serialize, Deserialize, Debug)]
// struct JsonWrapper {
//     data: String,
// }

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone)]
pub struct ErrorStatus {
    pub error_type: BallotError,
    pub error_msg: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, JsonSchema, Clone, Eq)]
pub enum BallotError {
    PARSE_ERROR,
    DESERIALIZE_AUDITABLE_ERROR,
    DESERIALIZE_HASHABLE_ERROR,
    CONVERT_ERROR,
    SERIALIZE_ERROR,
    INVALID_BALLOT,
}

impl From<ErrorStatus> for JsValue {
    fn from(error: ErrorStatus) -> JsValue {
        serde_wasm_bindgen::to_value(&error).unwrap()
    }
}

pub trait IntoResult<T> {
    fn into_json(self) -> Result<T, JsValue>;
}

impl<T> IntoResult<T> for Result<T, String> {
    fn into_json(self) -> Result<T, JsValue> {
        self.map_err(|err| {
            serde_wasm_bindgen::to_value(&err).unwrap_or_else(|serde_err| {
                JsValue::from_str(&format!(
                    "Error converting error to JSON: {}",
                    serde_err
                ))
            })
        })
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
    // Parse input
    let auditable_ballot_js: Value =
        serde_wasm_bindgen::from_value(auditable_ballot_json)
            .map_err(|err| format!("Failed to parse auditable ballot: {}", err))
            .into_json()?;
    let auditable_ballot: AuditableBallot =
        deserialize_value(auditable_ballot_js)
            .map_err(|err| format!("Failed to parse auditable ballot: {}", err))
            .into_json()?;

    // Test deserializing auditable ballot contests
    let _auditable_ballot_contests = auditable_ballot
        .deserialize_contests::<RistrettoCtx>()
        .map_err(|err| {
            JsValue::from(ErrorStatus {
                error_type: BallotError::DESERIALIZE_AUDITABLE_ERROR,
                error_msg: format!(
                    "Failed to deserialize auditable ballot contests: {}",
                    err
                ),
            })
        })?;

    // Convert auditable ballot to signed hashable ballot
    let deserialized_ballot: SignedHashableBallot =
        SignedHashableBallot::try_from(&auditable_ballot).map_err(|err| {
            JsValue::from(ErrorStatus {
                error_type: BallotError::CONVERT_ERROR,
                error_msg: format!(
                    "Failed to convert auditable ballot to hashable ballot: {}",
                    err
                ),
            })
        })?;

    // Test deserializing hashable ballot contests
    let _hashable_ballot_contests = deserialized_ballot
        .deserialize_contests::<RistrettoCtx>()
        .map_err(|err| {
            JsValue::from(ErrorStatus {
                error_type: BallotError::DESERIALIZE_HASHABLE_ERROR,
                error_msg: format!(
                    "Failed to deserialize hashable ballot contests: {}",
                    err
                ),
            })
        })?;

    // Serialize the hashable ballot
    let serializer = Serializer::json_compatible();
    deserialized_ballot.serialize(&serializer).map_err(|err| {
        JsValue::from(ErrorStatus {
            error_type: BallotError::SERIALIZE_ERROR,
            error_msg: format!("Failed to serialize hashable ballot: {}", err),
        })
    })
}

#[allow(clippy::all)]
#[wasm_bindgen]
pub fn to_hashable_multi_ballot_js(
    auditable_multi_ballot_json: JsValue,
) -> Result<JsValue, JsValue> {
    // Parse input
    let auditable_multi_ballot_js: Value =
        serde_wasm_bindgen::from_value(auditable_multi_ballot_json)
            .map_err(|err| {
                format!("Failed to parse auditable multi ballot: {}", err)
            })
            .into_json()?;
    let auditable_multi_ballot: AuditableMultiBallot =
        deserialize_value(auditable_multi_ballot_js)
            .map_err(|err| {
                format!("Failed to parse auditable multi ballot: {}", err)
            })
            .into_json()?;

    // Test deserializing auditable ballot contests
    let _auditable_ballot_contests = auditable_multi_ballot
        .deserialize_contests::<RistrettoCtx>()
        .map_err(|err| {
            JsValue::from(ErrorStatus {
                error_type: BallotError::DESERIALIZE_AUDITABLE_ERROR,
                error_msg: format!(
                    "Failed to deserialize auditable multi ballot contests: {}",
                    err
                ),
            })
        })?;

    // Convert auditable ballot to hashable ballot
    let deserialized_ballot: SignedHashableMultiBallot =
        SignedHashableMultiBallot::try_from(&auditable_multi_ballot).map_err(
            |err| {
                JsValue::from(ErrorStatus {
                    error_type: BallotError::CONVERT_ERROR,
                    error_msg: format!(
                    "Failed to convert auditable multi ballot to hashable multi ballot: {}",
                    err
                ),
                })
            },
        )?;

    // Test deserializing hashable ballot contests
    let _hashable_ballot_contests = deserialized_ballot
        .deserialize_contests::<RistrettoCtx>()
        .map_err(|err| {
            JsValue::from(ErrorStatus {
                error_type: BallotError::DESERIALIZE_HASHABLE_ERROR,
                error_msg: format!(
                    "Failed to deserialize hashable multi ballot contests: {}",
                    err
                ),
            })
        })?;

    // Serialize the hashable ballot
    let serializer = Serializer::json_compatible();
    deserialized_ballot.serialize(&serializer).map_err(|err| {
        JsValue::from(ErrorStatus {
            error_type: BallotError::SERIALIZE_ERROR,
            error_msg: format!(
                "Failed to serialize hashable multi ballot: {}",
                err
            ),
        })
    })
}

#[allow(clippy::all)]
#[wasm_bindgen]
pub fn hash_auditable_ballot_js(
    auditable_ballot_json: JsValue,
) -> Result<JsValue, JsValue> {
    // parse input
    let auditable_ballot_js: serde_json::Value =
        serde_wasm_bindgen::from_value(auditable_ballot_json)
            .map_err(|err| {
                format!("Error deserializing auditable multi ballot into value: {err}",)
            })
            .into_json()?;
    let auditable_ballot: AuditableBallot =
        deserialize_value(auditable_ballot_js)
            .map_err(|err| {
                format!("Error deserializing auditable ballot: {err}",)
            })
            .into_json()?;
    let signed_hashable_ballot =
        SignedHashableBallot::try_from(&auditable_ballot).map_err(|err| {
            format!(
                "Error converting auditable ballot into hashable ballot: {err}",
            )
        })?;
    let hashable_ballot = HashableBallot::try_from(&signed_hashable_ballot)
        .map_err(|err| {
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
pub fn hash_auditable_multi_ballot_js(
    auditable_multi_ballot_json: JsValue,
) -> Result<JsValue, JsValue> {
    // parse input
    let auditable_multi_ballot_js: serde_json::Value =
        serde_wasm_bindgen::from_value(auditable_multi_ballot_json)
            .map_err(|err| {
                format!("Error deserializing auditable multi ballot into value: {err}",)
            })
            .into_json()?;
    let auditable_multi_ballot: AuditableMultiBallot =
        deserialize_value(auditable_multi_ballot_js)
            .map_err(|err| {
                format!("Error deserializing auditable multi ballot: {err}",)
            })
            .into_json()?;

    let hashable_multi_ballot =
        HashableMultiBallot::try_from(&auditable_multi_ballot).map_err(|err| {
            format!(
                "Error converting auditable ballot into hashable multi ballot: {err}",
            )
        })?;

    // return hash
    let hash_string: String = hash_multi_ballot(&hashable_multi_ballot)
        .map_err(|err| format!("Error hashing multi ballot: {err}",))
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
    let decoded_contests_js: Value =
        serde_wasm_bindgen::from_value(decoded_contests_json)
            .map_err(|err| format!("Error parsing decoded contests: {}", err))
            .into_json()?;
    let decoded_contests: Vec<DecodedVoteContest> =
        deserialize_value(decoded_contests_js)
            .map_err(|err| format!("Error parsing decoded contests: {}", err))
            .into_json()?;
    let election_js: Value = serde_wasm_bindgen::from_value(election_json)
        .map_err(|err| format!("Error parsing election: {}", err))
        .into_json()?;
    let election: BallotStyle = deserialize_value(election_js)
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

#[allow(clippy::all)]
#[wasm_bindgen]
pub fn encrypt_decoded_multi_contest_js(
    decoded_multi_contests_json: JsValue,
    election_json: JsValue,
) -> Result<JsValue, JsValue> {
    // parse inputs
    let decoded_multi_contests_js: Value =
        serde_wasm_bindgen::from_value(decoded_multi_contests_json)
            .map_err(|err| format!("Error parsing decoded contests: {}", err))
            .into_json()?;
    let decoded_multi_contests: Vec<DecodedVoteContest> =
        deserialize_value(decoded_multi_contests_js)
            .map_err(|err| format!("Error parsing decoded contests: {}", err))
            .into_json()?;
    let election_js: Value = serde_wasm_bindgen::from_value(election_json)
        .map_err(|err| format!("Error parsing election: {}", err))
        .into_json()?;
    let election: BallotStyle = deserialize_value(election_js)
        .map_err(|err| format!("Error parsing election: {}", err))
        .into_json()?;
    // create context
    let ctx = RistrettoCtx;

    // encrypt ballot
    let auditable_multi_ballot = encrypt_decoded_multi_contest::<RistrettoCtx>(
        &ctx,
        &decoded_multi_contests,
        &election,
    )
    .map_err(|err| format!("Error encrypting decoded contests {:?}", err))
    .into_json()?;

    // convert to json output
    let serializer = Serializer::json_compatible();
    auditable_multi_ballot
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
    let auditable_ballot_js: Value =
        serde_wasm_bindgen::from_value(auditable_ballot_json)
            .map_err(|err| {
                format!(
                    "Error parsing auditable ballot javascript string: {}",
                    err
                )
            })
            .into_json()?;
    let auditable_ballot: AuditableBallot =
        deserialize_value(auditable_ballot_js)
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

// before: map_to_decoded_ballot
#[allow(clippy::all)]
#[wasm_bindgen]
pub fn decode_auditable_multi_ballot_js(
    auditable_multi_ballot_json: JsValue,
) -> Result<JsValue, JsValue> {
    let auditable_multi_ballot_js: Value =
        serde_wasm_bindgen::from_value(auditable_multi_ballot_json)
            .map_err(|err| {
                format!(
                    "Error parsing auditable ballot javascript string: {}",
                    err
                )
            })
            .into_json()?;
    let auditable_multi_ballot: AuditableMultiBallot =
        deserialize_value(auditable_multi_ballot_js)
            .map_err(|err| {
                format!(
                    "Error parsing auditable ballot javascript string: {}",
                    err
                )
            })
            .into_json()?;

    let plaintext =
        map_to_decoded_multi_contest::<RistrettoCtx>(&auditable_multi_ballot)
            .into_json()?;
    // https://crates.io/crates/serde-wasm-bindgen
    let serializer = Serializer::json_compatible();
    plaintext
        .serialize(&serializer)
        .map_err(|err| {
            format!("Error converting decoded multi ballot to json {:?}", err)
        })
        .into_json()
}

#[wasm_bindgen]
pub fn sort_candidates_list_js(
    all_candidates: JsValue,
    order: JsValue,
    apply_random: JsValue,
) -> Result<JsValue, JsValue> {
    let all_candidates_js: Value =
        serde_wasm_bindgen::from_value(all_candidates)
            .map_err(|err| format!("Error parsing candidates: {}", err))
            .into_json()?;
    let mut all_candidates: Vec<Candidate> =
        deserialize_value(all_candidates_js)
            .map_err(|err| format!("Error parsing candidates: {}", err))
            .into_json()?;
    let order_field: CandidatesOrder =
        serde_wasm_bindgen::from_value(order.clone())
            .unwrap_or(CandidatesOrder::default());

    let should_apply_random: bool =
        serde_wasm_bindgen::from_value(apply_random.clone()).unwrap_or(false);

    match order_field {
        CandidatesOrder::Alphabetical => {
            all_candidates.sort_by(|a, b| {
                let name_a = a
                    .alias
                    .as_ref()
                    .or(a.name.as_ref())
                    .unwrap_or(&String::new())
                    .to_lowercase();
                let name_b = b
                    .alias
                    .as_ref()
                    .or(b.name.as_ref())
                    .unwrap_or(&String::new())
                    .to_lowercase();
                name_a.cmp(&name_b)
            });
        }
        CandidatesOrder::Custom => {
            all_candidates.sort_by(|a, b| {
                let sort_order_a = a
                    .presentation
                    .as_ref()
                    .and_then(|p| p.sort_order)
                    .unwrap_or(-1);
                let sort_order_b = b
                    .presentation
                    .as_ref()
                    .and_then(|p| p.sort_order)
                    .unwrap_or(-1);
                sort_order_a.cmp(&sort_order_b)
            });
        }

        CandidatesOrder::Random => {
            if should_apply_random {
                let mut rng = thread_rng();
                all_candidates.shuffle(&mut rng);
            }
        }
    }

    let serializer = Serializer::json_compatible();
    Serialize::serialize(&all_candidates, &serializer)
        .map_err(|err| format!("Error converting array to json {:?}", err))
        .into_json()
}

#[wasm_bindgen]
pub fn sort_contests_list_js(
    contests_json: JsValue,
    order: JsValue,
    apply_random: JsValue,
) -> Result<JsValue, JsValue> {
    let contests_js: Value = serde_wasm_bindgen::from_value(contests_json)
        .map_err(|err| format!("Error parsing contests: {}", err))
        .into_json()?;
    let mut all_contests: Vec<Contest> = deserialize_value(contests_js)
        .map_err(|err| format!("Error parsing contests: {}", err))
        .into_json()?;
    let order_field: ContestsOrder =
        serde_wasm_bindgen::from_value(order.clone())
            .unwrap_or(ContestsOrder::default());

    let should_apply_random: bool =
        serde_wasm_bindgen::from_value(apply_random.clone()).unwrap_or(false);

    match order_field {
        ContestsOrder::Alphabetical => {
            all_contests.sort_by(|a, b| {
                let name_a = a
                    .alias
                    .as_ref()
                    .or(a.name.as_ref())
                    .unwrap_or(&String::new())
                    .to_lowercase();
                let name_b = b
                    .alias
                    .as_ref()
                    .or(b.name.as_ref())
                    .unwrap_or(&String::new())
                    .to_lowercase();
                name_a.cmp(&name_b)
            });
        }
        ContestsOrder::Custom => {
            all_contests.sort_by(|a, b| {
                let sort_order_a = a
                    .presentation
                    .as_ref()
                    .and_then(|p| p.sort_order)
                    .unwrap_or(-1);
                let sort_order_b = b
                    .presentation
                    .as_ref()
                    .and_then(|p| p.sort_order)
                    .unwrap_or(-1);
                sort_order_a.cmp(&sort_order_b)
            });
        }

        ContestsOrder::Random => {
            if should_apply_random {
                let mut rng = thread_rng();
                all_contests.shuffle(&mut rng);
            }
        }
    }

    let serializer = Serializer::json_compatible();
    Serialize::serialize(&all_contests, &serializer)
        .map_err(|err| format!("Error converting array to json {:?}", err))
        .into_json()
}

#[wasm_bindgen]
pub fn sort_elections_list_js(
    elections_json: JsValue,
    order: JsValue,
    apply_random: JsValue,
) -> Result<JsValue, JsValue> {
    let elections_js: Value = serde_wasm_bindgen::from_value(elections_json)
        .map_err(|err| format!("Error parsing elections: {}", err))
        .into_json()?;
    let mut all_elections: Vec<Election> = deserialize_value(elections_js)
        .map_err(|err| format!("Error parsing elections: {}", err))
        .into_json()?;
    let order_field: ElectionsOrder =
        serde_wasm_bindgen::from_value(order.clone())
            .unwrap_or(ElectionsOrder::default());

    let should_apply_random: bool =
        serde_wasm_bindgen::from_value(apply_random.clone()).unwrap_or(false);

    match order_field {
        ElectionsOrder::Alphabetical => {
            all_elections.sort_by(|a, b| {
                let name_a = a
                    .alias
                    .as_ref()
                    .or(a.name.as_ref())
                    .unwrap_or(&String::new())
                    .to_lowercase();
                let name_b = b
                    .alias
                    .as_ref()
                    .or(b.name.as_ref())
                    .unwrap_or(&String::new())
                    .to_lowercase();
                name_a.cmp(&name_b)
            });
        }
        ElectionsOrder::Custom => {
            all_elections.sort_by(|a, b| {
                let sort_order_a = a
                    .presentation
                    .as_ref()
                    .and_then(|p| p.sort_order)
                    .unwrap_or(-1);
                let sort_order_b = b
                    .presentation
                    .as_ref()
                    .and_then(|p| p.sort_order)
                    .unwrap_or(-1);
                sort_order_a.cmp(&sort_order_b)
            });
        }

        ElectionsOrder::Random => {
            if should_apply_random {
                let mut rng = thread_rng();
                all_elections.shuffle(&mut rng);
            }
        }
    }

    let serializer = Serializer::json_compatible();
    Serialize::serialize(&all_elections, &serializer)
        .map_err(|err| format!("Error converting array to json {:?}", err))
        .into_json()
}

#[wasm_bindgen]
pub fn get_layout_properties_from_contest_js(
    contest_json: JsValue,
) -> Result<JsValue, JsValue> {
    let contests_js: Value = serde_wasm_bindgen::from_value(contest_json)
        .map_err(|err| format!("Error parsing contest: {}", err))
        .into_json()?;
    let contest: Contest = deserialize_value(contests_js)
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
    contest_json: JsValue,
    candidate_val: JsValue,
) -> Result<JsValue, JsValue> {
    let contests_js: Value = serde_wasm_bindgen::from_value(contest_json)
        .map_err(|err| format!("Error parsing contest: {}", err))
        .into_json()?;
    let contest: Contest = deserialize_value(contests_js)
        .map_err(|err| format!("Error parsing contest: {}", err))
        .into_json()?;
    let candidate: DecodedVoteChoice =
        serde_wasm_bindgen::from_value(candidate_val)
            .map_err(|err| format!("Error parsing vote choice: {}", err))
            .into_json()?;
    let points = get_points(&contest, &candidate);

    let serializer = Serializer::json_compatible();
    Serialize::serialize(&points, &serializer)
        .map_err(|err| format!("{:?}", err))
        .into_json()
}

#[wasm_bindgen]
pub fn test_contest_reencoding_js(
    decoded_contest_json: JsValue,
    ballot_style_json: JsValue,
) -> Result<JsValue, JsValue> {
    // parse inputs
    let decoded_contest_js: Value =
        serde_wasm_bindgen::from_value(decoded_contest_json)
            .map_err(|err| format!("Error parsing decoded contest: {}", err))
            .into_json()?;
    let decoded_contest: DecodedVoteContest =
        deserialize_value(decoded_contest_js)
            .map_err(|err| format!("Error parsing decoded contest: {}", err))
            .into_json()?;
    let ballot_style_js: Value =
        serde_wasm_bindgen::from_value(ballot_style_json)
            .map_err(|err| format!("Error parsing election: {}", err))
            .into_json()?;
    let ballot_style: BallotStyle = deserialize_value(ballot_style_js)
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
        contest.get_counting_algorithm(),
        true,
        &invalid_candidate_ids,
    );
    let output_compare = normalize_vote_contest(
        &modified_decoded_contest,
        contest.get_counting_algorithm(),
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
pub fn test_multi_contest_reencoding_js(
    decoded_multi_contest_json: JsValue,
    ballot_style_json: JsValue,
) -> Result<JsValue, JsValue> {
    // parse inputs
    let decoded_multi_contest_js: Value =
        serde_wasm_bindgen::from_value(decoded_multi_contest_json)
            .map_err(|err| {
                format!("Error parsing decoded contest vec: {}", err)
            })
            .into_json()?;
    let decoded_multi_contests: Vec<DecodedVoteContest> =
        deserialize_value(decoded_multi_contest_js)
            .map_err(|err| {
                format!("Error parsing decoded contest vec: {}", err)
            })
            .into_json()?;
    let ballot_style_js: Value =
        serde_wasm_bindgen::from_value(ballot_style_json)
            .map_err(|err| format!("Error parsing election: {}", err))
            .into_json()?;
    let ballot_style: BallotStyle = deserialize_value(ballot_style_js)
        .map_err(|err| format!("Error parsing election: {}", err))
        .into_json()?;

    let output_decoded_contests =
        test_multi_contest_reencoding(&decoded_multi_contests, &ballot_style)
            .map_err(|err| JsValue::from_str(&err))?;

    let serializer = Serializer::json_compatible();
    output_decoded_contests
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
    let decoded_contest_js: Value =
        serde_wasm_bindgen::from_value(decoded_contest_json)
            .map_err(|err| format!("Error parsing decoded contest: {}", err))
            .into_json()?;
    let decoded_contest: DecodedVoteContest =
        deserialize_value(decoded_contest_js)
            .map_err(|err| format!("Error parsing decoded contest: {}", err))
            .into_json()?;
    let ballot_style_js: Value =
        serde_wasm_bindgen::from_value(ballot_style_json)
            .map_err(|err| format!("Error parsing ballot style: {}", err))
            .into_json()?;
    let ballot_style: BallotStyle = deserialize_value(ballot_style_js)
        .map_err(|err| format!("Error parsing ballot style: {}", err))
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

    let voting_not_allowed =
        check_voting_not_allowed_next_util(all_contests, all_decoded_contests);

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

    let show_voting_alert =
        check_voting_error_dialog_util(all_contests, all_decoded_contests);

    Ok(JsValue::from_bool(show_voting_alert))
}

#[allow(clippy::all)]
#[wasm_bindgen]
pub fn get_auth_url_js(
    base_url_json: JsValue,
    tenant_id_json: JsValue,
    event_id_json: JsValue,
    auth_action_json: JsValue,
) -> Result<JsValue, JsValue> {
    // parse input
    let base_url: String = serde_wasm_bindgen::from_value(base_url_json)
        .map_err(|err| format!("Error deserializing base_url: {err}",))
        .into_json()?;
    let tenant_id: String = serde_wasm_bindgen::from_value(tenant_id_json)
        .map_err(|err| format!("Error deserializing tenant_id: {err}",))
        .into_json()?;
    let event_id: String = serde_wasm_bindgen::from_value(event_id_json)
        .map_err(|err| format!("Error deserializing event_id: {err}",))
        .into_json()?;
    let auth_action_str: String =
        serde_wasm_bindgen::from_value(auth_action_json)
            .map_err(|err| format!("Error deserializing auth_action: {err}",))
            .into_json()?;

    let auth_action = match auth_action_str.as_str() {
        "login" => AuthAction::Login,
        "enroll" => AuthAction::Enroll,
        _ => return Err(JsValue::from_str("Invalid auth action")),
    };

    // return result
    let auth_url: String =
        get_auth_url(&base_url, &tenant_id, &event_id, auth_action);
    serde_wasm_bindgen::to_value(&auth_url)
        .map_err(|err| format!("Error writing javascript string: {err}",))
        .into_json()
}

#[wasm_bindgen]
pub fn sign_hashable_ballot_with_ephemeral_voter_signing_key_js(
    ballot_id: JsValue,
    election_id: JsValue,
    content: JsValue,
) -> Result<JsValue, JsValue> {
    // Deserialize inputs
    let ballot_id: String = serde_wasm_bindgen::from_value(ballot_id)
        .map_err(|err| format!("Error deserializing ballot_id: {err}"))
        .into_json()?;
    let election_id: String = serde_wasm_bindgen::from_value(election_id)
        .map_err(|err| format!("Error deserializing election_id: {err}"))
        .into_json()?;
    let auditable_ballot_js: Value = serde_wasm_bindgen::from_value(content)
        .map_err(|err| {
            format!("Failed to parse auditable multi ballot: {}", err)
        })
        .into_json()?;
    let auditable_ballot: AuditableBallot =
        deserialize_value(auditable_ballot_js)
            .map_err(|err| {
                format!("Error deserializing auditable multi ballot: {err}",)
            })
            .into_json()?;

    let signed_hashable_ballot =
        SignedHashableBallot::try_from(&auditable_ballot).map_err(|err| {
            format!(
                "Error converting auditable ballot into hashable multi ballot: {err}",
            )
        })?;

    let hashable_ballot =
        HashableBallot::try_from(&signed_hashable_ballot).map_err(|err| {
            format!(
                "Error converting auditable ballot into hashable multi ballot: {err}",
            )
        })?;

    // Generates ephemeral voter signing key and signs the ballot
    let signed_content = sign_hashable_ballot_with_ephemeral_voter_signing_key(
        &ballot_id,
        &election_id,
        &hashable_ballot,
    )
    .map_err(|err| format!("Error signing the ballot signature: {err}"))?;
    serde_wasm_bindgen::to_value(&signed_content)
        .map_err(|err| format!("Error writing javascript string: {err}",))
        .into_json()
}

#[wasm_bindgen]
pub fn sign_hashable_multi_ballot_with_ephemeral_voter_signing_key_js(
    ballot_id: JsValue,
    election_id: JsValue,
    auditable_multi_ballot_json: JsValue,
) -> Result<JsValue, JsValue> {
    // Deserialize inputs
    let ballot_id: String = serde_wasm_bindgen::from_value(ballot_id)
        .map_err(|err| format!("Error deserializing ballot_id: {err}"))
        .into_json()?;
    let election_id: String = serde_wasm_bindgen::from_value(election_id)
        .map_err(|err| format!("Error deserializing election_id: {err}"))
        .into_json()?;
    let auditable_multi_ballot_js: Value =
        serde_wasm_bindgen::from_value(auditable_multi_ballot_json)
            .map_err(|err| {
                format!(
                    "Error parsing auditable ballot javascript string: {}",
                    err
                )
            })
            .into_json()?;
    let auditable_multi_ballot: AuditableMultiBallot =
        deserialize_value(auditable_multi_ballot_js)
            .map_err(|err| {
                format!("Error deserializing auditable multi ballot: {err}",)
            })
            .into_json()?;

    let hashable_multi_ballot =
        HashableMultiBallot::try_from(&auditable_multi_ballot).map_err(|err| {
            format!(
                "Error converting auditable ballot into hashable multi ballot: {err}",
            )
        })
        .into_json()?;

    // Generates ephemeral voter signing key and signs the ballot
    let signed_content =
        sign_hashable_multi_ballot_with_ephemeral_voter_signing_key(
            &ballot_id,
            &election_id,
            &hashable_multi_ballot,
        )
        .map_err(|err| format!("Error signing the ballot: {err}"))
        .into_json()?;
    serde_wasm_bindgen::to_value(&signed_content)
        .map_err(|err| format!("Error writing javascript string: {err}",))
        .into_json()
}

// returns true/false if verified/no-signature, error if the signature can't be
// verified
#[wasm_bindgen]
pub fn verify_ballot_signature_js(
    ballot_id: JsValue,
    election_id: JsValue,
    content: JsValue,
) -> Result<JsValue, JsValue> {
    // Deserialize inputs
    let ballot_id: String = serde_wasm_bindgen::from_value(ballot_id)
        .map_err(|err| format!("Error deserializing ballot_id: {err}"))
        .into_json()?;
    let election_id: String = serde_wasm_bindgen::from_value(election_id)
        .map_err(|err| format!("Error deserializing election_id: {err}"))
        .into_json()?;
    let auditable_ballot_js: Value = serde_wasm_bindgen::from_value(content)
        .map_err(|err| {
            format!("Failed to parse auditable multi ballot: {}", err)
        })
        .into_json()?;
    let auditable_ballot: AuditableBallot =
        deserialize_value(auditable_ballot_js)
            .map_err(|err| {
                format!("Error deserializing auditable multi ballot: {err}",)
            })
            .into_json()?;

    let signed_hashable_ballot =
        SignedHashableBallot::try_from(&auditable_ballot).map_err(|err| {
            format!(
                "Error converting auditable ballot into hashable multi ballot: {err}",
            )
        })?;

    // Verifies the ballot signature
    let result = verify_ballot_signature(
        &ballot_id,
        &election_id,
        &signed_hashable_ballot,
    )
    .map_err(|err| format!("Error verifying the ballot: {err}"))?;

    serde_wasm_bindgen::to_value(&result.is_some())
        .map_err(|err| format!("Error writing javascript string: {err}",))
        .into_json()
}

#[wasm_bindgen]
pub fn verify_multi_ballot_signature_js(
    ballot_id: JsValue,
    election_id: JsValue,
    auditable_multi_ballot_json: JsValue,
) -> Result<JsValue, JsValue> {
    // Deserialize inputs
    let ballot_id: String = serde_wasm_bindgen::from_value(ballot_id)
        .map_err(|err| format!("Error deserializing ballot_id: {err}"))
        .into_json()?;
    let election_id: String = serde_wasm_bindgen::from_value(election_id)
        .map_err(|err| format!("Error deserializing election_id: {err}"))
        .into_json()?;
    let auditable_multi_ballot_js: Value =
        serde_wasm_bindgen::from_value(auditable_multi_ballot_json)
            .map_err(|err| {
                format!(
                    "Error parsing auditable ballot javascript string: {}",
                    err
                )
            })
            .into_json()?;
    let auditable_multi_ballot: AuditableMultiBallot =
        deserialize_value(auditable_multi_ballot_js)
            .map_err(|err| {
                format!("Error deserializing auditable multi ballot: {err}",)
            })
            .into_json()?;

    let signed_hashable_multi_ballot =
        SignedHashableMultiBallot::try_from(&auditable_multi_ballot).map_err(|err| {
            format!(
                "Error converting auditable ballot into hashable multi ballot: {err}",
            )
        })
        .into_json()?;

    // Verifies the ballot signature
    let result = verify_multi_ballot_signature(
        &ballot_id,
        &election_id,
        &signed_hashable_multi_ballot,
    )
    .map_err(|err| format!("Error verifying the ballot signature: {err}"))
    .into_json()?;

    serde_wasm_bindgen::to_value(&result.is_some())
        .map_err(|err| format!("Error writing javascript string: {err}",))
        .into_json()
}
