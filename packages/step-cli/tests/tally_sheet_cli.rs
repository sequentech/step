// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Run the shipped tally-sheet commands against an owned loopback server. The
//! executable and its stored configuration live in a private directory.
use serde_json::{json, Value};
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

const EVENT_UUID: &str = "aaaaaaaa-0000-4000-8000-000000000001";
const IMPORT_UUID: &str = "bbbbbbbb-0000-4000-8000-000000000002";
const TALLY_UUID: &str = "cccccccc-0000-4000-8000-000000000003";
const ABC_DIGEST: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
const OUTAGE: &str = "HTTP Status: 503 Service Unavailable\nError Message: synthetic outage";

#[derive(Clone)]
struct Request {
    line: String,
    authorization: Option<String>,
    body: Vec<u8>,
}

impl Request {
    fn json(&self) -> Value {
        serde_json::from_slice(&self.body).unwrap_or(Value::Null)
    }

    fn operation(&self) -> String {
        self.json()["operationName"]
            .as_str()
            .unwrap_or_default()
            .to_string()
    }
}

type Respond = fn(&Request, &str) -> String;

/// A private installation whose stored endpoint is a loopback server that
/// answers each request with `respond(request, server_url)`.
struct Cli {
    directory: tempfile::TempDir,
    executable: PathBuf,
    requests: Arc<Mutex<Vec<Request>>>,
}

impl Cli {
    fn new(respond: Respond) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let executable = directory.path().join("step-cli");
        let binary = env!("CARGO_BIN_EXE_step-cli");
        if fs::hard_link(binary, &executable).is_err() {
            fs::copy(binary, &executable).unwrap();
        }
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let server = format!("http://{}", listener.local_addr().unwrap());
        fs::create_dir(directory.path().join("config")).unwrap();
        fs::write(
            directory.path().join("config/configuration.json"),
            json!({
                "endpoint_url": format!("{server}/graphql"), "tenant_id": "synthetic-tenant",
                "keycloak_url": server, "auth_token": "synthetic-token", "refresh_token": "synthetic",
                "client_id": "synthetic", "client_secret": "synthetic", "username": "synthetic"
            })
            .to_string(),
        )
        .unwrap();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&requests);
        thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let Some(request) = read_request(&stream) else {
                    continue;
                };
                let response = respond(&request, &server);
                // Record before answering, so a finished command has been recorded.
                recorded.lock().unwrap().push(request);
                let _ = stream.write_all(response.as_bytes());
            }
        });
        Cli {
            directory,
            executable,
            requests,
        }
    }

    fn path(&self, name: &str) -> String {
        self.directory
            .path()
            .join(name)
            .to_str()
            .unwrap()
            .to_string()
    }

    fn run(&self, arguments: &[&str]) -> Output {
        let mut command = Command::new(&self.executable);
        command
            .args(["step", "tally-sheet"])
            .args(arguments)
            .env_clear()
            .stdin(Stdio::null());
        for name in ["LLVM_PROFILE_FILE", "LD_LIBRARY_PATH"] {
            if let Some(value) = std::env::var_os(name) {
                command.env(name, value);
            }
        }
        command.output().unwrap()
    }

    fn requests(&self) -> Vec<Request> {
        self.requests.lock().unwrap().clone()
    }
}

fn read_request(stream: &TcpStream) -> Option<Request> {
    stream.set_read_timeout(Some(Duration::from_secs(5))).ok()?;
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    if reader.read_line(&mut line).ok()? == 0 {
        return None;
    }
    let mut length = 0;
    let mut authorization = None;
    loop {
        let mut header = String::new();
        reader.read_line(&mut header).ok()?;
        if header == "\r\n" || header.is_empty() {
            break;
        }
        let (name, value) = header.split_once(':')?;
        match name.to_ascii_lowercase().as_str() {
            "content-length" => length = value.trim().parse().ok()?,
            "authorization" => authorization = Some(value.trim().to_string()),
            _ => {}
        }
    }
    let mut body = vec![0; length];
    reader.read_exact(&mut body).ok()?;
    Some(Request {
        line: line.trim_end().to_string(),
        authorization,
        body,
    })
}

fn http(status: &str, body: &str) -> String {
    format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}

