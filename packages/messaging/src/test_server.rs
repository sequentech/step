// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! A loopback HTTP peer for adapter tests: it answers queued responses in
//! order and records each request.
//!
//! [`TestServer::start`] serves one connection at a time and closes it
//! after each answer, which keeps adapter tests deterministic.
//! [`TestServer::start_with`] can also act as a provider under load, for
//! the OTP capacity bench: a latency before each answer, a reply repeated
//! once the queue runs out, concurrent kept-alive connections, and counters
//! instead of recorded requests.

use std::collections::VecDeque;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

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

/// How the server behaves beyond its queued replies.
#[derive(Debug, Clone, Default)]
pub struct ServerOptions {
    /// Waited before each answer, as the provider's own processing time.
    pub latency: Duration,
    /// Answered once the queued replies run out. `None`: the connection is
    /// dropped, as [`Reply::Drop`].
    pub then: Option<Reply>,
    /// Serve connections at the same time and keep each open for further
    /// requests, as a provider does. Otherwise one connection at a time,
    /// closed after its answer.
    pub concurrent: bool,
    /// Count requests without keeping them, for long runs.
    pub count_only: bool,
}

/// The JSON a successful send gets from WhatsApp Cloud API, Messenger Send
/// API and Infobip at once: each adapter reads its own fields.
pub fn accepted_by_any_provider(message_id: &str) -> serde_json::Value {
    serde_json::json!({
        "messages": [{
            "id": message_id,
            "messageId": message_id,
            "status": {"groupName": "PENDING"},
        }],
        "recipient_id": "loopback",
        "message_id": message_id,
    })
}

struct Shared {
    replies: Mutex<VecDeque<Reply>>,
    requests: Mutex<Vec<Recorded>>,
    options: ServerOptions,
    served: AtomicU64,
    in_flight: AtomicU64,
    peak_in_flight: AtomicU64,
}

pub struct TestServer {
    pub base_url: String,
    shared: Arc<Shared>,
}

impl TestServer {
    pub async fn start(replies: Vec<Reply>) -> TestServer {
        TestServer::start_with(replies, ServerOptions::default()).await
    }

    pub async fn start_with(replies: Vec<Reply>, options: ServerOptions) -> TestServer {
        let address: SocketAddr = ([127, 0, 0, 1], 0).into();
        TestServer::bind(address, replies, options).await
    }

    /// Serves on `address`, such as a fixed port for another process.
    pub async fn bind(
        address: SocketAddr,
        replies: Vec<Reply>,
        options: ServerOptions,
    ) -> TestServer {
        let listener = TcpListener::bind(address).await.expect("bind");
        let base_url = format!("http://{}", listener.local_addr().expect("address"));
        let shared = Arc::new(Shared {
            replies: Mutex::new(VecDeque::from(replies)),
            requests: Mutex::new(Vec::new()),
            options,
            served: AtomicU64::new(0),
            in_flight: AtomicU64::new(0),
            peak_in_flight: AtomicU64::new(0),
        });
        let server = shared.clone();
        tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                if server.options.concurrent {
                    tokio::spawn(serve(stream, server.clone()));
                } else {
                    serve(stream, server.clone()).await;
                }
            }
        });
        TestServer { base_url, shared }
    }

    pub fn requests(&self) -> Vec<Recorded> {
        self.shared.requests.lock().expect("requests").clone()
    }

    /// Requests read so far, answered or not.
    pub fn served(&self) -> u64 {
        self.shared.served.load(Ordering::SeqCst)
    }

    /// The most requests waiting for their answer at the same time.
    pub fn peak_in_flight(&self) -> u64 {
        self.shared.peak_in_flight.load(Ordering::SeqCst)
    }
}

/// One request read from `stream`: head and body. `None` at the end of the
/// connection.
async fn read_request(stream: &mut TcpStream) -> Option<(String, Vec<u8>)> {
    let mut buffer = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        let read = stream.read(&mut chunk).await.unwrap_or(0);
        if read == 0 {
            return None;
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
            return Some((head, body));
        }
    }
}

async fn serve(mut stream: TcpStream, shared: Arc<Shared>) {
    let keep_alive = shared.options.concurrent;
    loop {
        let (head, body) = read_request(&mut stream)
            .await
            .unwrap_or_else(|| (String::new(), Vec::new()));
        if head.is_empty() && keep_alive {
            // The client closed a kept-alive connection.
            break;
        }
        shared.served.fetch_add(1, Ordering::SeqCst);
        let in_flight = shared.in_flight.fetch_add(1, Ordering::SeqCst) + 1;
        shared.peak_in_flight.fetch_max(in_flight, Ordering::SeqCst);
        if !shared.options.count_only {
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
            shared.requests.lock().expect("requests").push(Recorded {
                method,
                path,
                headers,
                body: String::from_utf8_lossy(&body).to_string(),
            });
        }
        let reply = shared
            .replies
            .lock()
            .expect("replies")
            .pop_front()
            .or_else(|| shared.options.then.clone());
        if !shared.options.latency.is_zero() {
            tokio::time::sleep(shared.options.latency).await;
        }
        let answer = match reply {
            Some(Reply::Json(status, value)) => {
                Some((status, "application/json", value.to_string()))
            }
            Some(Reply::Raw(status, content_type, body)) => Some((status, content_type, body)),
            Some(Reply::Drop) | None => None,
        };
        let answered = match answer {
            Some((status, content_type, body)) => {
                let connection = if keep_alive { "keep-alive" } else { "close" };
                let response = format!(
                    "HTTP/1.1 {status} X\r\ncontent-type: {content_type}\r\ncontent-length: {}\r\nconnection: {connection}\r\n\r\n{body}",
                    body.len()
                );
                stream.write_all(response.as_bytes()).await.is_ok()
            }
            None => false,
        };
        shared.in_flight.fetch_sub(1, Ordering::SeqCst);
        if !(keep_alive && answered) {
            break;
        }
    }
    let _ = stream.shutdown().await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[tokio::test]
    async fn a_loaded_provider_answers_concurrently_after_its_latency() {
        let server = TestServer::start_with(
            vec![],
            ServerOptions {
                latency: Duration::from_millis(200),
                then: Some(Reply::Json(200, accepted_by_any_provider("loop-1"))),
                concurrent: true,
                count_only: true,
            },
        )
        .await;
        let client = reqwest::Client::new();
        let started = Instant::now();
        let sends = (0..8).map(|_| {
            let client = client.clone();
            let url = format!("{}/send", server.base_url);
            async move { client.post(url).body("{}").send().await.unwrap().status() }
        });
        let statuses = futures_join_all(sends).await;
        let elapsed = started.elapsed();
        assert!(statuses.iter().all(|status| status.as_u16() == 200));
        assert!(elapsed >= Duration::from_millis(200), "{elapsed:?}");
        // Eight sends of 200 ms each, at the same time: well under 8 × 200.
        assert!(elapsed < Duration::from_millis(1200), "{elapsed:?}");
        assert_eq!(server.served(), 8);
        assert!(server.peak_in_flight() > 1);
        assert!(server.requests().is_empty());
    }

    async fn futures_join_all<F: std::future::Future + Send + 'static>(
        futures: impl Iterator<Item = F>,
    ) -> Vec<F::Output>
    where
        F::Output: Send + 'static,
    {
        let handles: Vec<_> = futures.map(tokio::spawn).collect();
        let mut outputs = Vec::new();
        for handle in handles {
            outputs.push(handle.await.unwrap());
        }
        outputs
    }
}
