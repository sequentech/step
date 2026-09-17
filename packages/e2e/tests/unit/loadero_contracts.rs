// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
use super::*;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::Mutex;
use std::time::Instant;

static ENVIRONMENT: Mutex<()> = Mutex::new(());
struct Environment(Vec<(&'static str, Option<std::ffi::OsString>)>);
impl Environment {
    fn set(values: &[(&'static str, &str)]) -> Self {
        let saved = values
            .iter()
            .map(|(key, value)| {
                let old = env::var_os(key);
                env::set_var(key, value);
                (*key, old)
            })
            .collect();
        Self(saved)
    }
}
impl Drop for Environment {
    fn drop(&mut self) {
        for (key, value) in &self.0 {
            match value {
                Some(value) => env::set_var(key, value),
                None => env::remove_var(key),
            }
        }
    }
}

// Owned loopback port, bounded reads/accepts, explicit HTTP and JSON expectations.
// No request can reach a paid Loadero account.
fn fixture(
    replies: Vec<(&'static str, u16, &'static str)>,
) -> (String, std::thread::JoinHandle<Vec<Value>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    listener.set_nonblocking(true).unwrap();
    let task = std::thread::spawn(move || {
        let mut bodies = Vec::new();
        for (request_line, status, body) in replies {
            let deadline = Instant::now() + Duration::from_secs(5);
            let mut stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        assert!(
                            Instant::now() < deadline,
                            "fixture did not receive {request_line}"
                        );
                        std::thread::sleep(Duration::from_millis(2));
                    }
                    Err(error) => panic!("accept: {error}"),
                }
            };
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            stream
                .set_write_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut request = Vec::new();
            let end = loop {
                let mut byte = [0];
                stream.read_exact(&mut byte).unwrap();
                request.push(byte[0]);
                assert!(request.len() < 65536, "oversized fixture request");
                if request.ends_with(b"\r\n\r\n") {
                    break request.len();
                }
            };
            let headers = std::str::from_utf8(&request).unwrap();
            assert_eq!(headers.lines().next().unwrap(), request_line);
            assert!(headers
                .to_ascii_lowercase()
                .contains("authorization: loaderoauth synthetic-key\r\n"));
            let length = headers
                .lines()
                .find_map(|line| {
                    let (key, value) = line.split_once(':')?;
                    key.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().unwrap())
                })
                .unwrap_or(0);
            request.resize(end + length, 0);
            stream.read_exact(&mut request[end..]).unwrap();
            bodies.push(if length == 0 {
                Value::Null
            } else {
                serde_json::from_slice(&request[end..]).unwrap()
            });
            write!(stream, "HTTP/1.1 {status} Fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
        }
        bodies
    });
    (url, task)
}

#[test]
fn lookup_requires_an_exact_name_and_preserves_http_failures() {
    let _lock = ENVIRONMENT
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let (url, server) = fixture(vec![
        (
            "GET /tests HTTP/1.1",
            200,
            r#"{"results":[{"id":9,"name":"other"},{"id":17,"name":"synthetic"}]}"#,
        ),
        (
            "GET /tests HTTP/1.1",
            200,
            r#"{"results":[{"id":17,"name":"synthetic-extra"}]}"#,
        ),
        (
            "GET /tests HTTP/1.1",
            503,
            r#"{"error":"synthetic unavailable"}"#,
        ),
    ]);
    let _env = Environment::set(&[
        ("LOADERO_API_KEY", "synthetic-key"),
        ("LOADERO_BASE_URL", &url),
    ]);
    assert_eq!(
        get_test_id_by_name("synthetic".into()).unwrap(),
        Some("17".into())
    );
    assert_eq!(get_test_id_by_name("synthetic".into()).unwrap(), None);
    let error = get_test_id_by_name("synthetic".into())
        .unwrap_err()
        .to_string();
    assert!(error.contains("HTTP Status: 503"), "{error}");
    assert!(error.contains("synthetic unavailable"));
    server.join().unwrap();
}

#[test]
fn create_and_launch_check_literal_payloads_and_reject_missing_identifiers() {
    let _lock = ENVIRONMENT
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let (url, server) = fixture(vec![
        ("POST /tests HTTP/1.1", 201, r#"{"id":23}"#),
        ("POST /tests/23/runs/ HTTP/1.1", 201, r#"{"id":41}"#),
        ("POST /tests/23/runs/ HTTP/1.1", 200, r#"{"id":"41"}"#),
        ("POST /tests HTTP/1.1", 200, r#"{}"#),
    ]);
    let _env = Environment::set(&[("LOADERO_API_KEY", "synthetic-key")]);
    let config = TestConfig {
        increment_strategy: "linear".into(),
        mode: "load".into(),
        name: "synthetic".into(),
        participant_timeout: 17,
        script: "literal script".into(),
        start_interval: 3,
    };
    assert_eq!(create_test(&url, &config).unwrap(), "23");
    assert_eq!(launch_test(&url, "23").unwrap(), "41");
    assert!(launch_test(&url, "23")
        .unwrap_err()
        .to_string()
        .contains("No run id found"));
    assert!(create_test(&url, &config)
        .unwrap_err()
        .to_string()
        .contains("No test ID found"));
    let bodies = server.join().unwrap();
    assert_eq!(
        bodies[0],
        json!({"increment_strategy":"linear", "mode":"load", "name":"synthetic", "participant_timeout":17, "script":"literal script", "start_interval":3})
    );
}

#[test]
fn completed_status_reports_both_counts_and_pending_is_not_completion() {
    let _lock = ENVIRONMENT
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let (url, server) = fixture(vec![
        (
            "GET /tests/23/runs/41/ HTTP/1.1",
            200,
            r#"{"status":"done","participant_results":{"pass":7,"fail":2}}"#,
        ),
        (
            "GET /tests/23/runs/41/ HTTP/1.1",
            200,
            r#"{"status":"running"}"#,
        ),
    ]);
    let _env = Environment::set(&[("LOADERO_API_KEY", "synthetic-key")]);
    assert_eq!(check_test_status(&url, "23", "41").unwrap(), Some((7, 2)));
    assert_eq!(check_test_status(&url, "23", "41").unwrap(), None);
    server.join().unwrap();
}

#[test]
fn polling_http_failure_is_not_reported_as_a_successful_run() {
    let _lock = ENVIRONMENT
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let (url, server) = fixture(vec![
        ("POST /tests/23/runs/ HTTP/1.1", 201, r#"{"id":41}"#),
        (
            "GET /tests/23/runs/41/ HTTP/1.1",
            200,
            r#"{"status":"running"}"#,
        ),
        (
            "GET /tests/23/runs/41/ HTTP/1.1",
            200,
            r#"{"status":"done","participant_results":{"pass":1,"fail":0}}"#,
        ),
        ("POST /tests/23/runs/ HTTP/1.1", 201, r#"{"id":42}"#),
        (
            "GET /tests/23/runs/42/ HTTP/1.1",
            503,
            r#"{"error":"synthetic unavailable"}"#,
        ),
    ]);
    let _env = Environment::set(&[
        ("LOADERO_API_KEY", "synthetic-key"),
        ("LOADERO_INTERVAL_POLLING_TIME", "0"),
    ]);
    run_test(&url, "23").unwrap();
    let result = run_test(&url, "23");
    server.join().unwrap();
    assert!(result.unwrap_err().to_string().contains("HTTP Status: 503"));
}

#[test]
fn malformed_poll_response_is_returned_instead_of_retried() {
    let _lock = ENVIRONMENT
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let (url, server) = fixture(vec![
        ("POST /tests/23/runs/ HTTP/1.1", 201, r#"{"id":41}"#),
        ("GET /tests/23/runs/41/ HTTP/1.1", 200, "not JSON"),
        // The old loop retries and returns this different error. The fixed loop
        // returns the decode error; the explicit request below drains the sentinel.
        ("GET /tests/23/runs/41/ HTTP/1.1", 503, "unexpected retry"),
    ]);
    let _env = Environment::set(&[
        ("LOADERO_API_KEY", "synthetic-key"),
        ("LOADERO_INTERVAL_POLLING_TIME", "0"),
    ]);
    let error = run_test(&url, "23").unwrap_err();
    assert_eq!(
        error.to_string(),
        "Failed to parse JSON in check_test_status"
    );
    assert!(error.chain().any(|cause| cause.is::<serde_json::Error>()));
    assert!(check_test_status(&url, "23", "41")
        .unwrap_err()
        .to_string()
        .contains("HTTP Status: 503"));
    server.join().unwrap();
}
