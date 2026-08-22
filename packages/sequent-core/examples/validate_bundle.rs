// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Validate an election event bundle from a file.
//!
//! `cargo run -p sequent-core --features default_features --example validate_bundle -- <path>`
//!
//! A thin harness for checking a real export against the shared rules; the same
//! call step-cli and the browser make.

use sequent_core::election_config::{validate, ImportElectionEventSchema};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("usage: validate_bundle <path>")?;
    let text = std::fs::read_to_string(&path)?;
    let bundle: ImportElectionEventSchema = serde_json::from_str(&text)?;

    let report = validate(&bundle);
    print!("{report}");
    println!(
        "{} error(s), {} warning(s)",
        report.errors().count(),
        report.warnings().count()
    );
    if report.has_errors() {
        std::process::exit(1);
    }
    Ok(())
}
