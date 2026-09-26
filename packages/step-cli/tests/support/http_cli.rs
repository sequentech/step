// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
//! Exercise endpoint selection and token persistence through the shipped CLI.
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

pub fn command(
    args: &[&str],
    token_response: Option<Value>,
) -> (Output, Value, Vec<(String, Vec<u8>)>) {
    let directory = tempfile::tempdir().unwrap();
    let binary = directory.path().join("step-cli");
    fs::hard_link(env!("CARGO_BIN_EXE_step-cli"), &binary)
        .or_else(|_| fs::copy(env!("CARGO_BIN_EXE_step-cli"), &binary).map(|_| ()))
        .unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let config = directory.path().join("config/configuration.json");
    fs::create_dir(config.parent().unwrap()).unwrap();
    fs::write(
        &config,
        json!({"endpoint_url":format!("{endpoint}/graphql"), "tenant_id":"tenant",
        "keycloak_url":endpoint, "auth_token":"old-access", "refresh_token":"stored-refresh",
        "client_id":"client", "client_secret":"secret", "username":"operator"})
        .to_string(),
    )
    .unwrap();
    fs::write(
        directory.path().join("payload.json"),
        b"{\"synthetic\":true}\n",
    )
    .unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let done = stop.clone();
    let peer = thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(15);
        let mut requests = Vec::new();
        while !done.load(Ordering::Relaxed) && Instant::now() < deadline {
            let (mut stream, _) = match listener.accept() {
                Ok(value) => value,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(5));
                    continue;
                }
                Err(error) => panic!("accept: {error}"),
            };
            stream.set_nonblocking(false).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            stream
                .set_write_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut first = String::new();
            if reader.read_line(&mut first).unwrap() == 0 {
                continue;
            }
            let mut length = 0;
            loop {
                let mut header = String::new();
                reader.read_line(&mut header).unwrap();
                if header == "\r\n" {
                    break;
                }
                assert!(!header.is_empty());
                if let Some(value) = header.to_ascii_lowercase().strip_prefix("content-length:") {
                    length = value.trim().parse::<usize>().unwrap();
                }
            }
            assert!(length < 16384);
            let mut body = vec![0; length];
            reader.read_exact(&mut body).unwrap();
            let response = if first.starts_with("POST /graphql ") {
                json!({"data":{"get_upload_url":{"url":format!("{endpoint}/blob?signature=literal"),"document_id":"document"}}})
            } else if first.starts_with("PUT /blob?signature=literal ") {
                json!({})
            } else {
                assert!(
                    first.starts_with("POST /realms/tenant-tenant/protocol/openid-connect/token "),
                    "{first}"
                );
                token_response.clone().expect("unexpected token request")
            };
            requests.push((first, body));
            let body = response.to_string();
            write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
        }
        requests
    });
    let mut process = Command::new(binary);
    process
        .args(["step"])
        .args(args)
        .current_dir(directory.path())
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for name in ["LLVM_PROFILE_FILE", "LD_LIBRARY_PATH"] {
        if let Some(value) = std::env::var_os(name) {
            process.env(name, value);
        }
    }
    let mut child = process.spawn().unwrap();
    let deadline = Instant::now() + Duration::from_secs(15);
    while child.try_wait().unwrap().is_none() && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(10));
    }
    let timed_out = child.try_wait().unwrap().is_none();
    if timed_out {
        child.kill().unwrap();
    }
    let output = child.wait_with_output().unwrap();
    stop.store(true, Ordering::Relaxed);
    let requests = peer.join().unwrap();
    assert!(!timed_out, "CLI timed out");
    (
        output,
        serde_json::from_slice(&fs::read(config).unwrap()).unwrap(),
        requests,
    )
}
