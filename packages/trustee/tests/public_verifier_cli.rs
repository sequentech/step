// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use serde_json::{json, Value};
use std::process::Command;

fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../protocol-board/tests/data/public-transcript-v1.json"
    ))
    .unwrap()
}

fn run(transcript: &Value, request: &Value, tally_id: &str) -> (i32, Value) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("transcript.json");
    std::fs::write(&path, serde_json::to_vec(transcript).unwrap()).unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_trustee"));
    command
        .env_remove("TRUSTEE_CONFIG_PATH")
        .env_remove("B4_URL")
        .args(["verify", "--transcript"])
        .arg(path)
        .args([
            "--configuration-hash",
            request["configuration_hash"].as_str().unwrap(),
            "--tally-id",
            tally_id,
        ]);
    if let Some(hash) = request["ballots_hash"].as_str() {
        command.args(["--ballots-hash", hash]);
    }
    for ciphertext in request["ballot_ciphertexts"].as_array().unwrap() {
        command.args(["--ballot-ciphertext", ciphertext.as_str().unwrap()]);
    }
    let output = command.output().unwrap();
    let report = serde_json::from_slice(&output.stdout).unwrap_or_else(|_| {
        panic!(
            "CLI did not emit a report: {}",
            String::from_utf8_lossy(&output.stderr)
        )
    });
    (output.status.code().unwrap(), report)
}

#[test]
fn offline_cli_needs_no_keys_and_reproduces_public_proof_report_with_uuid_or_decimal_id(
) {
    let fixture = fixture();
    let request = &fixture["request"];
    let id = request["tally_id"].as_str().unwrap();
    let (code, report) = run(&fixture["transcript"], request, id);
    assert_eq!(code, 0);
    assert_eq!(report, fixture["report"]);
    let hex = format!("{:032x}", id.parse::<u128>().unwrap());
    let uuid = format!(
        "{}-{}-{}-{}-{}",
        &hex[..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..]
    );
    assert_eq!(run(&fixture["transcript"], request, &uuid), (0, report));
}

#[test]
fn offline_cli_fails_closed_for_incomplete_forged_or_wrong_election_evidence() {
    let fixture = fixture();
    let id = fixture["request"]["tally_id"].as_str().unwrap();
    let mut evidence = fixture["transcript"].clone();
    evidence["tally_messages"] = json!([]);
    let (code, report) = run(&evidence, &fixture["request"], id);
    assert_ne!(code, 0);
    assert_eq!(report["status"], "incomplete");
    assert_eq!(report["plaintexts"], json!([]));
    let mut request = fixture["request"].clone();
    request["configuration_hash"] = json!("00".repeat(64));
    let (code, report) = run(&fixture["transcript"], &request, id);
    assert_ne!(code, 0);
    assert_eq!(report["status"], "invalid");
    let mut evidence = fixture["transcript"].clone();
    let encoded = evidence["dkg_messages"][0].as_str().unwrap();
    evidence["dkg_messages"][0] = json!(format!("{}AAAA", encoded));
    let (code, report) = run(&evidence, &fixture["request"], id);
    assert_ne!(code, 0);
    assert_eq!(report["status"], "invalid");
}

#[test]
fn cli_does_not_accept_arbitrary_dashes_as_a_uuid_trust_anchor() {
    let fixture = fixture();
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("transcript.json");
    std::fs::write(&path, serde_json::to_vec(&fixture["transcript"]).unwrap())
        .unwrap();
    let id = fixture["request"]["tally_id"]
        .as_str()
        .unwrap()
        .parse::<u128>()
        .unwrap();
    let malformed = format!("-{:032x}", id);
    let output = Command::new(env!("CARGO_BIN_EXE_trustee"))
        .args(["verify", "--transcript"])
        .arg(path)
        .args([
            "--configuration-hash",
            fixture["request"]["configuration_hash"].as_str().unwrap(),
        ])
        .arg(format!("--tally-id={malformed}"))
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "malformed UUID must not become a valid trust anchor"
    );
}
