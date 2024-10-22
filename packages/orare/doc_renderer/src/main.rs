// SPDX-FileCopyrightText: 2024 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use orare::lambda_runtime;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct Input {
    name: String,
}

#[derive(Serialize, Deserialize)]
struct Output {
    message: String,
}

#[lambda_runtime]
fn hello(input: Input) -> Output {
    Output {
        message: format!("Hello, {}!", input.name),
    }
}
