// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Exercise the process boundary that shell automation relies on.
use std::{
    fs,
    io::{Read, Write},
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
            let mut request = [0; 8192];
            stream.read(&mut request).unwrap();
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
