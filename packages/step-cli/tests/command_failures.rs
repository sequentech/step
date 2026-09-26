// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Exercise the process boundary that shell automation relies on.
use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    net::TcpListener,
    process::Command,
    thread,
};

#[test]
fn failed_http_commands_exit_nonzero() {
    let temporary = tempfile::tempdir().unwrap();
    let binary = temporary.path().join("step-cli");
    let source = env!("CARGO_BIN_EXE_step-cli");
    if fs::hard_link(source, &binary).is_err() {
        fs::copy(source, &binary).unwrap();
    }
    let server = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = server.local_addr().unwrap();
    let config = temporary.path().join("config");
    fs::create_dir(&config).unwrap();
    fs::write(
        config.join("configuration.json"),
        serde_json::json!({
            "endpoint_url": format!("http://{address}"), "tenant_id": "synthetic",
            "keycloak_url": "http://127.0.0.1", "auth_token": "synthetic",
            "refresh_token": "", "client_id": "", "client_secret": "", "username": ""
        })
        .to_string(),
    )
    .unwrap();
    let handler = thread::spawn(move || {
        for _ in 0..2 {
            let (mut stream, _) = server.accept().unwrap();
            let mut reader = BufReader::new(&mut stream);
            let mut request_line = String::new();
            assert!(reader.read_line(&mut request_line).unwrap() > 0);
            let mut content_length = 0;
            loop {
                let mut header = String::new();
                assert!(reader.read_line(&mut header).unwrap() > 0);
                if header == "\r\n" {
                    break;
                }
                let (name, value) = header.split_once(':').unwrap();
                if name.eq_ignore_ascii_case("content-length") {
                    content_length = value.trim().parse::<usize>().unwrap();
                }
            }
            let mut body = vec![0; content_length];
            reader.read_exact(&mut body).unwrap();
            stream.write_all(b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").unwrap();
        }
    });
    for arguments in [
        ["step", "create-tenant", "--slug", "synthetic"],
        ["step", "delete-tenant", "--tenant-id", "synthetic"],
    ] {
        let output = Command::new(&binary).args(arguments).output().unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&output.stderr).contains("503"));
    }
    handler.join().unwrap();
}

#[test]
fn a_failed_voter_generation_reports_the_error_but_exits_zero() {
    let working_directory = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_step-cli"))
        .args(["step", "generate-voters", "--num-users", "1"])
        .arg("--working-directory")
        .arg(working_directory.path())
        .output()
        .unwrap();
    // Pinned as found: scripts cannot tell this failure from a success.
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "Error! Failed to generate voters: Os { code: 2, kind: NotFound, message: \"No such file or directory\" }\n"
    );
    assert!(output.stdout.is_empty());
    assert_eq!(fs::read_dir(working_directory.path()).unwrap().count(), 0);
}
