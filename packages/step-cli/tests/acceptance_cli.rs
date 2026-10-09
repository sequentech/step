// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Exit codes of `step-cli acceptance`, which scripts use as the stage's verdict.
use sequent_core::{
    ballot::{HashableBallot, SignedHashableBallot},
    encrypt::{encrypt_decoded_contest, hash_ballot_sha512},
    fixtures::ballot_codec::{get_writein_ballot_style, get_writein_plaintext},
};
use serde_json::{json, Value};
use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    net::TcpListener,
    path::{Path, PathBuf},
    process::{Command, Output},
    thread,
};
use strand::backend::ristretto::RistrettoCtx;

const STAGE: &str = "id: demo\ntitle: Demo\nchecks:\n  - id: seen\n    title: Seen\n    method: witnessed\n  - id: optional\n    title: Optional\n    severity: advisory\n    method: witnessed\n";

struct Cli {
    directory: tempfile::TempDir,
    binary: PathBuf,
}

impl Cli {
    /// A private copy of the binary with its configuration beside it.
    fn new() -> Self {
        Self::with_endpoint("http://127.0.0.1:9")
    }

    fn with_endpoint(endpoint: &str) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let binary = directory.path().join("step-cli");
        let source = env!("CARGO_BIN_EXE_step-cli");
        if fs::hard_link(source, &binary).is_err() {
            fs::copy(source, &binary).unwrap();
        }
        let config = directory.path().join("config");
        fs::create_dir(&config).unwrap();
        fs::write(
            config.join("configuration.json"),
            serde_json::json!({
                "endpoint_url": format!("{endpoint}/graphql"), "tenant_id": "tenant",
                "keycloak_url": endpoint, "auth_token": "synthetic",
                "refresh_token": "refresh", "client_id": "", "client_secret": "",
                "username": "operator"
            })
            .to_string(),
        )
        .unwrap();
        fs::write(directory.path().join("stage.yaml"), STAGE).unwrap();
        Self { directory, binary }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.directory.path().join(name)
    }

    fn run(&self, arguments: &[&str]) -> Output {
        Command::new(&self.binary)
            .arg("acceptance")
            .args(arguments)
            .current_dir(self.directory.path())
            .output()
            .unwrap()
    }

    fn open(&self) {
        let output = self.run(&[
            "open",
            "stage.yaml",
            "--ledger",
            "ledger.jsonl",
            "--election-event-id",
            "event",
        ]);
        assert!(output.status.success(), "{}", text(&output.stderr));
        assert!(text(&output.stdout).contains("Ledger head: "));
    }

    fn record(&self, check: &str, outcome: &str) -> Output {
        self.run(&[
            "record",
            "ledger.jsonl",
            "--check",
            check,
            "--outcome",
            outcome,
            "--witness",
            "Witness",
        ])
    }
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn recorder(ledger: &Path) -> String {
    let first = fs::read_to_string(ledger).unwrap();
    let entry: serde_json::Value = serde_json::from_str(first.lines().next().unwrap()).unwrap();
    entry["recorder"].as_str().unwrap().into()
}

#[test]
fn the_verdict_is_the_exit_code() {
    let cli = Cli::new();
    cli.open();
    assert_eq!(recorder(&cli.path("ledger.jsonl")), "operator");

    let incomplete = cli.run(&["verdict", "ledger.jsonl"]);
    assert_eq!(incomplete.status.code(), Some(1));
    assert!(text(&incomplete.stdout).contains("Stage demo: incomplete"));
    assert!(text(&incomplete.stderr).contains("Stage demo is incomplete"));

    assert!(cli.record("seen", "fail").status.success());
    let failed = cli.run(&["verdict", "ledger.jsonl", "--report", "report.md"]);
    assert_eq!(failed.status.code(), Some(1));
    assert!(text(&failed.stdout).contains("Stage demo: failed"));
    assert!(fs::read_to_string(cli.path("report.md"))
        .unwrap()
        .starts_with("# Demo: failed"));

    assert!(cli.record("seen", "pass").status.success());
    assert!(cli.record("optional", "fail").status.success());
    let passed = cli.run(&["verdict", "ledger.jsonl"]);
    assert!(passed.status.success(), "{}", text(&passed.stderr));
    assert!(text(&passed.stdout).contains("Stage demo: passed"));
}

#[test]
fn an_altered_ledger_and_invalid_records_exit_nonzero() {
    let cli = Cli::new();
    cli.open();
    assert!(!cli.record("unknown", "pass").status.success());
    assert!(!cli.record("seen", "maybe").status.success());
    assert!(cli.record("seen", "fail").status.success());

    let ledger = cli.path("ledger.jsonl");
    let original = fs::read_to_string(&ledger).unwrap();
    assert_eq!(original.lines().count(), 2);
    fs::write(&ledger, original.replace("\"fail\"", "\"pass\"")).unwrap();
    for arguments in [
        vec!["verdict", "ledger.jsonl"],
        vec![
            "record",
            "ledger.jsonl",
            "--check",
            "seen",
            "--outcome",
            "pass",
            "--witness",
            "Witness",
        ],
    ] {
        let output = cli.run(&arguments);
        assert_eq!(output.status.code(), Some(1));
        assert!(text(&output.stderr).contains("Ledger entry 1 was altered"));
    }
}

#[test]
fn init_and_open_never_overwrite() {
    let cli = Cli::new();
    for template in ["voting", "pqri-voting"] {
        let name = format!("{template}.yaml");
        let created = cli.run(&["init", "--template", template, "--output", &name]);
        assert!(created.status.success(), "{}", text(&created.stderr));
        assert!(!cli
            .run(&["init", "--template", template, "--output", &name])
            .status
            .success());
    }
    assert!(!cli
        .run(&["init", "--template", "unknown", "--output", "other.yaml"])
        .status
        .success());

    cli.open();
    let again = cli.run(&[
        "open",
        "stage.yaml",
        "--ledger",
        "ledger.jsonl",
        "--election-event-id",
        "event",
    ]);
    assert!(!again.status.success());
    assert!(text(&again.stderr).contains("Cannot create ledger"));
}

