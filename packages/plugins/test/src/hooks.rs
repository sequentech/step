// SPDX-FileCopyrightText: 2024 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use extism_pdk::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct AddRequest {
    pub a: i64,
    pub b: i64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AddResponse {
    pub result: i64,
}

pub fn add(req: AddRequest) -> FnResult<AddResponse> {
    Ok(AddResponse {
        result: req.a + req.b,
    })
}
