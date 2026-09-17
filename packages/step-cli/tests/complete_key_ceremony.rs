// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Drive `complete-key-ceremony` against a synthetic GraphQL endpoint.
use serde_json::{json, Value};
use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    net::TcpListener,
    path::PathBuf,
    process::Command,
    sync::{Arc, Mutex},
    thread,
};

const TRUSTEE: &str = "trustee1";
const ELECTION_EVENT_ID: &str = "synthetic-event";
const DOWNLOAD_UNAVAILABLE: &str = r#"{"errors":[{"message":"Private key download is no longer available","extensions":{"code":"PrivateKeyDownloadUnavailable"}}]}"#;

struct Request {
    operation: String,
    variables: Value,
    stored_key: Option<String>,
}

struct Run {
    output: String,
    requests: Vec<Request>,
    stored_key: Option<String>,
}

fn downloaded(private_key: &str) -> String {
    json!({"data": {"get_private_key": {"private_key_base64": private_key}}}).to_string()
}

fn checked(is_valid: bool) -> String {
    json!({"data": {"check_private_key": {"is_valid": is_valid}}}).to_string()
}

/// Runs the command with `stored_key` already on disk, answering each GraphQL
/// request with the next of `responses`.
fn complete_key_ceremony(stored_key: Option<&str>, responses: Vec<String>) -> Run {
    let temporary = tempfile::tempdir().unwrap();
    let binary = temporary.path().join("step-cli");
    let source = env!("CARGO_BIN_EXE_step-cli");
    if fs::hard_link(source, &binary).is_err() {
        fs::copy(source, &binary).unwrap();
    }
    let key_path: PathBuf = temporary.path().join("keys").join(format!(
        "encrypted_private_key_trustee_{TRUSTEE}_{ELECTION_EVENT_ID}.txt"
    ));
    if let Some(stored_key) = stored_key {
        fs::create_dir(key_path.parent().unwrap()).unwrap();
        fs::write(&key_path, stored_key).unwrap();
    }

    let server = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = server.local_addr().unwrap();
    let config = temporary.path().join("config");
    fs::create_dir(&config).unwrap();
    fs::write(
        config.join("configuration.json"),
        json!({
            "endpoint_url": format!("http://{address}"), "tenant_id": "synthetic",
            "keycloak_url": "http://127.0.0.1", "auth_token": "synthetic",
            "refresh_token": "", "client_id": "", "client_secret": "", "username": TRUSTEE
        })
        .to_string(),
    )
    .unwrap();

    let requests = Arc::new(Mutex::new(Vec::new()));
    let recorded = Arc::clone(&requests);
    let observed_key_path = key_path.clone();
    thread::spawn(move || {
        for response in responses {
            let (mut stream, _) = server.accept().unwrap();
            let mut reader = BufReader::new(&mut stream);
            let mut length = 0;
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                if line == "\r\n" {
                    break;
                }
                if let Some(value) = line.to_lowercase().strip_prefix("content-length:") {
                    length = value.trim().parse().unwrap();
                }
            }
            let mut body = vec![0; length];
            reader.read_exact(&mut body).unwrap();
            let request: Value = serde_json::from_slice(&body).unwrap();
            recorded.lock().unwrap().push(Request {
                operation: request["operationName"].as_str().unwrap_or_default().into(),
                variables: request["variables"].clone(),
                stored_key: fs::read_to_string(&observed_key_path).ok(),
            });
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                response.len(),
                response
            )
            .unwrap();
        }
    });

    let output = Command::new(&binary)
        .args([
            "step",
            "complete-key-ceremony",
            "--election-event-id",
            ELECTION_EVENT_ID,
            "--key-ceremony-id",
            "synthetic-ceremony",
        ])
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    let requests = std::mem::take(&mut *requests.lock().unwrap());

    Run {
        output: format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ),
        requests,
        stored_key: fs::read_to_string(&key_path).ok(),
    }
}

fn operations(run: &Run) -> Vec<&str> {
    run.requests
        .iter()
        .map(|request| request.operation.as_str())
        .collect()
}

#[test]
fn stores_the_key_before_checking_it() {
    let run = complete_key_ceremony(None, vec![downloaded("downloaded-key"), checked(true)]);

    assert!(run.output.contains("Success!"), "{}", run.output);
    assert_eq!(operations(&run), ["GetPrivateKey", "CheckPrivateKey"]);
    assert_eq!(
        run.requests[1].stored_key.as_deref(),
        Some("downloaded-key")
    );
    assert_eq!(
        run.requests[1].variables["privateKeyBase64"],
        "downloaded-key"
    );
    assert_eq!(run.stored_key.as_deref(), Some("downloaded-key"));
}

#[test]
fn checks_the_stored_key_once_it_can_no_longer_be_downloaded() {
    let run = complete_key_ceremony(
        Some("stored-key"),
        vec![DOWNLOAD_UNAVAILABLE.to_string(), checked(true)],
    );

    assert!(run.output.contains("Success!"), "{}", run.output);
    assert_eq!(operations(&run), ["GetPrivateKey", "CheckPrivateKey"]);
    assert_eq!(run.requests[1].variables["privateKeyBase64"], "stored-key");
    assert_eq!(run.stored_key.as_deref(), Some("stored-key"));
}

#[test]
fn fails_without_a_stored_key_once_it_can_no_longer_be_downloaded() {
    let run = complete_key_ceremony(None, vec![DOWNLOAD_UNAVAILABLE.to_string()]);

    assert!(run.output.contains("Error!"), "{}", run.output);
    assert!(
        run.output
            .contains("Private key download is no longer available"),
        "{}",
        run.output
    );
    assert_eq!(operations(&run), ["GetPrivateKey"]);
    assert_eq!(run.stored_key, None);
}

#[test]
fn ignores_the_stored_key_when_the_download_fails_for_another_reason() {
    let run = complete_key_ceremony(
        Some("stored-key"),
        vec![
            r#"{"errors":[{"message":"internal error","extensions":{"code":"unexpected"}}]}"#
                .to_string(),
        ],
    );

    assert!(run.output.contains("Error!"), "{}", run.output);
    assert_eq!(operations(&run), ["GetPrivateKey"]);
}

#[test]
fn fails_when_the_key_does_not_pass_the_check() {
    let run = complete_key_ceremony(None, vec![downloaded("downloaded-key"), checked(false)]);

    assert!(run.output.contains("Error!"), "{}", run.output);
    assert!(run.output.contains("Failed to check key"), "{}", run.output);
    assert_eq!(operations(&run), ["GetPrivateKey", "CheckPrivateKey"]);
}
