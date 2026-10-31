// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Exercise the shipped CLI and its authentication/request/exit-status boundary.
use serde_json::{json, Value};
use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    net::TcpListener,
    process::{Command, Output, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};

const OFFICIAL: &str = "00000000-0000-4000-8000-000000000001";
const TESTED: &str = "00000000-0000-4000-8000-000000000002";

/// Every fixture owns its executable/config and listener. A stopped or invalid
/// command cannot leave an accept thread blocked indefinitely.
fn command(
    args: &[&str],
    status: u16,
    response: Value,
    expected_requests: usize,
) -> (Output, Vec<(String, Value)>) {
    let temporary = tempfile::tempdir().unwrap();
    let binary = temporary.path().join("step-cli");
    fs::hard_link(env!("CARGO_BIN_EXE_step-cli"), &binary)
        .or_else(|_| fs::copy(env!("CARGO_BIN_EXE_step-cli"), &binary).map(|_| ()))
        .unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let config = temporary.path().join("config");
    fs::create_dir(&config).unwrap();
    fs::write(
        config.join("configuration.json"),
        json!({
            "endpoint_url":format!("{endpoint}/graphql"), "tenant_id":"synthetic-tenant",
            "keycloak_url":endpoint, "auth_token":"synthetic-token", "refresh_token":"",
            "client_id":"", "client_secret":"", "username":""
        })
        .to_string(),
    )
    .unwrap();
    let input = temporary.path().join("voters.csv");
    fs::write(&input, "username\nalice\n").unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let server_stop = stop.clone();
    let server_endpoint = endpoint.clone();
    let server = thread::spawn(move || {
        let mut requests = Vec::new();
        let deadline = Instant::now() + Duration::from_secs(20);
        while requests.len() < expected_requests
            && !server_stop.load(Ordering::Relaxed)
            && Instant::now() < deadline
        {
            let (mut stream, _) = match listener.accept() {
                Ok(connection) => connection,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(5));
                    continue;
                }
                Err(error) => panic!("fixture accept: {error}"),
            };
            stream.set_nonblocking(false).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            stream
                .set_write_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut request = String::new();
            if reader.read_line(&mut request).unwrap() == 0 {
                continue;
            }
            let mut length = 0;
            let mut authorized = false;
            let mut role = String::new();
            loop {
                let mut header = String::new();
                assert!(reader.read_line(&mut header).unwrap() > 0);
                if header == "\r\n" {
                    break;
                }
                let (name, value) = header.split_once(':').unwrap();
                if name.eq_ignore_ascii_case("content-length") {
                    length = value.trim().parse().unwrap();
                }
                if name.eq_ignore_ascii_case("x-hasura-role") {
                    role = value.trim().to_string();
                }
                if name.eq_ignore_ascii_case("authorization") {
                    authorized = value.trim() == "Bearer synthetic-token";
                }
            }
            assert!(length < 16_384);
            let mut bytes = vec![0; length];
            reader.read_exact(&mut bytes).unwrap();
            let (body, answer, response_status) = if request.starts_with("PUT /blob ") {
                assert_eq!(bytes, b"username\nalice\n");
                (Value::Null, json!({}), 200)
            } else {
                assert!(authorized, "JWT must accompany API requests");
                let body: Value = serde_json::from_slice(&bytes).unwrap();
                if body["operationName"] == "ReadinessCheck" {
                    assert_eq!(
                        role, "readiness-check",
                        "Hasura must select the action permission"
                    );
                }
                if body["operationName"] == "GetUploadUrl" {
                    (
                        body,
                        json!({"data":{"get_upload_url":{"url":format!("{server_endpoint}/blob"),"document_id":"document"}},"errors":[]}),
                        200,
                    )
                } else {
                    let answer = if body["operationName"] == "ReadinessCheck"
                        && response.get("errors").is_none()
                    {
                        json!({"data":{"readiness_check":response}})
                    } else {
                        response.clone()
                    };
                    (body, answer, status)
                }
            };
            requests.push((request.trim().to_owned(), body));
            let bytes = answer.to_string();
            write!(stream, "HTTP/1.1 {response_status} Fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{bytes}", bytes.len()).unwrap();
        }
        requests
    });
    let resolved: Vec<String> = args
        .iter()
        .map(|arg| match *arg {
            "ENDPOINT" => endpoint.clone(),
            "INPUT" => input.to_str().unwrap().to_owned(),
            _ => (*arg).to_owned(),
        })
        .collect();
    let mut cmd = Command::new(binary);
    cmd.args(resolved)
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for name in ["LLVM_PROFILE_FILE", "LD_LIBRARY_PATH"] {
        if let Some(value) = std::env::var_os(name) {
            cmd.env(name, value);
        }
    }
    let mut child = cmd.spawn().unwrap();
    let deadline = Instant::now() + Duration::from_secs(20);
    while child.try_wait().unwrap().is_none() && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(10));
    }
    let timed_out = child.try_wait().unwrap().is_none();
    if timed_out {
        child.kill().unwrap();
    }
    let output = child.wait_with_output().unwrap();
    stop.store(true, Ordering::Relaxed);
    let requests = server.join().unwrap();
    assert!(!timed_out, "CLI fixture exceeded twenty seconds");
    (output, requests)
}