#[test]
fn run_needs_a_session_before_it_records_anything() {
    let cli = Cli::new();
    cli.open();
    let before = fs::read_to_string(cli.path("ledger.jsonl")).unwrap();
    let output = cli.run(&["run", "ledger.jsonl", "--voter", "ana"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(text(&output.stderr).contains("Cannot refresh the session"));
    assert_eq!(
        fs::read_to_string(cli.path("ledger.jsonl")).unwrap(),
        before
    );
}

/// Answer the session refresh and the acceptance queries of one voter who voted.
fn election_event(server: TcpListener, requests: usize, logged_hash: Vec<u8>, content: String) {
    for _ in 0..requests {
        let (mut stream, _) = server.accept().unwrap();
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut first = String::new();
        reader.read_line(&mut first).unwrap();
        let mut length = 0;
        loop {
            let mut header = String::new();
            reader.read_line(&mut header).unwrap();
            if header == "\r\n" {
                break;
            }
            if let Some(value) = header.to_ascii_lowercase().strip_prefix("content-length:") {
                length = value.trim().parse::<usize>().unwrap();
            }
        }
        let mut body = vec![0; length];
        reader.read_exact(&mut body).unwrap();
        let response = if first.starts_with("POST /realms/tenant-tenant/") {
            json!({"access_token": "fresh"})
        } else {
            let request: Value = serde_json::from_slice(&body).unwrap();
            let query = request["query"].as_str().unwrap();
            let kind = &request["variables"]["filter"]["statement_kind"];
            let log = |message: Value| {
                json!({"data": {"listElectoralLog": {"items": [
                    {"statement_timestamp": 4102444800_i64, "message": message.to_string()}
                ]}}})
            };
            if query.contains("AcceptanceVoter") {
                json!({"data": {"get_users": {"items": [
                    {"id": "voter-1", "username": "ana", "enabled": true,
                     "area": {"id": "area-1", "name": "Area One"}}
                ]}}})
            } else if kind == "KeycloakUserEvent" {
                log(json!({"statement": {"head": {"log_type": "INFO"},
                    "body": {"KeycloakUserEvent": ["null", "LOGIN"]}}}))
            } else if kind == "CastVote" {
                log(
                    json!({"ballot_id": "ballot-1", "statement": {"body": {"CastVoteWithChannel":
                    ["election", [0], logged_hash, "", "", "ONLINE"]}}}),
                )
            } else if query.contains("AcceptanceBallotStyles") {
                json!({"data": {"sequent_backend_ballot_style": [{"area_id": "area-1"}]}})
            } else {
                assert!(query.contains("AcceptanceCastVotes"), "{query}");
                json!({"data": {"sequent_backend_cast_vote": [
                    {"voter_id_string": "voter-1", "ballot_id": "ballot-1", "content": content,
                     "created_at": "2100-01-01T00:00:00+00:00"}
                ]}})
            }
        };
        let body = response.to_string();
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
        .unwrap();
    }
}

#[test]
fn run_decides_the_automatic_checks_from_the_election_event() {
    let ballot = encrypt_decoded_contest(
        &RistrettoCtx,
        &vec![get_writein_plaintext()],
        &get_writein_ballot_style(),
    )
    .unwrap();
    let signed = SignedHashableBallot::try_from(&ballot).unwrap();
    let hash = hash_ballot_sha512(&HashableBallot::try_from(&signed).unwrap()).unwrap();
    let content = serde_json::to_string(&signed).unwrap();

    for (logged_hash, passes) in [(hash.to_vec(), true), (vec![0; 64], false)] {
        let server = TcpListener::bind("127.0.0.1:0").unwrap();
        let cli = Cli::with_endpoint(&format!("http://{}", server.local_addr().unwrap()));
        let created = cli.run(&["init", "--output", "voting.yaml"]);
        assert!(created.status.success());
        let opened = cli.run(&[
            "open",
            "voting.yaml",
            "--ledger",
            "ledger.jsonl",
            "--election-event-id",
            "event",
        ]);
        assert!(opened.status.success(), "{}", text(&opened.stderr));

        // One session refresh, then the voter, both log kinds, the styles and the cast votes.
        let content = content.clone();
        let peer = thread::spawn(move || election_event(server, 6, logged_hash, content));
        let output = cli.run(&[
            "run",
            "ledger.jsonl",
            "--voter",
            "ana",
            "--ballot-id",
            "ballot-1",
        ]);
        peer.join().unwrap();
        let stdout = text(&output.stdout);
        assert_eq!(output.status.success(), passes, "{}", text(&output.stderr));
        for check in [
            "voting.authentication",
            "voting.ballot-published",
            "voting.ballot-cast",
            "voting.receipt",
        ] {
            assert!(stdout.contains(&format!("pass     {check}")), "{stdout}");
        }
        let stored = if passes { "pass" } else { "fail" };
        assert!(stdout.contains(&format!("{stored}     voting.ballot-stored")));

        assert!(cli
            .record("voting.ballot-displayed", "pass")
            .status
            .success());
        let verdict = cli.run(&["verdict", "ledger.jsonl"]);
        assert_eq!(verdict.status.success(), passes);
        let ledger = fs::read_to_string(cli.path("ledger.jsonl")).unwrap();
        assert!(!ledger.contains("\"contests\""));
    }
}
