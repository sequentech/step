// SPDX-FileCopyrightText: 2024 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::hooks::{AddRequest, AddResponse};
use extism_pdk::*;
mod hooks;
mod routes;
const MANIFEST: &str = include_str!("../extism.json");

#[plugin_fn]
pub fn get_manifest(_: ()) -> FnResult<String> {
    Ok(MANIFEST.to_string())
}

// #[plugin_fn]
// pub fn get_routes(_: ()) -> FnResult<Json<serde_json::Value>> {
//     routes::get_routes()
// }

#[plugin_fn]
pub fn handle_route(input: String) -> FnResult<String> {
    routes::handle_route(input)
}

#[plugin_fn]
pub fn add_hook(input: Json<AddRequest>) -> FnResult<Json<AddResponse>> {
    hooks::add(input.0).map(Json)
}