#[test]
fn failures_are_reported_and_exit_nonzero_only_for_import_preview_and_create() {
    let cli = Cli::new(|_, _| http("503 Service Unavailable", "synthetic outage"));
    let content = cli.path("sheet.json");
    fs::write(&content, r#"{"total_votes":7}"#).unwrap();
    let (missing, output_dir, csv) = (
        cli.path("missing"),
        cli.path("downloads"),
        cli.path("out.csv"),
    );
    let sheet = |file: &str| {
        let arguments = ["create", "--election-event-id", "synthetic-event"];
        let mut arguments = arguments.map(String::from).to_vec();
        for argument in ["--area-id", "area-1", "--contest-id", "contest-1"] {
            arguments.push(argument.into());
        }
        arguments.extend(["--content-file".into(), file.to_string()]);
        arguments
    };
    let no_such_file = "No such file or directory (os error 2)";
    let cases: Vec<(Vec<String>, String, i32)> = vec![
        (
            sheet(&content),
            format!("Failed to create tally sheet: {OUTAGE}"),
            0,
        ),
        (
            sheet(&missing),
            format!("Failed to read content file: {no_such_file}"),
            0,
        ),
        (
            args(&[
                "review",
                "--election-event-id",
                "synthetic-event",
                "--tally-sheet-id",
                "sheet-1",
                "--status",
                "APPROVED",
            ]),
            format!("Failed to review tally sheet: {OUTAGE}"),
            0,
        ),
        (
            args(&[
                "import-preview",
                "--election-event-id",
                "synthetic-event",
                "--document-id",
                "document-1",
            ]),
            format!("Failed to preview tally sheet import: {OUTAGE}"),
            1,
        ),
        (
            args(&[
                "import-create",
                "--election-event-id",
                "synthetic-event",
                "--document-id",
                "document-1",
            ]),
            format!("Failed to create tally sheet import: {OUTAGE}"),
            1,
        ),
        (
            args(&[
                "import-review",
                "--election-event-id",
                "synthetic-event",
                "--import-id",
                "import-1",
                "--decision",
                "DISAPPROVE",
            ]),
            format!("Failed to review tally sheet import: {OUTAGE}"),
            0,
        ),
        (
            args(&["import-list", "--election-event-id", EVENT_UUID]),
            format!("Failed to list tally sheet imports: {OUTAGE}"),
            0,
        ),
        (
            args(&[
                "import-show",
                "--election-event-id",
                EVENT_UUID,
                "--import-id",
                IMPORT_UUID,
            ]),
            format!("Failed to show tally sheet import: {OUTAGE}"),
            0,
        ),
        (
            args(&[
                "import-download-source",
                "--election-event-id",
                EVENT_UUID,
                "--import-id",
                IMPORT_UUID,
                "--output-dir",
                &output_dir,
            ]),
            format!("Failed to download tally sheet import source: {OUTAGE}"),
            0,
        ),
        (
            args(&[
                "recount",
                "--election-event-id",
                EVENT_UUID,
                "--tally-id",
                TALLY_UUID,
            ]),
            format!("Failed to recount tally session: {OUTAGE}"),
            0,
        ),
        (
            args(&["convert-ess-xml", "--input", &missing, "--output", &csv]),
            format!("Failed to convert ES&S XML: {no_such_file}"),
            0,
        ),
    ];
    for (arguments, message, code) in cases {
        let arguments: Vec<&str> = arguments.iter().map(String::as_str).collect();
        let output = cli.run(&arguments);
        assert_eq!(
            String::from_utf8_lossy(&output.stderr),
            format!("Error! {message}\n"),
            "{arguments:?}"
        );
        // Pinned as found: only the import commands fail for shell callers.
        assert_eq!(output.status.code(), Some(code), "{arguments:?}");
        assert!(output.stdout.is_empty(), "{arguments:?}");
    }
    let requests = cli.requests();
    assert_eq!(
        requests.iter().map(Request::operation).collect::<Vec<_>>(),
        [
            "CreateNewTallySheet",
            "ReviewTallySheet",
            "PreviewTallySheetImport",
            "CreateTallySheetImport",
            "ReviewTallySheetImport",
            "ListTallySheetImports",
            "GetTallySheetImport",
            "GetTallySheetImport",
            "RecountTallySession"
        ]
    );
    for request in requests {
        assert_eq!(request.line, "POST /graphql HTTP/1.1");
        assert_eq!(
            request.authorization.as_deref(),
            Some("Bearer synthetic-token")
        );
    }
}

fn args(arguments: &[&str]) -> Vec<String> {
    arguments
        .iter()
        .map(|argument| argument.to_string())
        .collect()
}

#[test]
fn a_local_source_is_uploaded_to_the_event_before_it_is_previewed() {
    let cli = Cli::new(|request, server| {
        match request.operation().as_str() {
        "GetUploadUrl" => http(
            "200 OK",
            &json!({"data": {"get_upload_url": {
                "url": format!("{server}/upload"), "document_id": "uploaded-document"
            }}})
            .to_string(),
        ),
        "PreviewTallySheetImport" => http(
            "200 OK",
            &json!({"data": {"preview_tally_sheet_import": {"preview": {"id": "synthetic-preview"}}}})
                .to_string(),
        ),
        _ => http("200 OK", ""),
    }
    });
    let source = cli.path("results.csv");
    fs::write(&source, "abc").unwrap();
    let output = cli.run(&[
        "import-preview",
        "--election-event-id",
        "synthetic-event",
        "--source-format",
        "CANONICAL_CSV",
        "--file-path",
        &source,
    ]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("synthetic-preview"));
    let requests = cli.requests();
    assert_eq!(
        requests
            .iter()
            .map(|request| request.line.as_str())
            .collect::<Vec<_>>(),
        [
            "POST /graphql HTTP/1.1",
            "PUT /upload HTTP/1.1",
            "POST /graphql HTTP/1.1"
        ]
    );
    assert_eq!(
        requests[0].json()["variables"],
        json!({
            "name": "results.csv", "media_type": "text/csv", "size": 3,
            "is_public": false, "is_local": false, "election_event_id": "synthetic-event"
        })
    );
    assert_eq!(requests[1].body, b"abc");
    assert_eq!(
        requests[2].json()["variables"],
        json!({
            "election_event_id": "synthetic-event", "document_id": "uploaded-document",
            "sha256": ABC_DIGEST, "source_format": "CANONICAL_CSV", "selected_channel": "PAPER"
        })
    );
}
