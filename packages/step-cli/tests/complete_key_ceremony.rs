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
const KEY_CEREMONY_ID: &str = "synthetic-ceremony";
const DOWNLOAD_UNAVAILABLE: &str = r#"{"errors":[{"message":"Private key download is no longer available","extensions":{"code":"PrivateKeyDownloadUnavailable"}}]}"#;
/// The same answer when Hasura could not parse it and kept it as text.
const DOWNLOAD_UNAVAILABLE_UNPARSED: &str = r#"{"errors":[{"message":"unexpected","extensions":{"code":"unexpected","internal":{"response":{"status":409,"body":"{\"message\":\"Private key download is no longer available\",\"extensions\":{\"code\":\"PrivateKeyDownloadUnavailable\"}}"}}}}]}"#;

struct Request {
    operation: String,
    variables: Value,
    ceremony_key: Option<String>,
    event_key: Option<String>,
}

struct Run {
    output: String,
    requests: Vec<Request>,
}

impl Run {
    fn operations(&self) -> Vec<&str> {
        self.requests
            .iter()
            .map(|request| request.operation.as_str())
            .collect()
    }
}

/// A CLI installation whose stored keys persist across runs.
struct Cli {
    directory: tempfile::TempDir,
    binary: PathBuf,
}

impl Cli {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let binary = directory.path().join("step-cli");
        let source = env!("CARGO_BIN_EXE_step-cli");
        if fs::hard_link(source, &binary).is_err() {
            fs::copy(source, &binary).unwrap();
        }
        fs::create_dir(directory.path().join("config")).unwrap();
        Cli { directory, binary }
    }

    /// The key stored for a key ceremony, or for the whole election event.
    fn key_path(&self, key_ceremony_id: Option<&str>) -> PathBuf {
        let file_name = match key_ceremony_id {
            Some(key_ceremony_id) => format!(
                "encrypted_private_key_trustee_{TRUSTEE}_{ELECTION_EVENT_ID}_{key_ceremony_id}.txt"
            ),
            None => format!("encrypted_private_key_trustee_{TRUSTEE}_{ELECTION_EVENT_ID}.txt"),
        };
        self.directory.path().join("keys").join(file_name)
    }

    fn store(&self, key_ceremony_id: Option<&str>, private_key: &str) {
        let path = self.key_path(key_ceremony_id);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, private_key).unwrap();
    }

    fn stored(&self, key_ceremony_id: Option<&str>) -> Option<String> {
        fs::read_to_string(self.key_path(key_ceremony_id)).ok()
    }

    /// Completes `key_ceremony_id`, answering each GraphQL request with the
    /// next of `responses`.
    fn complete(&self, key_ceremony_id: &str, responses: Vec<String>) -> Run {
        let server = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = server.local_addr().unwrap();
        fs::write(
            self.directory.path().join("config/configuration.json"),
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
        let ceremony_key_path = self.key_path(Some(key_ceremony_id));
        let event_key_path = self.key_path(None);
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
                    ceremony_key: fs::read_to_string(&ceremony_key_path).ok(),
                    event_key: fs::read_to_string(&event_key_path).ok(),
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

        let output = Command::new(&self.binary)
            .args([
                "step",
                "complete-key-ceremony",
                "--election-event-id",
                ELECTION_EVENT_ID,
                "--key-ceremony-id",
                key_ceremony_id,
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
        }
    }
}

fn downloaded(private_key: &str) -> String {
    json!({"data": {"get_private_key": {"private_key_base64": private_key}}}).to_string()
}

fn checked(is_valid: bool) -> String {
    json!({"data": {"check_private_key": {"is_valid": is_valid}}}).to_string()
}

#[test]
fn stores_the_key_before_checking_it() {
    let cli = Cli::new();

    let run = cli.complete(
        KEY_CEREMONY_ID,
        vec![downloaded("downloaded-key"), checked(true)],
    );

    assert!(run.output.contains("Success!"), "{}", run.output);
    assert_eq!(run.operations(), ["GetPrivateKey", "CheckPrivateKey"]);
    let check = &run.requests[1];
    assert_eq!(check.variables["privateKeyBase64"], "downloaded-key");
    assert_eq!(check.ceremony_key.as_deref(), Some("downloaded-key"));
    assert_eq!(check.event_key, None);
    assert_eq!(
        cli.stored(Some(KEY_CEREMONY_ID)).as_deref(),
        Some("downloaded-key")
    );
    assert_eq!(cli.stored(None).as_deref(), Some("downloaded-key"));
}

#[test]
fn keeps_the_keys_of_each_ceremony_apart() {
    let cli = Cli::new();

    for (key_ceremony_id, private_key) in [
        ("first-ceremony", "first-key"),
        ("second-ceremony", "second-key"),
    ] {
        let run = cli.complete(
            key_ceremony_id,
            vec![downloaded(private_key), checked(true)],
        );
        assert!(run.output.contains("Success!"), "{}", run.output);
    }

    assert_eq!(
        cli.stored(Some("first-ceremony")).as_deref(),
        Some("first-key")
    );
    assert_eq!(
        cli.stored(Some("second-ceremony")).as_deref(),
        Some("second-key")
    );
    assert_eq!(cli.stored(None).as_deref(), Some("second-key"));
}

#[test]
fn checks_the_ceremony_key_once_it_can_no_longer_be_downloaded() {
    let cli = Cli::new();
    cli.store(Some(KEY_CEREMONY_ID), "stored-key");
    cli.store(None, "other-ceremony-key");

    let run = cli.complete(
        KEY_CEREMONY_ID,
        vec![DOWNLOAD_UNAVAILABLE.to_string(), checked(true)],
    );

    assert!(run.output.contains("Success!"), "{}", run.output);
    assert_eq!(run.operations(), ["GetPrivateKey", "CheckPrivateKey"]);
    assert_eq!(run.requests[1].variables["privateKeyBase64"], "stored-key");
    assert_eq!(
        cli.stored(Some(KEY_CEREMONY_ID)).as_deref(),
        Some("stored-key")
    );
    assert_eq!(cli.stored(None).as_deref(), Some("stored-key"));
}

#[test]
fn checks_the_ceremony_key_when_hasura_could_not_parse_the_unavailable_answer() {
    let cli = Cli::new();
    cli.store(Some(KEY_CEREMONY_ID), "stored-key");

    let run = cli.complete(
        KEY_CEREMONY_ID,
        vec![DOWNLOAD_UNAVAILABLE_UNPARSED.to_string(), checked(true)],
    );

    assert!(run.output.contains("Success!"), "{}", run.output);
    assert_eq!(run.operations(), ["GetPrivateKey", "CheckPrivateKey"]);
    assert_eq!(run.requests[1].variables["privateKeyBase64"], "stored-key");
}

#[test]
fn does_not_check_another_ceremony_key_once_it_can_no_longer_be_downloaded() {
    let cli = Cli::new();
    cli.store(Some("other-ceremony"), "other-ceremony-key");
    cli.store(None, "other-ceremony-key");

    let run = cli.complete(KEY_CEREMONY_ID, vec![DOWNLOAD_UNAVAILABLE.to_string()]);

    assert!(run.output.contains("Error!"), "{}", run.output);
    assert!(
        run.output
            .contains("Private key download is no longer available"),
        "{}",
        run.output
    );
    assert_eq!(run.operations(), ["GetPrivateKey"]);
    assert_eq!(cli.stored(Some(KEY_CEREMONY_ID)), None);
    assert_eq!(cli.stored(None).as_deref(), Some("other-ceremony-key"));
}

#[test]
fn ignores_the_stored_key_when_the_download_fails_for_another_reason() {
    let cli = Cli::new();
    cli.store(Some(KEY_CEREMONY_ID), "stored-key");

    let run = cli.complete(
        KEY_CEREMONY_ID,
        vec![
            r#"{"errors":[{"message":"internal error","extensions":{"code":"unexpected"}}]}"#
                .to_string(),
        ],
    );

    assert!(run.output.contains("Error!"), "{}", run.output);
    assert_eq!(run.operations(), ["GetPrivateKey"]);
}

#[test]
fn keeps_a_key_that_fails_the_check_out_of_the_event_key() {
    let cli = Cli::new();

    let run = cli.complete(
        KEY_CEREMONY_ID,
        vec![downloaded("downloaded-key"), checked(false)],
    );

    assert!(run.output.contains("Error!"), "{}", run.output);
    assert!(run.output.contains("Failed to check key"), "{}", run.output);
    assert_eq!(run.operations(), ["GetPrivateKey", "CheckPrivateKey"]);
    assert_eq!(cli.stored(None), None);
}
