// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Exercise the shipped command against an owned loopback HTTP peer. Its config
//! lives beside a private executable link, never in the developer's CLI config.
use serde_json::{json, Value};
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::process::{Command, Output, Stdio};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;
use std::time::{Duration, Instant};

fn import_command(error_stage: Option<&str>) -> (Output, Vec<String>) {
    import_kind(error_stage, None)
}

fn import_kind(error_stage: Option<&str>, tally_command: Option<&str>) -> (Output, Vec<String>) {
    let binary = std::path::Path::new(env!("CARGO_BIN_EXE_step-cli"));
    let directory = tempfile::tempdir_in(binary.parent().unwrap()).unwrap();
    let executable = directory.path().join("step-cli");
    fs::hard_link(binary, &executable).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    listener.set_nonblocking(true).unwrap();
    // Queue a peer that closes before sending a request. The following real
    // command must still complete every expected HTTP exchange.
    drop(std::net::TcpStream::connect(listener.local_addr().unwrap()).unwrap());
    let config = directory.path().join("config");
    fs::create_dir(&config).unwrap();
    fs::write(
        config.join("configuration.json"),
        json!({
            "endpoint_url": format!("{endpoint}/graphql"), "tenant_id": "synthetic-tenant",
            "keycloak_url": endpoint, "auth_token": "synthetic-token", "refresh_token": "synthetic",
            "client_id": "synthetic", "client_secret": "synthetic", "username": "synthetic"
        })
        .to_string(),
    )
    .unwrap();
    let input = directory.path().join("election.json");
    fs::write(&input, "{\"election\":\"synthetic\"}").unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let stop_peer = stop.clone();
    let error_stage = error_stage.map(str::to_owned);
    let server = thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(20);
        let mut operations = Vec::new();
        while !stop_peer.load(Ordering::Relaxed) && Instant::now() < deadline {
            let (mut stream, _) = match listener.accept() {
                Ok(connection) => connection,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(5));
                    continue;
                }
                Err(error) => panic!("local accept failed: {error}"),
            };
            stream
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            stream
                .set_write_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut request = String::new();
            // A connection can close before sending an HTTP request (for
            // example, a readiness probe). It must not consume an exchange.
            if reader.read_line(&mut request).unwrap() == 0 {
                continue;
            }
            let mut length = 0;
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                if line == "\r\n" {
                    break;
                }
                if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                    length = value.trim().parse::<usize>().unwrap();
                }
                assert!(!line.is_empty(), "request headers ended early");
            }
            assert!(length < 16_384, "fixture request unexpectedly large");
            let mut body = vec![0; length];
            reader.read_exact(&mut body).unwrap();
            let (operation, mut response) = if request.starts_with("PUT /blob ") {
                assert_eq!(body, b"{\"election\":\"synthetic\"}");
                ("upload", json!({}))
            } else {
                assert!(request.starts_with("POST /graphql "), "{request}");
                let query: Value = serde_json::from_slice(&body).unwrap();
                match query["operationName"].as_str().unwrap() {
                    "GetUploadUrl" => (
                        "prepare",
                        json!({"data":{"get_upload_url":{
                        "url":format!("{endpoint}/blob"), "document_id":"synthetic-document"}}, "errors":[]}),
                    ),
                    "ImportElectionEvent" => {
                        assert_eq!(query["variables"]["documentId"], "synthetic-document");
                        (
                            "import",
                            json!({"data":{"import_election_event":{
                            "id":"synthetic-election", "message":null, "error":null}}, "errors":[]}),
                        )
                    }
                    "PreviewTallySheetImport" => (
                        "preview",
                        json!({"data":{"preview_tally_sheet_import":{"preview":{"id":"synthetic-preview"}}},"errors":[]}),
                    ),
                    "CreateTallySheetImport" => (
                        "create",
                        json!({"data":{"create_tally_sheet_import":{"tally_sheet_import":{"id":"synthetic-import"}}},"errors":[]}),
                    ),
                    other => panic!("unexpected operation: {other}"),
                }
            };
            if error_stage.as_deref() == Some(operation) {
                response["errors"] = json!([{"message":"synthetic denial"}]);
            }
            operations.push(operation.to_owned());
            let body = response.to_string();
            write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
        }
        operations
    });
    let mut command = Command::new(executable);
    if let Some(tally_command) = tally_command {
        command.args([
            "step",
            "tally-sheet",
            tally_command,
            "--election-event-id",
            "synthetic-event",
            "--document-id",
            "synthetic-document",
        ]);
    } else {
        command
            .args(["step", "import-election", "--file-path"])
            .arg(input);
    }
    command
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for name in ["LLVM_PROFILE_FILE", "LD_LIBRARY_PATH"] {
        if let Some(value) = std::env::var_os(name) {
            command.env(name, value);
        }
    }
    let mut child = command.spawn().unwrap();
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
    let operations = server.join().unwrap();
    assert!(!timed_out, "local import exceeded twenty seconds");
    (output, operations)
}

#[test]
fn complete_import_with_empty_error_lists_reports_the_created_id() {
    let (output, operations) = import_command(None);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("synthetic-election"));
    assert_eq!(operations, ["prepare", "upload", "import"]);
}

fn assert_partial_response_fails(stage: &str, request_count: usize) {
    let (output, operations) = import_command(Some(stage));
    assert!(String::from_utf8_lossy(&output.stderr).contains("synthetic denial"));
    assert!(
        !output.status.success(),
        "a rejected import must fail for shell callers"
    );
    assert!(!String::from_utf8_lossy(&output.stdout).contains("Success!"));
    assert_eq!(operations.len(), request_count);
}

#[test]
fn partial_upload_data_never_uploads_or_reports_success() {
    assert_partial_response_fails("prepare", 1);
}

#[test]
fn partial_import_data_never_reports_success() {
    assert_partial_response_fails("import", 3);
}

#[test]
fn tally_import_source_errors_have_a_failing_exit_status() {
    for command in ["import-preview", "import-create"] {
        let result = Command::new(env!("CARGO_BIN_EXE_step-cli"))
            .args([
                "step",
                "tally-sheet",
                command,
                "--election-event-id",
                "synthetic-event",
            ])
            .output()
            .unwrap();
        assert_eq!(
            result.status.code(),
            Some(1),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(String::from_utf8_lossy(&result.stderr)
            .contains("provide --file-path or --document-id"));
        assert!(!String::from_utf8_lossy(&result.stdout).contains("Success!"));
    }
}

#[test]
fn tally_import_commands_report_backend_errors_and_preserve_successful_json() {
    for (command, operation, expected_id) in [
        ("import-preview", "preview", "synthetic-preview"),
        ("import-create", "create", "synthetic-import"),
    ] {
        let (success, operations) = import_kind(None, Some(command));
        assert!(
            success.status.success(),
            "{}",
            String::from_utf8_lossy(&success.stderr)
        );
        assert_eq!(operations, [operation]);
        assert!(String::from_utf8_lossy(&success.stdout).contains(expected_id));
        let (failure, operations) = import_kind(Some(operation), Some(command));
        assert_eq!(failure.status.code(), Some(1));
        assert_eq!(operations, [operation]);
        assert!(String::from_utf8_lossy(&failure.stderr).contains("synthetic denial"));
        assert!(!String::from_utf8_lossy(&failure.stdout).contains("Success!"));
    }
}
