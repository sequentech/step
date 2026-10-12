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
        for index in 0..4 {
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
            if index >= 2 {
                let request: serde_json::Value = serde_json::from_slice(&body).unwrap();
                assert_eq!(request["variables"]["is_local"], index == 3);
            }
            stream.write_all(b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").unwrap();
        }
    });
    let upload = temporary.path().join("upload.json");
    fs::write(&upload, "{}").unwrap();
    for arguments in [
        vec!["step", "create-tenant", "--slug", "synthetic"],
        vec!["step", "delete-tenant", "--tenant-id", "synthetic"],
        vec![
            "step",
            "upload-document",
            "--file-path",
            upload.to_str().unwrap(),
        ],
        vec![
            "step",
            "upload-document",
            "--file-path",
            upload.to_str().unwrap(),
            "--is-local",
        ],
    ] {
        let output = Command::new(&binary).args(arguments).output().unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&output.stderr).contains("503"));
    }
    handler.join().unwrap();
}

#[test]
fn worker_image_context_includes_the_local_url_parser() {
    use std::os::unix::fs::PermissionsExt;

    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path();
    let docker = root.join("docker");
    // Inspect the real CLI's build context without building or publishing an image.
    fs::write(
        &docker,
        r#"#!/bin/sh
set -eu
[ "$1" != "buildx" ] || exit 1
[ "$1" = "build" ]
tar -xf - -C "$MOCK_IMAGE_CONTEXT"
"#,
    )
    .unwrap();
    fs::set_permissions(&docker, fs::Permissions::from_mode(0o700)).unwrap();
    let context = root.join("context");
    fs::create_dir(&context).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_step-cli"))
        .args(["load", "image", "--engine", "k6", "--tag", "synthetic:test"])
        .env(
            "PATH",
            format!("{}:{}", root.display(), std::env::var("PATH").unwrap()),
        )
        .env("MOCK_IMAGE_CONTEXT", &context)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let replay = fs::read_to_string(context.join("packages/voting-load/replay.k6.js")).unwrap();
    assert!(replay.contains(r#"from "./url-1.0.0.js""#));
    let parser = fs::read(context.join("packages/voting-load/url-1.0.0.js")).unwrap();
    assert_eq!(parser, include_bytes!("../../voting-load/url-1.0.0.js"));
}
