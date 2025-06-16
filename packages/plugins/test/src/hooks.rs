// SPDX-FileCopyrightText: 2024 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use extism_pdk::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct SumRequest {
    pub a: i64,
    pub b: i64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SumResponse {
    pub result: i64,
}

pub fn sum(req: SumRequest) -> FnResult<SumResponse> {
    Ok(SumResponse {
        result: req.a + req.b,
    })
}
