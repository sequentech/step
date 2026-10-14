// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
// A compiled consumer of the macro, exercised by runtime_contracts.rs.
#[derive(serde::Deserialize)]
struct Input {
    amount: u32,
}
#[derive(serde::Serialize)]
struct Output {
    doubled: u32,
}
#[orare::lambda_runtime]
fn calculate(input: Input) -> Output {
    Output {
        doubled: input.amount * 2,
    }
}
