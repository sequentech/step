// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Drive `complete-key-ceremony` against a synthetic GraphQL endpoint.
use protocol_board::{CustodyRequest, TrusteeSecrets};
use serde_json::{json, Value};
use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    net::TcpListener,
    path::{Path, PathBuf},
    process::Command,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
};

const TRUSTEE: &str = "trustee1";
const ELECTION_EVENT_ID: &str = "synthetic-event";
const KEY_CEREMONY_ID: &str = "synthetic-ceremony";
const DOWNLOAD_UNAVAILABLE: &str = r#"{"errors":[{"message":"Custody request download is no longer available","extensions":{"code":"CustodyRequestDownloadUnavailable"}}]}"#;
/// The same answer when Hasura could not parse it and kept it as text.
const DOWNLOAD_UNAVAILABLE_UNPARSED: &str = r#"{"errors":[{"message":"unexpected","extensions":{"code":"unexpected","internal":{"response":{"status":409,"body":"{\"message\":\"Custody request download is no longer available\",\"extensions\":{\"code\":\"CustodyRequestDownloadUnavailable\"}}"}}}}]}"#;

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
    secrets: TrusteeSecrets,
    keys_file: PathBuf,
}

impl Cli {
    fn new() -> Self {
        let source = Path::new(env!("CARGO_BIN_EXE_step-cli"));
        // A same-filesystem hard link avoids writable executable descriptors,
        // which concurrent child processes can retain until exec (ETXTBSY).
        let directory = tempfile::tempdir_in(source.parent().unwrap()).unwrap();
        let binary = directory.path().join("step-cli");
        // Each installation still has its own executable path, config and keys.
        fs::hard_link(source, &binary).unwrap();
        fs::create_dir(directory.path().join("config")).unwrap();
        let secrets = TrusteeSecrets::generate();
        let keys_file = directory.path().join("trustee.toml");
        fs::write(&keys_file, secrets.to_toml().unwrap()).unwrap();
        Cli {
            directory,
            binary,
            secrets,
            keys_file,
        }
    }

    /// The key stored for a key ceremony, or for the whole election event.
    fn key_path(&self, key_ceremony_id: Option<&str>) -> PathBuf {
        let file_name = match key_ceremony_id {
            Some(key_ceremony_id) => format!(
                "custody_request_trustee_{TRUSTEE}_{ELECTION_EVENT_ID}_{key_ceremony_id}.json"
            ),
            None => format!("custody_request_trustee_{TRUSTEE}_{ELECTION_EVENT_ID}.json"),
        };
        self.directory.path().join("keys").join(file_name)
    }

    fn request(&self, key_ceremony_id: &str) -> CustodyRequest {
        let keys = self.secrets.public_keys().unwrap();
        CustodyRequest {
            tenant_id: "synthetic".into(),
            election_event_id: ELECTION_EVENT_ID.into(),
            keys_ceremony_id: key_ceremony_id.into(),
            tally_session_id: None,
            trustee_name: TRUSTEE.into(),
            signing_public_key: keys.signing_public_key,
            share_encryption_public_key: keys.share_encryption_public_key,
        }
    }

