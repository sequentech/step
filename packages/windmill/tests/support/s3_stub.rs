// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! A loopback S3 for tests: answers every GET with one body and records the
//! request lines, so code that downloads a document can run without storage.
//! Include it with `#[path = "support/s3_stub.rs"] mod s3_stub;`.

use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// A loopback S3 that answers every GET with `body`, recording the paths asked.
pub async fn serve(body: impl Into<Vec<u8>>) -> (String, Arc<Mutex<Vec<String>>>) {
    let body: Arc<Vec<u8>> = Arc::new(body.into());
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let asked = Arc::new(Mutex::new(Vec::new()));
    let seen = asked.clone();
    tokio::spawn(async move {
        loop {
            let Ok((mut socket, _)) = listener.accept().await else {
                return;
            };
            let seen = seen.clone();
            let body = body.clone();
            tokio::spawn(async move {
                let mut request = Vec::new();
                let mut buffer = [0u8; 4096];
                while !request.windows(4).any(|w| w == b"\r\n\r\n") {
                    match socket.read(&mut buffer).await {
                        Ok(0) | Err(_) => return,
                        Ok(n) => request.extend_from_slice(&buffer[..n]),
                    }
                }
                let line = String::from_utf8_lossy(&request)
                    .lines()
                    .next()
                    .unwrap_or_default()
                    .to_owned();
                seen.lock().unwrap().push(line);
                let head = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let _ = socket.write_all(head.as_bytes()).await;
                let _ = socket.write_all(&body).await;
                let _ = socket.shutdown().await;
            });
        }
    });
    (format!("http://{address}"), asked)
}
