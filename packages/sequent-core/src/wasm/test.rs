// SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::ballot::*;
use crate::base64::{Base64Serialize, Base64Deserialize};
use crate::encrypt::*;
use crate::plaintext::DecodedVoteQuestion;
use strand::backend::ristretto::RistrettoCtx;
use wasm_bindgen::prelude::*;
extern crate console_error_panic_hook;
use serde_wasm_bindgen;
use std::panic;

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
pub fn hash_cyphertext_js(cyphertext_json: JsValue) -> Result<String, String> {
    // parse input
    let cyphertext_string: String =
        serde_wasm_bindgen::from_value(cyphertext_json)
            .map_err(|err| format!("Error parsing cyphertext: {}", err))?;
    let cyphertext: HashableBallot<RistrettoCtx> =
        Base64Deserialize::deserialize(cyphertext_string)
            .map_err(|err| format!("{:?}", err))?;

    // return hash
    hash_to(&cyphertext).map_err(|err| format!("{:?}", err))
}

#[allow(clippy::all)]
#[wasm_bindgen]
pub fn encrypt_decoded_question_js(
    decoded_questions_json: JsValue,
    election_json: JsValue,
) -> Result<JsValue, String> {
    // parse inputs
    let decoded_questions: Vec<DecodedVoteQuestion> =
        serde_wasm_bindgen::from_value(decoded_questions_json)
            .map_err(|err| format!("Error parsing cyphertext: {}", err))?;
    let election: ElectionDTO =
        serde_wasm_bindgen::from_value(election_json)
            .map_err(|err| format!("Error parsing election: {}", err))?;

    // create context
    let ctx = RistrettoCtx;

    // encrypt ballot
    let auditable_ballot = encrypt_decoded_question::<RistrettoCtx>(
        &ctx,
        &decoded_questions,
        &election,
    )
    .map_err(|err| format!("{:?}", err))?;

    let auditable_ballot_serialized: String =
        Base64Serialize::serialize(&auditable_ballot)
            .map_err(|err| format!("{:?}", err))?;

    // convert to json output
    serde_wasm_bindgen::to_value(&auditable_ballot_serialized)
        .map_err(|err| format!("{:?}", err))
}
