// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

#[rocket::main]
async fn main() -> std::process::ExitCode {
    // Deliberately do not load .env, accept flags, or print underlying errors.
    if std::env::args_os().count() != 1
        || harvest::operational_demo_composition::serve_disposable()
            .await
            .is_err()
    {
        eprintln!(
            "The isolated Harvest signing fixture failed; no alternate listener was started."
        );
        return std::process::ExitCode::FAILURE;
    }
    std::process::ExitCode::SUCCESS
}
