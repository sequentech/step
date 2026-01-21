// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use sequent_core::ballot::*;
use sequent_core::encrypt::*;
use sequent_core::interpret_plaintext::{get_layout_properties, get_points};
use sequent_core::plaintext::map_to_decoded_contest;
use sequent_core::plaintext::*;
use serde_wasm_bindgen;
use strand::backend::ristretto::RistrettoCtx;
use wasm_bindgen::prelude::*;
extern crate console_error_panic_hook;
use std::panic;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
    #[wasm_bindgen]
    fn postMessage(s: &str);
}

// FIXME: doesn't compile
// #[allow(clippy::all)]
// #[wasm_bindgen]
// pub fn check_ballot_format(val: JsValue) -> Result<bool, String> {
//     serde_wasm_bindgen::from_value::<AuditableBallot<RistrettoCtx>>(val)
//         .map(|_| true)
//         .map_err(|err| format!("Error parsing auditable ballot: {}", err))
// }
//
// #[allow(clippy::all)]
// #[wasm_bindgen]
// pub fn hash_ballot(val: JsValue) -> Result<String, String> {
//     let ballot: AuditableBallot<RistrettoCtx> =
// serde_wasm_bindgen::from_value(val)         .map_err(|err| format!("Error
// parsing auditable ballot: {}", err))?;     hash_to(&ballot).map_err(|err|
// format!("{:?}", err))     Err(String::from(""))
// }
//
// #[allow(clippy::all)]
// #[wasm_bindgen]
// pub fn map_to_decoded_ballot(val: JsValue) -> Result<JsValue, String> {
//     let ballot: AuditableBallot<RistrettoCtx> =
// serde_wasm_bindgen::from_value(val)         .map_err(|err| format!("Error
// parsing auditable ballot: {}", err))?;     let plaintext =
// map_to_decoded_contest(&ballot)?;     // https://crates.io/crates/serde-wasm-bindgen
//     serde_wasm_bindgen::to_value(&plaintext).map_err(|err| format!("{:?}",
// err))// }

#[wasm_bindgen]
pub fn set_hooks() {
    panic::set_hook(Box::new(console_error_panic_hook::hook));
}

#[wasm_bindgen]
pub fn get_layout_properties_from_contest(
    val: JsValue,
) -> Result<JsValue, String> {
    let contest: Contest = serde_wasm_bindgen::from_value(val)
        .map_err(|err| format!("Error parsing contest: {}", err))?;
    let properties = get_layout_properties(&contest);
    serde_wasm_bindgen::to_value(&properties)
        .map_err(|err| format!("{:?}", err))
}

#[wasm_bindgen]
pub fn get_answer_points(
    contest_val: JsValue,
    answer_val: JsValue,
) -> Result<JsValue, String> {
    let contest: Contest = serde_wasm_bindgen::from_value(contest_val)
        .map_err(|err| format!("Error parsing contest: {}", err))?;
    let answer: DecodedVoteChoice = serde_wasm_bindgen::from_value(answer_val)
        .map_err(|err| format!("Error parsing vote choice: {}", err))?;
    let points = get_points(&contest, &answer);
    serde_wasm_bindgen::to_value(&points).map_err(|err| format!("{:?}", err))
}
