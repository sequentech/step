// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! A bounded, loopback-only HTTP peer for testing the real Keycloak client.
//!
//! Responses are matched by method and path, so concurrent permission lookups
//! need not arrive in a particular order. Tests inspect captured requests to
//! verify payloads and authentication, rather than mocking client methods.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use keycloak::{KeycloakAdmin, KeycloakAdminToken};
use sequent_core::services::keycloak::{KeycloakAdminClient, PubKeycloakAdmin};
use serde_json::{json, Value};

pub struct Exchange {
    method: &'static str,
    path: String,
    status: u16,
    body: String,
    headers: Vec<(String, String)>,
}

impl Exchange {
    pub fn json(
        method: &'static str,
        path: &str,
        status: u16,
        body: Value,
    ) -> Self {
        Self {
            method,
            path: path.into(),
            status,
            body: body.to_string(),
            headers: vec![],
        }
    }

    pub fn header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }
}

#[derive(Debug)]
pub struct Request {
    pub method: String,
    pub url: reqwest::Url,
    pub headers: HashMap<String, String>,
    pub body: String,
}

impl Request {
    pub fn json(&self) -> Value {
        serde_json::from_str(&self.body).expect("request must contain JSON")
    }

    pub fn query(&self) -> HashMap<String, String> {
        self.url.query_pairs().into_owned().collect()
    }
}

pub struct HttpServer {
    pub url: String,
    requests: Arc<Mutex<Vec<Request>>>,
    pending: Arc<Mutex<Vec<Exchange>>>,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl HttpServer {
    pub fn start(exchanges: Vec<Exchange>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let pending = Arc::new(Mutex::new(exchanges));
        let requests = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let (peer_pending, peer_requests, peer_stop) =
            (pending.clone(), requests.clone(), stop.clone());
        let worker = thread::spawn(move || {
            while !peer_stop.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        // A broken client must fail quickly, never hang the suite.
                        stream
                            .set_read_timeout(Some(Duration::from_secs(3)))
                            .unwrap();
                        stream
                            .set_write_timeout(Some(Duration::from_secs(3)))
                            .unwrap();
                        let request = read_request(&stream);
                        let mut exchanges = peer_pending.lock().unwrap();
                        let matched = exchanges.iter().position(|exchange| {
                            exchange.method == request.method
                                && exchange.path == request.url.path()
                        });
                        let response = matched
                            .map(|index| exchanges.remove(index))
                            .unwrap_or_else(|| {
                                Exchange::json(
                                    "",
                                    "",
                                    500,
                                    json!({"error": "unexpected test request"}),
                                )
                            });
                        peer_requests.lock().unwrap().push(request);
                        write!(stream, "HTTP/1.1 {} Test response\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: {}\r\n", response.status, response.body.len()).unwrap();
                        for (name, value) in response.headers {
                            write!(stream, "{name}: {value}\r\n").unwrap();
                        }
                        write!(stream, "\r\n{}", response.body).unwrap();
                    }
                    Err(error)
                        if error.kind() == std::io::ErrorKind::WouldBlock =>
                    {
                        thread::sleep(Duration::from_millis(2));
                    }
                    Err(error) => panic!("local HTTP listener failed: {error}"),
                }
            }
        });
        Self {
            url,
            requests,
            pending,
            stop,
            worker: Some(worker),
        }
    }

    pub fn client(&self) -> KeycloakAdminClient {
        KeycloakAdminClient {
            client: KeycloakAdmin::new(&self.url, token(), http_client()),
        }
    }

    pub fn public_client(&self) -> PubKeycloakAdmin {
        PubKeycloakAdmin {
            url: self.url.clone(),
            client: http_client(),
            token_supplier: token(),
        }
    }

    /// Stop the peer, require every scripted exchange, and return wire evidence.
    pub fn finish(mut self) -> Vec<Request> {
        self.stop.store(true, Ordering::Relaxed);
        self.worker.take().unwrap().join().unwrap();
        assert!(
            self.pending.lock().unwrap().is_empty(),
            "expected HTTP requests were not made"
        );
        std::mem::take(&mut *self.requests.lock().unwrap())
    }
}

impl Drop for HttpServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(worker) = self.worker.take() {
            // Keep the test's original assertion when unwinding; finish() checks
            // thread failures on the successful path.
            let _ = worker.join();
        }
    }
}

fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap()
}

pub fn token_json() -> Value {
    json!({"access_token": "synthetic-access-token", "expires_in": 300,
        "scope": "openid", "token_type": "Bearer"})
}

fn token() -> KeycloakAdminToken {
    serde_json::from_value(token_json()).unwrap()
}

fn read_request(stream: &TcpStream) -> Request {
    let mut reader = BufReader::new(stream);
    let mut first = String::new();
    reader.read_line(&mut first).unwrap();
    let mut parts = first.split_whitespace();
    let method = parts.next().unwrap().into();
    let url = reqwest::Url::parse(&format!(
        "http://localhost{}",
        parts.next().unwrap()
    ))
    .unwrap();
    let mut headers = HashMap::new();
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        if line == "\r\n" {
            break;
        }
        let (name, value) = line.split_once(':').expect("valid HTTP header");
        headers.insert(name.to_lowercase(), value.trim().to_string());
    }
    let length = headers
        .get("content-length")
        .map(|value| value.parse().unwrap())
        .unwrap_or(0);
    assert!(length <= 1024 * 1024, "unexpectedly large test request");
    let mut body = vec![0; length];
    reader.read_exact(&mut body).unwrap();
    Request {
        method,
        url,
        headers,
        body: String::from_utf8(body).unwrap(),
    }
}
