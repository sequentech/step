// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Spawn the shipped CLI: scripts need a failing exit status when conversion
//! fails, and argument validation must happen before touching either file.

use std::fs;
use std::process::Command;

#[test]
fn failed_conversion_is_visible_to_shell_scripts_and_preserves_old_output() {
    let directory = tempfile::tempdir().unwrap();
    let input = directory.path().join("invalid.csv");
    let output = directory.path().join("credentials.csv");
    fs::write(&input, "username,note\nfirst,no password column\n").unwrap();
    fs::write(&output, "previous export").unwrap();

    let result = Command::new(env!("CARGO_BIN_EXE_step-cli"))
        .args(["step", "hash-password", "--input-file"])
        .arg(&input)
        .arg("--output-file")
        .arg(&output)
        .args(["--iterations", "2"])
        .output()
        .unwrap();
    assert!(
        !result.status.success(),
        "a printed error with exit zero misleads automation"
    );
    assert!(String::from_utf8_lossy(&result.stderr).contains("password"));
    assert_eq!(fs::read_to_string(output).unwrap(), "previous export");
}

#[test]
fn zero_iterations_are_rejected_by_the_argument_parser_before_file_access() {
    let directory = tempfile::tempdir().unwrap();
    let output = directory.path().join("must-not-exist.csv");
    let result = Command::new(env!("CARGO_BIN_EXE_step-cli"))
        .args([
            "step",
            "hash-password",
            "--input-file",
            "missing.csv",
            "--output-file",
        ])
        .arg(&output)
        .args(["--iterations", "0"])
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&result.stderr).contains("iterations"));
    assert!(!output.exists());
}

#[test]
fn successful_conversion_exits_zero_and_writes_only_derived_credentials() {
    let directory = tempfile::tempdir().unwrap();
    let input = directory.path().join("voters.csv");
    let output = directory.path().join("credentials.csv");
    fs::write(&input, "username,password\nfirst,synthetic-voter-secret\n").unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_step-cli"))
        .args(["step", "hash-password", "--input-file"])
        .arg(&input)
        .arg("--output-file")
        .arg(&output)
        .args(["--iterations", "2"])
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let bytes = fs::read_to_string(&output).unwrap();
    assert!(!bytes.contains("synthetic-voter-secret"));
    assert!(bytes.starts_with("username,password_salt,hashed_password,num_of_iterations\n"));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(output).unwrap().permissions().mode() & 0o077,
            0,
            "derived credential exports must be private to the operator by default"
        );
    }
}
