// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use super::PubKeycloakAdminToken;
use std::collections::HashMap;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tracing_subscriber::fmt::format::FmtSpan;

pub const SECRET: &str = "synthetic-client-secret-never-log-8129";
pub const ACCESS: &str = "synthetic-access-token-never-log-8129";
pub const REFRESH: &str = "synthetic-refresh-token-never-log-8129";

pub fn token() -> PubKeycloakAdminToken {
    PubKeycloakAdminToken {
        access_token: ACCESS.into(),
        expires_in: 300,
        not_before_policy: None,
        refresh_expires_in: Some(600),
        refresh_token: Some(REFRESH.into()),
        scope: "openid".into(),
        session_state: None,
        token_type: "Bearer".into(),
    }
}

pub struct ObservedRequest {
    pub request_line: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

pub struct HttpServer {
    pub url: String,
    pub request: tokio::task::JoinHandle<ObservedRequest>,
}

// Actual local HTTP transport with synthetic status/body responses. This is not
// a Keycloak service and does not enforce realm permissions or token validity.
pub async fn server(status: &str, body: &str) -> HttpServer {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let request = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut input = BufReader::new(&mut stream);
        let mut request_line = String::new();
        input.read_line(&mut request_line).await.unwrap();
        let mut headers = HashMap::new();
        loop {
            let mut line = String::new();
            assert_ne!(input.read_line(&mut line).await.unwrap(), 0);
            if line == "\r\n" {
                break;
            }
            let (name, value) = line.trim_end().split_once(':').unwrap();
            headers.insert(name.to_ascii_lowercase(), value.trim().to_owned());
        }
        let length = headers
            .get("content-length")
            .unwrap()
            .parse::<usize>()
            .unwrap();
        assert!(length < 65536);
        let mut body = vec![0; length];
        input.read_exact(&mut body).await.unwrap();
        stream.write_all(response.as_bytes()).await.unwrap();
        ObservedRequest {
            request_line,
            headers,
            body,
        }
    });
    HttpServer { url, request }
}

#[derive(Clone, Default)]
pub struct LogCapture(Arc<Mutex<Vec<u8>>>);

impl Write for LogCapture {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl LogCapture {
    pub fn subscriber(&self) -> impl tracing::Subscriber + Send + Sync {
        let writer = self.clone();
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::TRACE)
            .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
            .without_time()
            .with_ansi(false)
            .with_writer(move || writer.clone())
            .finish()
    }

    pub fn output(&self) -> String {
        String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
    }
}
