// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Exercise the public realm-upsert path against a bounded synthetic HTTP peer.
#![cfg(feature = "keycloak")]

use keycloak::{KeycloakAdmin, KeycloakAdminToken};
use sequent_core::services::keycloak::KeycloakAdminClient;
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::thread;
use std::time::{Duration, Instant};

struct Peer {
    client: KeycloakAdminClient,
    worker: thread::JoinHandle<Vec<Value>>,
}

impl Peer {
    fn start(realm_status: u16, existing: Option<bool>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let worker = thread::spawn(move || {
            let mut writes = vec![];
            let expected = if realm_status == 403 {
                1
            } else if realm_status == 200 {
                3
            } else {
                2
            };
            for _ in 0..expected {
                let deadline = Instant::now() + Duration::from_secs(10);
                let (mut stream, _) = loop {
                    match listener.accept() {
                        Ok(pair) => break pair,
                        Err(e)
                            if e.kind() == std::io::ErrorKind::WouldBlock =>
                        {
                            assert!(
                                Instant::now() < deadline,
                                "upsert request missing"
                            );
                            thread::sleep(Duration::from_millis(5));
                        }
                        Err(e) => panic!("accept: {e}"),
                    }
                };
                stream.set_nonblocking(false).unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(10)))
                    .unwrap();
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                let method =
                    line.split_whitespace().next().unwrap().to_string();
                let path = line.split_whitespace().nth(1).unwrap().to_string();
                let mut length = 0;
                loop {
                    let mut header = String::new();
                    assert!(reader.read_line(&mut header).unwrap() > 0);
                    if header == "\r\n" {
                        break;
                    }
                    if let Some((name, value)) = header.split_once(':') {
                        if name.eq_ignore_ascii_case("content-length") {
                            length = value.trim().parse().unwrap();
                        }
                    }
                }
                let mut body = vec![0; length];
                reader.read_exact(&mut body).unwrap();
                let (status, response) = if method == "GET"
                    && path.contains("/clients?")
                {
                    (200, existing.map_or(json!([]), |enabled| json!([{"clientId":"api-key-client", "enabled":enabled, "id":"existing"}])))
                } else if method == "GET" {
                    (realm_status, json!({}))
                } else {
                    assert_eq!(
                        method,
                        if realm_status == 404 { "POST" } else { "PUT" }
                    );
                    writes.push(serde_json::from_slice(&body).unwrap());
                    (204, json!(null))
                };
                let response = if status == 204 {
                    String::new()
                } else {
                    response.to_string()
                };
                write!(stream, "HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{response}", response.len()).unwrap();
            }
            writes
        });
        let token: KeycloakAdminToken = serde_json::from_value(json!({"access_token":"synthetic", "expires_in":300,"scope":"openid","token_type":"Bearer"})).unwrap();
        Self {
            client: KeycloakAdminClient {
                client: KeycloakAdmin::new(&url, token, reqwest::Client::new()),
            },
            worker,
        }
    }
}

// This is the only test in this integration-test binary, so environment changes
// cannot race another test. Restore them on exit, including on panic.
struct Environment(Vec<(&'static str, Option<std::ffi::OsString>)>);
impl Environment {
    fn set() -> Self {
        let mut previous = vec![];
        for (key, value) in [
            ("VOTING_PORTAL_URL", Some("https://vote.example.invalid")),
            (
                "BALLOT_VERIFIER_URL",
                Some("https://verify.example.invalid"),
            ),
            ("ADMIN_PORTAL_URL", Some("https://admin.example.invalid")),
            ("RESULTS_PORTAL_URL", None),
        ] {
            previous.push((key, std::env::var_os(key)));
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
        Self(previous)
    }
}
impl Drop for Environment {
    fn drop(&mut self) {
        for (key, value) in &self.0 {
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
    }
}

#[rocket::async_test]
async fn api_key_client_upsert_distinguishes_creation_update_and_lookup_failure(
) {
    let _environment = Environment::set();
    for name in ["tenant-test", "tenant-test-event-test"] {
        for (status, existing) in [
            (404, None),
            (200, None),
            (200, Some(true)),
            (200, Some(false)),
            (403, None),
        ] {
            for enabled in [Some(true), Some(false), None] {
                let peer = Peer::start(status, existing);
                let mut client = json!({"clientId":"api-key-client", "secret":"synthetic-generated"});
                if let Some(enabled) = enabled {
                    client["enabled"] = json!(enabled);
                }
                let body = json!({"clients":[client], "adminEventsEnabled":false, "adminEventsDetailsEnabled":true, "eventsEnabled":false}).to_string();
                let result = peer
                    .client
                    .upsert_realm(name, &body, "test", false, None, None)
                    .await;
                let writes = peer.worker.join().unwrap();
                if status == 403 {
                    let error = result.unwrap_err();
                    assert!(
                        matches!(
                            error.downcast_ref::<keycloak::KeycloakError>(),
                            Some(keycloak::KeycloakError::HttpFailure {
                                status: 403,
                                ..
                            })
                        ),
                        "{error:?}"
                    );
                    assert!(writes.is_empty());
                } else {
                    result.unwrap();
                    assert_eq!(writes.len(), 1);
                    if status == 404 {
                        assert_eq!(writes[0]["adminEventsEnabled"], true);
                        assert_eq!(
                            writes[0]["adminEventsDetailsEnabled"],
                            false
                        );
                    } else {
                        assert!(writes[0].get("adminEventsEnabled").is_none());
                        assert!(writes[0]
                            .get("adminEventsDetailsEnabled")
                            .is_none());
                    }
                    assert_eq!(writes[0]["eventsEnabled"], false);
                    let clients = writes[0]["clients"].as_array().unwrap();
                    if existing.is_some() {
                        assert!(
                            clients.is_empty(),
                            "existing configuration must never be sent back"
                        );
                    } else {
                        assert_eq!(clients[0]["enabled"], false);
                        assert_eq!(clients[0]["secret"], "synthetic-generated");
                    }
                }
            }
        }
    }
}