    fn store(&self, key_ceremony_id: Option<&str>, custody_request: &str) {
        let path = self.key_path(key_ceremony_id);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, custody_request).unwrap();
    }

    fn stored(&self, key_ceremony_id: Option<&str>) -> Option<String> {
        fs::read_to_string(self.key_path(key_ceremony_id)).ok()
    }

    /// Completes `key_ceremony_id`, answering each GraphQL request with the
    /// next of `responses`.
    fn complete(&self, key_ceremony_id: &str, responses: Vec<String>) -> Run {
        self.execute(
            &[
                "step",
                "complete-key-ceremony",
                "--election-event-id",
                ELECTION_EVENT_ID,
                "--key-ceremony-id",
                key_ceremony_id,
            ],
            key_ceremony_id,
            responses,
        )
    }

    fn tally(&self, tally_id: &str) -> Run {
        self.execute(
            &[
                "step",
                "confirm-key-tally",
                "--election-event-id",
                ELECTION_EVENT_ID,
                "--tally-id",
                tally_id,
            ],
            KEY_CEREMONY_ID,
            vec![json!({"data": {"authorize_tally_custody": {"is_valid": true}}}).to_string()],
        )
    }

    fn execute(&self, arguments: &[&str], key_ceremony_id: &str, responses: Vec<String>) -> Run {
        let server = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = server.local_addr().unwrap();
        server.set_nonblocking(true).unwrap();
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
        let finished = Arc::new(AtomicBool::new(false));
        let server_finished = Arc::clone(&finished);
        let worker = thread::spawn(move || {
            for response in responses {
                let (mut stream, _) = loop {
                    match server.accept() {
                        Ok(connection) => break connection,
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            if server_finished.load(Ordering::Acquire) {
                                return;
                            }
                            thread::sleep(std::time::Duration::from_millis(5));
                        }
                        Err(error) => panic!("synthetic endpoint: {error}"),
                    }
                };
                stream
                    .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                    .unwrap();
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
            .args(arguments)
            .arg("--trustee-config")
            .arg(&self.keys_file)
            .env("NO_COLOR", "1")
            .output()
            .unwrap();
        finished.store(true, Ordering::Release);
        worker.join().unwrap();
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

fn downloaded(custody_request: &str) -> String {
    json!({"data": {"get_custody_request": {"custody_request_json": custody_request}}}).to_string()
}

fn checked(is_valid: bool) -> String {
    json!({"data": {"confirm_key_custody": {"is_valid": is_valid}}}).to_string()
}

#[test]
fn stores_the_public_request_before_submitting_a_native_custody_proof() {
    let cli = Cli::new();
    let request = cli.request(KEY_CEREMONY_ID);
    let encoded = serde_json::to_string(&request).unwrap();
    let secrets_before = fs::read_to_string(&cli.keys_file).unwrap();
    let run = cli.complete(KEY_CEREMONY_ID, vec![downloaded(&encoded), checked(true)]);
    assert!(run.output.contains("Success!"), "{}", run.output);
    assert_eq!(run.operations(), ["GetCustodyRequest", "ConfirmKeyCustody"]);
    let check = &run.requests[1];
    let proof = check.variables["custodyProofJson"].as_str().unwrap();
    assert!(request.verifies(proof).unwrap());
    assert_ne!(proof, encoded);
    assert_eq!(check.ceremony_key.as_deref(), Some(encoded.as_str()));
    assert_eq!(check.event_key, None);
    assert_eq!(
        cli.stored(Some(KEY_CEREMONY_ID)).as_deref(),
        Some(encoded.as_str())
    );
    assert_eq!(cli.stored(None).as_deref(), Some(encoded.as_str()));
    assert_eq!(fs::read_to_string(&cli.keys_file).unwrap(), secrets_before);
    assert!(!proof.contains("signing_key"));
    assert!(!proof.contains("share_encryption_key"));
}

#[test]
fn keeps_the_public_requests_of_each_ceremony_apart() {
    let cli = Cli::new();
    let first = serde_json::to_string(&cli.request("first-ceremony")).unwrap();
    let second = serde_json::to_string(&cli.request("second-ceremony")).unwrap();
    for (ceremony, request) in [("first-ceremony", &first), ("second-ceremony", &second)] {
        let run = cli.complete(ceremony, vec![downloaded(request), checked(true)]);
        assert!(run.output.contains("Success!"), "{}", run.output);
    }
    assert_eq!(
        cli.stored(Some("first-ceremony")).as_deref(),
        Some(first.as_str())
    );
    assert_eq!(
        cli.stored(Some("second-ceremony")).as_deref(),
        Some(second.as_str())
    );
    assert_eq!(cli.stored(None).as_deref(), Some(second.as_str()));
}

#[test]
fn separate_installations_keep_their_configuration_and_custody_requests_apart() {
    let first = Cli::new();
    let second = Cli::new();
    let first_request = serde_json::to_string(&first.request(KEY_CEREMONY_ID)).unwrap();
    let second_request = serde_json::to_string(&second.request(KEY_CEREMONY_ID)).unwrap();

    for (cli, request) in [(&first, &first_request), (&second, &second_request)] {
        let run = cli.complete(KEY_CEREMONY_ID, vec![downloaded(request), checked(true)]);
        assert!(run.output.contains("Success!"), "{}", run.output);
        assert_eq!(run.operations(), ["GetCustodyRequest", "ConfirmKeyCustody"]);
        assert!(cli
            .request(KEY_CEREMONY_ID)
            .verifies(
                run.requests[1].variables["custodyProofJson"]
                    .as_str()
                    .unwrap()
            )
            .unwrap());
    }

    assert_eq!(
        first.stored(Some(KEY_CEREMONY_ID)),
        Some(first_request.clone())
    );
    assert_eq!(first.stored(None), Some(first_request));
    assert_eq!(
        second.stored(Some(KEY_CEREMONY_ID)),
        Some(second_request.clone())
    );
    assert_eq!(second.stored(None), Some(second_request));
}

#[test]
fn reuses_only_this_ceremonys_public_request_when_download_is_unavailable() {
    for unavailable in [DOWNLOAD_UNAVAILABLE, DOWNLOAD_UNAVAILABLE_UNPARSED] {
        let cli = Cli::new();
        let request = cli.request(KEY_CEREMONY_ID);
        let encoded = serde_json::to_string(&request).unwrap();
        let other = serde_json::to_string(&cli.request("other-ceremony")).unwrap();
        cli.store(Some(KEY_CEREMONY_ID), &encoded);
        cli.store(None, &other);
        let run = cli.complete(KEY_CEREMONY_ID, vec![unavailable.into(), checked(true)]);
        assert!(run.output.contains("Success!"), "{}", run.output);
        assert_eq!(run.operations(), ["GetCustodyRequest", "ConfirmKeyCustody"]);
        assert!(request
            .verifies(
                run.requests[1].variables["custodyProofJson"]
                    .as_str()
                    .unwrap()
            )
            .unwrap());
        assert_eq!(cli.stored(None).as_deref(), Some(encoded.as_str()));
    }
}

#[test]
fn does_not_reuse_another_ceremonys_public_request() {
    let cli = Cli::new();
    let other = serde_json::to_string(&cli.request("other-ceremony")).unwrap();
    cli.store(Some("other-ceremony"), &other);
    cli.store(None, &other);
    let run = cli.complete(KEY_CEREMONY_ID, vec![DOWNLOAD_UNAVAILABLE.into()]);
    assert!(run.output.contains("Error!"), "{}", run.output);
    assert_eq!(run.operations(), ["GetCustodyRequest"]);
    assert_eq!(cli.stored(Some(KEY_CEREMONY_ID)), None);
    assert_eq!(cli.stored(None).as_deref(), Some(other.as_str()));
}

#[test]
fn does_not_use_cached_requests_for_unrelated_download_errors() {
    let cli = Cli::new();
    cli.store(
        Some(KEY_CEREMONY_ID),
        &serde_json::to_string(&cli.request(KEY_CEREMONY_ID)).unwrap(),
    );
    let run = cli.complete(
        KEY_CEREMONY_ID,
        vec![
            r#"{"errors":[{"message":"internal error","extensions":{"code":"unexpected"}}]}"#
                .into(),
        ],
    );
    assert!(run.output.contains("Error!"), "{}", run.output);
    assert_eq!(run.operations(), ["GetCustodyRequest"]);
}

#[test]
fn does_not_promote_a_rejected_custody_request_to_the_event() {
    let cli = Cli::new();
    let encoded = serde_json::to_string(&cli.request(KEY_CEREMONY_ID)).unwrap();
    let run = cli.complete(KEY_CEREMONY_ID, vec![downloaded(&encoded), checked(false)]);
    assert!(
        run.output.contains("Failed to confirm native key custody"),
        "{}",
        run.output
    );
    assert_eq!(run.operations(), ["GetCustodyRequest", "ConfirmKeyCustody"]);
    assert_eq!(cli.stored(None), None);
}

#[test]
fn refuses_requests_that_do_not_match_both_local_public_keys() {
    let cli = Cli::new();
    let mut request = cli.request(KEY_CEREMONY_ID);
    request.share_encryption_public_key = TrusteeSecrets::generate()
        .public_keys()
        .unwrap()
        .share_encryption_public_key;
    let run = cli.complete(
        KEY_CEREMONY_ID,
        vec![
            downloaded(&serde_json::to_string(&request).unwrap()),
            checked(true),
        ],
    );
    assert!(
        run.output.contains("local trustee keys do not match"),
        "{}",
        run.output
    );
    assert_eq!(run.operations(), ["GetCustodyRequest"]);
    assert_eq!(cli.stored(None), None);
}

#[test]
fn refuses_requests_for_another_trustee_tenant_event_ceremony_or_tally() {
    for scope in 0..5 {
        let cli = Cli::new();
        let mut request = cli.request(KEY_CEREMONY_ID);
        match scope {
            0 => request.tenant_id = "another-tenant".into(),
            1 => request.election_event_id = "another-event".into(),
            2 => request.keys_ceremony_id = "another-ceremony".into(),
            3 => request.trustee_name = "another-trustee".into(),
            4 => request.tally_session_id = Some("tally-session".into()),
            _ => unreachable!(),
        }
        let run = cli.complete(
            KEY_CEREMONY_ID,
            vec![
                downloaded(&serde_json::to_string(&request).unwrap()),
                checked(true),
            ],
        );
        assert!(
            run.output.contains("custody request does not match"),
            "scope {scope}: {}",
            run.output
        );
        assert_eq!(run.operations(), ["GetCustodyRequest"]);
        assert_eq!(cli.stored(None), None);
    }
}

#[test]
fn tally_attests_for_the_requested_session_without_changing_cached_requests_or_secrets() {
    let cli = Cli::new();
    let mut request = cli.request(KEY_CEREMONY_ID);
    let encoded = serde_json::to_string(&request).unwrap();
    cli.store(None, &encoded);
    let original_keys = fs::read_to_string(&cli.keys_file).unwrap();
    let run = cli.tally("synthetic-tally");
    assert!(run.output.contains("Success!"), "{}", run.output);
    assert_eq!(run.operations(), ["AuthorizeTallyCustody"]);
    assert_eq!(
        run.requests[0].variables["tallySessionId"],
        "synthetic-tally"
    );
    request.tally_session_id = Some("synthetic-tally".into());
    let proof = run.requests[0].variables["custodyProofJson"]
        .as_str()
        .unwrap();
    assert!(request.verifies(proof).unwrap());
    request.tally_session_id = Some("another-tally".into());
    assert!(!request.verifies(proof).unwrap());
    assert_eq!(cli.stored(None).as_deref(), Some(encoded.as_str()));
    assert_eq!(fs::read_to_string(&cli.keys_file).unwrap(), original_keys);
}

#[test]
fn tally_refuses_a_cached_request_with_another_scope() {
    for scope in 0..4 {
        let cli = Cli::new();
        let mut request = cli.request(KEY_CEREMONY_ID);
        match scope {
            0 => request.tenant_id = "another-tenant".into(),
            1 => request.election_event_id = "another-event".into(),
            2 => request.trustee_name = "another-trustee".into(),
            3 => request.tally_session_id = Some("old-tally".into()),
            _ => unreachable!(),
        }
        cli.store(None, &serde_json::to_string(&request).unwrap());
        let run = cli.tally("synthetic-tally");
        assert!(
            run.output.contains("custody request does not match"),
            "scope {scope}: {}",
            run.output
        );
        assert!(run.requests.is_empty());
    }
}
