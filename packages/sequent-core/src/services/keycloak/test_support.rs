// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::{KeycloakAdminClient, PubKeycloakAdminToken};
use keycloak::KeycloakAdmin;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread;

/// Local stand-in for the Keycloak admin API that records every request.
pub struct FakeKeycloak {
    url: String,
    requests: Arc<Mutex<Vec<String>>>,
}

impl FakeKeycloak {
    /// `stored_names` maps a role name as it appears in a request path to the
    /// name Keycloak stores for that role.
    pub fn start(stored_names: &[(&str, &str)]) -> Self {
        let stored_names: HashMap<String, String> = stored_names
            .iter()
            .map(|(requested, stored)| {
                (requested.to_string(), stored.to_string())
            })
            .collect();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let requests = Arc::new(Mutex::new(vec![]));
        let log = Arc::clone(&requests);
        thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut request_line = String::new();
                if reader.read_line(&mut request_line).unwrap_or(0) == 0 {
                    continue;
                }
                let mut length = 0;
                loop {
                    let mut line = String::new();
                    reader.read_line(&mut line).unwrap();
                    if line == "\r\n" {
                        break;
                    }
                    if let Some(value) =
                        line.to_lowercase().strip_prefix("content-length:")
                    {
                        length = value.trim().parse().unwrap();
                    }
                }
                reader.read_exact(&mut vec![0; length]).unwrap();
                let mut parts = request_line.split_whitespace();
                let (method, path) =
                    (parts.next().unwrap(), parts.next().unwrap());
                log.lock().unwrap().push(format!("{method} {path}"));
                let (status, body) = respond(method, path, &stored_names);
                write!(
                    stream,
                    "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                )
                .unwrap();
            }
        });
        FakeKeycloak { url, requests }
    }

    pub fn client(&self) -> KeycloakAdminClient {
        let token = PubKeycloakAdminToken {
            access_token: "token".into(),
            expires_in: 300,
            not_before_policy: None,
            refresh_expires_in: None,
            refresh_token: None,
            scope: "openid".into(),
            session_state: None,
            token_type: "Bearer".into(),
        };
        KeycloakAdminClient {
            client: KeycloakAdmin::new(
                &self.url,
                token.try_into().unwrap(),
                reqwest::Client::new(),
            ),
        }
    }

    pub fn requests(&self) -> Vec<String> {
        self.requests.lock().unwrap().clone()
    }
}

fn respond(
    method: &str,
    path: &str,
    stored_names: &HashMap<String, String>,
) -> (&'static str, String) {
    let segments: Vec<&str> = path.split('/').collect();
    match (method, &segments[..]) {
        ("GET", ["", "admin", "realms", _, "roles", requested]) => {
            let stored = stored_names
                .get(*requested)
                .map(String::as_str)
                .unwrap_or(requested);
            ("200 OK", format!(r#"{{"id":"role-id","name":"{stored}"}}"#))
        }
        ("POST", ["", "admin", "realms", _, "roles" | "groups"]) => {
            ("201 Created", String::new())
        }
        (
            "POST",
            ["", "admin", "realms", _, "groups", _, "role-mappings", "realm"],
        ) => ("204 No Content", String::new()),
        _ => ("404 Not Found", "{}".to_string()),
    }
}