fn readiness(status: u16, body: Value) -> (Output, Vec<(String, Value)>) {
    command(
        &[
            "step",
            "readiness-check",
            "--official",
            OFFICIAL,
            "--tested",
            TESTED,
        ],
        status,
        body,
        1,
    )
}

#[test]
fn readiness_success_prints_complete_json_and_authenticated_event_ids() {
    let expected = json!({"passed":true,"checks":{"configuration":true},"evidence":{"content_sha256":"certified"},"log_entry_id":"signed-entry"});
    let (output, requests) = readiness(200, expected.clone());
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        expected
    );
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].0, "POST /graphql HTTP/1.1");
    assert_eq!(requests[0].1["operationName"], "ReadinessCheck");
    assert_eq!(
        requests[0].1["variables"],
        json!({"official_event_id":OFFICIAL,"tested_event_id":TESTED})
    );
}

#[test]
fn failed_readiness_keeps_json_evidence_and_exits_nonzero() {
    let expected = json!({"passed":false,"checks":{"official_zero":false},"evidence":null,"log_entry_id":null});
    let (output, requests) = readiness(200, expected.clone());
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(requests.len(), 1);
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        expected
    );
}

#[test]
fn readiness_auth_or_transport_errors_and_malformed_responses_fail() {
    for (status, body) in [
        (403, json!({"error":"forbidden"})),
        (200, json!({"checks":{}})),
    ] {
        let (output, requests) = readiness(status, body);
        assert_eq!(output.status.code(), Some(1));
        assert_eq!(requests.len(), 1);
        assert!(!output.stderr.is_empty());
    }
}

#[test]
fn voter_import_forwards_the_independently_certified_hash() {
    let expected_hash = "ab".repeat(32);
    let (output, requests) = command(
        &[
            "step",
            "import-voters",
            "--election-event-id",
            OFFICIAL,
            "--file-path",
            "INPUT",
            "--sha256",
            &expected_hash,
        ],
        200,
        json!({"errors":[{"message":"fixture stop after forwarding"}],"data":null}),
        3,
    );
    assert_eq!(
        requests.len(),
        3,
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(requests[2].1["operationName"], "ImportUsers");
    assert_eq!(requests[2].1["variables"]["sha256"], expected_hash);
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("fixture stop after forwarding"));
}

#[test]
fn invalid_event_ids_or_hashes_are_rejected_before_network_requests() {
    for args in [
        vec![
            "step",
            "readiness-check",
            "--official",
            "not-a-uuid",
            "--tested",
            TESTED,
        ],
        vec![
            "step",
            "import-voters",
            "--election-event-id",
            OFFICIAL,
            "--file-path",
            "INPUT",
            "--sha256",
            "wrong",
        ],
    ] {
        let (output, requests) = command(&args, 200, json!({}), 0);
        assert_eq!(output.status.code(), Some(2));
        assert!(requests.is_empty());
    }
}

#[test]
fn partial_graphql_errors_cannot_become_readiness_success() {
    let (output, requests) = readiness(
        200,
        json!({"data":{"readiness_check":{"passed":true,"log_entry_id":"entry"}},"errors":[{"message":"readiness denied"}]}),
    );
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(requests.len(), 1);
    assert!(String::from_utf8_lossy(&output.stderr).contains("readiness denied"));
    assert!(output.stdout.is_empty());
}

#[test]
fn a_passing_verdict_without_signed_evidence_is_not_success() {
    let (output, requests) = readiness(
        200,
        json!({"passed":true,"checks":{},"evidence":{},"log_entry_id":null}),
    );
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(requests.len(), 1);
    assert!(String::from_utf8_lossy(&output.stderr).contains("no signed log entry"));
}

#[test]
fn legacy_import_omits_the_optional_declared_hash() {
    let (output, requests) = command(
        &[
            "step",
            "import-voters",
            "--election-event-id",
            OFFICIAL,
            "--file-path",
            "INPUT",
        ],
        200,
        json!({"errors":[{"message":"fixture stop after forwarding"}],"data":null}),
        3,
    );
    assert_eq!(
        requests.len(),
        3,
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(requests[2].1["variables"]["sha256"], Value::Null);
    assert_eq!(output.status.code(), Some(1));
}
