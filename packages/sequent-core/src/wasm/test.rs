// SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::ballot::*;
use crate::base64::{Base64Deserialize, Base64Serialize};
use crate::encrypt::*;
use crate::plaintext::DecodedVoteQuestion;
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
            .map_err(|err| format!("Error parsing cyphertext: {}", err))
            .into_json()?;
    let base64_string: String =
        Base64Deserialize::deserialize(auditable_ballot_string)
            .map_err(|err| format!("{:?}", err))
            .into_json()?;
    serde_wasm_bindgen::to_value(&base64_string)
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
            .map_err(|err| format!("Error parsing cyphertext: {}", err))
            .into_json()?;
    let auditable_ballot: AuditableBallot<RistrettoCtx> =
        Base64Deserialize::deserialize(auditable_ballot_string)
            .map_err(|err| format!("{:?}", err))
            .into_json()?;
    let hashable_ballot = HashableBallot::from(&auditable_ballot);

    // return hash
    let hash_string: String = hash_to(&hashable_ballot)
        .map_err(|err| format!("{:?}", err))
        .into_json()?;
    serde_wasm_bindgen::to_value(&hash_string)
        .map_err(|err| format!("{:?}", err))
        .into_json()
}

#[allow(clippy::all)]
#[wasm_bindgen]
pub fn encrypt_decoded_question_js(
    decoded_questions_json: JsValue,
    election_json: JsValue,
) -> Result<JsValue, JsValue> {
    // parse inputs
    let decoded_questions: Vec<DecodedVoteQuestion> =
        serde_wasm_bindgen::from_value(decoded_questions_json)
            .map_err(|err| format!("Error parsing cyphertext: {}", err))
            .into_json()?;
    let election: ElectionDTO = serde_wasm_bindgen::from_value(election_json)
        .map_err(|err| format!("Error parsing election: {}", err))
        .into_json()?;
    // create context
    let ctx = RistrettoCtx;

    // encrypt ballot
    let auditable_ballot = encrypt_decoded_question::<RistrettoCtx>(
        &ctx,
        &decoded_questions,
        &election,
    )
    .map_err(|err| format!("Error encrypting decoded questions {:?}", err))
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
