// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::area_tree::*;
use crate::services::tally_sheet_validation::{
    effective_max_marks_per_ballot, validate_area_contest_results,
};
use crate::types::hasura::core::AreaContest;
use crate::types::tally_sheets::AreaContestResults;
use crate::wasm::wasm::IntoResult;
use std::collections::HashSet;
use wasm_bindgen::prelude::*;
extern crate console_error_panic_hook;
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen;
use serde_wasm_bindgen::Serializer;
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

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
struct ContestMarkBoundsInput {
    max_votes: Option<i64>,
    counting_algorithm: Option<String>,
    cumulative_number_of_checkboxes: Option<u64>,
}

#[allow(clippy::all)]
#[wasm_bindgen]
pub fn validate_area_contest_results_js(
    content_json: JsValue,
    contest_bounds_json: JsValue,
) -> Result<JsValue, JsValue> {
    let content: AreaContestResults =
        serde_wasm_bindgen::from_value(content_json).map_err(|err| {
            format!(
            "Error reading javascript area contest results for validation: {}",
            err
        )
        })?;
    let bounds: ContestMarkBoundsInput =
        serde_wasm_bindgen::from_value(contest_bounds_json).map_err(|err| {
            format!(
                "Error reading javascript contest bounds for validation: {}",
                err
            )
        })?;

    let max_marks_per_ballot = effective_max_marks_per_ballot(
        bounds.max_votes,
        bounds.counting_algorithm.as_deref(),
        bounds.cumulative_number_of_checkboxes,
    );

    let errors =
        validate_area_contest_results(&content, Some(max_marks_per_ballot));
    let serializer = Serializer::json_compatible();
    errors
        .serialize(&serializer)
        .map_err(|err| format!("{:?}", err))
        .into_json()
}
