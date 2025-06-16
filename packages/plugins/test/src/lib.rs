// SPDX-FileCopyrightText: 2024 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::hooks::{SumRequest, SumResponse};
use extism_pdk::*;
mod hooks;
const MANIFEST: &str = include_str!("../extism.json");

#[plugin_fn]
pub fn get_manifest(_: ()) -> FnResult<String> {
    Ok(MANIFEST.to_string())
}

#[plugin_fn]
pub fn sum_hook(input: Json<SumRequest>) -> FnResult<Json<SumResponse>> {
    hooks::sum(input.0).map(Json)
}
