// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use std::process::ExitCode;

#[rocket::main]
async fn main() -> ExitCode {
    #[cfg(not(feature = "operational-signing-demo-fixture"))]
    if std::env::args_os().count() == 1 {
        match harvest::local_initialize_ca::serve().await {
            Ok(()) => return ExitCode::SUCCESS,
            Err(error) => eprintln!("The Felix3 CA assessment failed ({error})."),
        }
    } else {
        eprintln!("The private CA assessment executable takes no arguments.");
    }
    #[cfg(feature = "operational-signing-demo-fixture")]
    eprintln!("The local CA assessment executable requires a separate build from the sealed fixture.");
    ExitCode::FAILURE
}
