// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Spawn the shipped CLI on the transmission package windmill's tests keep,
//! made with the ecies-tool jar as the OVCS makes it: a receiver relies on
//! the summary and on a nonzero exit status for every refusal.

use std::path::PathBuf;
use std::process::{Command, Output};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../windmill/tests/fixtures/transmission")
        .join(name)
}

fn run(key: &str, args: &[&str]) -> (Output, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_step-cli"))
        .env("NO_COLOR", "1")
        .args(["step", "verify-transmission-package"])
        .arg(fixture("all_servers.zip"))
        .arg("--key")
        .arg(fixture(key))
        .arg("--ovcs-key")
        .arg(fixture("ovcs.pub.pem"))
        .arg("--ca")
        .arg(fixture("ca.pem"))
        .arg("--certificates")
        .arg(fixture("certificates.pem"))
        .args(args)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (output, stdout, stderr)
}

#[test]
fn a_verified_package_prints_what_it_holds() {
    let (output, stdout, stderr) = run("destination-2.key.pem", &["--required-signatures", "2"]);
    assert!(output.status.success(), "{stderr}");
    for expected in [
        "ok: the transmission package verifies",
        "destination    ccs-2",
        "sealed for     its key, SHA-256 ",
        "station        1234567 Station Alpha",
        "transaction    3141",
        "SHA-256        DEB2F350 60A6082B 2CEE9A20 04B21133 835E3A39 EAA7BF2E 9D3FA500 7F64FEED",
        "signed by      Ana Board Member (SBEI-1): signature valid, certificate CN=Ana Board Member chains to the CA",
        "signatures     2 valid",
        "contest        PRES President (National Election)",
        "C2 Bob Sample & Co",
    ] {
        assert!(stdout.contains(expected), "{expected:?} in {stdout}");
    }
}

#[test]
fn a_refused_package_exits_with_a_failure_and_the_reason() {
    let (output, stdout, stderr) = run("destination-1.key.pem", &["--required-signatures", "3"]);
    assert!(!output.status.success());
    assert!(!stdout.contains("ok:"));
    assert!(stderr.contains("2 board member signature(s) accepted, 3 required"));
}

#[test]
fn another_destinations_key_exits_with_a_failure() {
    let (output, _, stderr) = run("destination-1.key.pem", &["--destination", "ccs-2"]);
    assert!(!output.status.success());
    assert!(stderr.contains("does not open with this key"));
}
