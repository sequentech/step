// SPDX-FileCopyrightText: 2024 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use extism_pdk::*;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct AddReq {
    a: i64,
    b: i64,
}

#[derive(Serialize)]
struct AddRes {
    sum: i64,
}

#[plugin_fn]
pub fn add(req: Vec<u8>) -> FnResult<Vec<u8>> {
    let AddReq { a, b } = serde_json::from_slice(&req)?;
    let res = AddRes { sum: a + b };
    Ok(serde_json::to_vec(&res)?)
}
