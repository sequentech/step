// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! A loopback HTTP peer for adapter tests: it answers queued responses in
//! order and records each request.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[derive(Debug, Clone)]
pub struct Recorded {
    pub method: String,
    pub path: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

impl Recorded {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }

    pub fn json(&self) -> serde_json::Value {
        serde_json::from_str(&self.body).expect("JSON request body")
    }
}

#[derive(Debug, Clone)]
pub enum Reply {
    Json(u16, serde_json::Value),
    /// Status, content type and body.
    Raw(u16, &'static str, String),
    /// Close the connection without answering, as a lost response would.
    Drop,
}

pub struct TestServer {
    pub base_url: String,
    requests: Arc<Mutex<Vec<Recorded>>>,
}

impl TestServer {
    pub async fn start(replies: Vec<Reply>) -> TestServer {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let base_url = format!("http://{}", listener.local_addr().expect("address"));
        let requests = Arc::new(Mutex::new(Vec::new()));
        let recorded = requests.clone();
        let replies = Arc::new(Mutex::new(VecDeque::from(replies)));
        tokio::spawn(async move {
            while let Ok((mut stream, _)) = listener.accept().await {
                let mut buffer = Vec::new();
                let mut chunk = [0u8; 4096];
                let (head, body) = loop {
                    let read = stream.read(&mut chunk).await.unwrap_or(0);
                    if read == 0 {
                        break (String::new(), Vec::new());
                    }
                    buffer.extend_from_slice(&chunk[..read]);
                    if let Some(end) = buffer.windows(4).position(|w| w == b"\r\n\r\n") {
                        let head = String::from_utf8_lossy(&buffer[..end]).to_string();
                        let length = head
                            .lines()
                            .find_map(|line| {
                                let (name, value) = line.split_once(':')?;
                                name.eq_ignore_ascii_case("content-length")
                                    .then(|| value.trim().parse::<usize>().ok())
                                    .flatten()
                            })
                            .unwrap_or(0);
                        let mut body = buffer[end + 4..].to_vec();
                        while body.len() < length {
                            let read = stream.read(&mut chunk).await.unwrap_or(0);
                            if read == 0 {
                                break;
                            }
                            body.extend_from_slice(&chunk[..read]);
                        }
                        break (head, body);
                    }
                };
                let mut lines = head.lines();
                let mut request_line = lines.next().unwrap_or_default().split_whitespace();
                let method = request_line.next().unwrap_or_default().to_string();
                let path = request_line.next().unwrap_or_default().to_string();
                let headers = lines
                    .filter_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        Some((name.trim().to_string(), value.trim().to_string()))
                    })
                    .collect();
                recorded.lock().expect("requests").push(Recorded {
                    method,
                    path,
                    headers,
                    body: String::from_utf8_lossy(&body).to_string(),
                });
                let reply = replies.lock().expect("replies").pop_front();
                let answer = match reply {
                    Some(Reply::Json(status, value)) => {
                        Some((status, "application/json", value.to_string()))
                    }
                    Some(Reply::Raw(status, content_type, body)) => {
                        Some((status, content_type, body))
                    }
                    Some(Reply::Drop) | None => None,
                };
                if let Some((status, content_type, body)) = answer {
                    let response = format!(
                        "HTTP/1.1 {status} X\r\ncontent-type: {content_type}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                        body.len()
                    );
                    let _ = stream.write_all(response.as_bytes()).await;
                }
                let _ = stream.shutdown().await;
            }
        });
        TestServer { base_url, requests }
    }

    pub fn requests(&self) -> Vec<Recorded> {
        self.requests.lock().expect("requests").clone()
    }
}
