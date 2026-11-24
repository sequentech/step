// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The separately built demo binary never falls back to the ordinary server.

// The binary does not exist in the paired, common Harvest coverage cohort.
#![cfg(feature = "operational-signing-demo-fixture")]

#[test]
fn extra_arguments_refuse_before_loading_any_runtime_mount() {
    let result = std::process::Command::new(env!(
        "CARGO_BIN_EXE_harvest-operational-demo"
    ))
    .arg("not-an-option")
    .env_clear()
    .output()
    .unwrap();
    assert_eq!(result.status.code(), Some(1));
    assert!(result.stdout.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&result.stderr).trim(),
        "The isolated Harvest signing fixture failed; no alternate listener was started."
    );
}
