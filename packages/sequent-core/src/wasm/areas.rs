// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::area_tree::*;
use crate::types::hasura::core::AreaContest;
use crate::wasm::wasm::IntoResult;
use std::collections::HashSet;
use strand::backend::ristretto::RistrettoCtx;
use wasm_bindgen::prelude::*;
extern crate console_error_panic_hook;
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen;
use serde_wasm_bindgen::Serializer;
use std::collections::HashMap;
use std::panic;

#[allow(clippy::all)]
#[wasm_bindgen]
pub fn create_tree_js(
    areas_json: JsValue,
    area_contests_json: JsValue,
) -> Result<JsValue, JsValue> {
    // parse input
    let areas: Vec<TreeNodeArea> = serde_wasm_bindgen::from_value(areas_json)
        .map_err(|err| {
        format!("Error reading javascript areas: {}", err)
    })?;
    let area_contests: Vec<AreaContest> =
        serde_wasm_bindgen::from_value(area_contests_json).map_err(|err| {
            format!("Error reading javascript area_contests: {}", err)
        })?;

    let base_tree =
        TreeNode::<()>::from_areas(areas).map_err(|err| format!("{}", err))?;

    let contests_data_tree = base_tree.get_contests_data_tree(&area_contests);
    let serializer = Serializer::json_compatible();
    contests_data_tree
        .serialize(&serializer)
        .map_err(|err| format!("{:?}", err))
        .into_json()
}

#[allow(clippy::all)]
#[wasm_bindgen]
pub fn get_contest_matches_js(
    contests_tree_js: JsValue,
    contest_id_js: JsValue,
) -> Result<JsValue, JsValue> {
    // parse input
    let contests_tree: TreeNode<ContestsData> =
        serde_wasm_bindgen::from_value(contests_tree_js).map_err(|err| {
            format!("Error reading javascript contests_tree: {}", err)
        })?;
    let contest_id: String = serde_wasm_bindgen::from_value(contest_id_js)
        .map_err(|err| {
            format!("Error reading javascript contest_id_js: {}", err)
        })?;
    let contests_hashset: HashSet<String> =
        vec![contest_id].into_iter().collect();

    let area_contests = contests_tree.get_contest_matches(&contests_hashset);
    let serializer = Serializer::json_compatible();
    area_contests
        .serialize(&serializer)
        .map_err(|err| format!("{:?}", err))
        .into_json()
}
