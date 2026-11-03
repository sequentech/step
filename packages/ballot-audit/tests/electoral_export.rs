// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
use ballot_audit::ExitStatus;
use std::process::Command;

const EVENT: &str = "00000000-0000-4000-8000-000000000020";

#[test]
fn refuses_remote_before_connecting_or_creating_output() {
    let output =
        std::env::temp_dir().join(format!("electoral-export-denied-{}", std::process::id()));
    let result = Command::new(env!("CARGO_BIN_EXE_export-electoral-log"))
        .args([
            "--server-url",
            "http://example.invalid:3322",
            "--database",
            "synthetic",
            "--username",
            "synthetic",
            "--election-event-id",
            EVENT,
            "--output",
        ])
        .arg(&output)
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(ExitStatus::InvalidInput.code()));
    assert!(!output.exists());
    assert!(result.stdout.is_empty());
    let error = String::from_utf8_lossy(&result.stderr);
    assert!(error.contains("Electoral-log export failed:"));
    assert!(!error.contains("example.invalid"));
}

#[test]
fn missing_password_fails_without_exposing_variable_or_username() {
    let output =
        std::env::temp_dir().join(format!("electoral-export-password-{}", std::process::id()));
    let result = Command::new(env!("CARGO_BIN_EXE_export-electoral-log"))
        .args([
            "--server-url",
            "http://127.0.0.1:43322",
            "--database",
            "synthetic",
            "--username",
            "private-user",
            "--password-env",
            "BALLOT_AUDIT_MISSING_TEST_SECRET",
            "--election-event-id",
            EVENT,
            "--output",
        ])
        .env_remove("BALLOT_AUDIT_MISSING_TEST_SECRET")
        .arg(&output)
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(ExitStatus::InvalidInput.code()));
    assert!(!output.exists());
    assert!(result.stdout.is_empty());
    let error = String::from_utf8_lossy(&result.stderr);
    assert!(error.contains("password environment variable is missing"));
    assert!(!error.contains("private-user"));
    assert!(!error.contains("BALLOT_AUDIT_MISSING_TEST_SECRET"));
}

#[test]
fn requires_a_canonical_event_scope() {
    let output =
        std::env::temp_dir().join(format!("electoral-export-scope-{}", std::process::id()));
    let base = [
        "--server-url",
        "http://127.0.0.1:43322",
        "--database",
        "synthetic",
        "--username",
        "synthetic",
    ];
    let missing = Command::new(env!("CARGO_BIN_EXE_export-electoral-log"))
        .args(base)
        .arg("--output")
        .arg(&output)
        .output()
        .unwrap();
    assert_eq!(missing.status.code(), Some(ExitStatus::Usage.code()));
    let invalid = Command::new(env!("CARGO_BIN_EXE_export-electoral-log"))
        .args(base)
        .args(["--election-event-id", EVENT, "--election-id", "NOT-A-UUID"])
        .arg("--output")
        .arg(&output)
        .output()
        .unwrap();
    assert_eq!(invalid.status.code(), Some(ExitStatus::InvalidInput.code()));
    assert!(!output.exists());
}
