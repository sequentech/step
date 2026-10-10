// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
#![cfg(feature = "keycloak")]

use rocket::http::{Header, Status};
use rocket::local::asynchronous::Client;
use sequent_core::services::connection::{
    DatafixClaims, LastDatafixAccessToken,
};
use sequent_core::services::keycloak::get_third_party_client_access_token;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex, Once};
use tracing::instrument::WithSubscriber;
use tracing_subscriber::fmt::format::FmtSpan;

const CLIENT_ID: &str = "datafix-client";
const CLIENT_SECRET: &str = "ClientSecretValue6d1e";
const ACCESS_TOKEN: &str = "AccessTokenValue3b7a";

/// Points KEYCLOAK_URL at a local server that answers every request with a
/// token response whose `expires_in` has the wrong type, so parsing fails on a
/// body that still carries a token.
fn start_keycloak_stub() {
    static STARTED: Once = Once::new();
    STARTED.call_once(|| {
        let listener =
            TcpListener::bind("127.0.0.1:0").expect("bind keycloak stub");
        let address = listener.local_addr().expect("keycloak stub address");
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let _ = answer(stream);
            }
        });
        std::env::set_var("KEYCLOAK_URL", format!("http://{address}"));
    });
}

fn answer(mut stream: TcpStream) -> std::io::Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut content_length = 0;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 || line == "\r\n" {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            if name.eq_ignore_ascii_case("content-length") {
                content_length = value.trim().parse().unwrap_or(0);
            }
        }
    }
    let mut body = vec![0; content_length];
    reader.read_exact(&mut body)?;

    let response =
        format!(r#"{{"access_token":"{ACCESS_TOKEN}","expires_in":"soon"}}"#);
    write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n{response}",
        response.len()
    )?;
    stream.flush()
}

#[derive(Clone, Default)]
struct LogBuffer(Arc<Mutex<Vec<u8>>>);

impl Write for LogBuffer {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if let Ok(mut buffer) = self.0.lock() {
            buffer.extend_from_slice(bytes);
        }
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl LogBuffer {
    fn contents(&self) -> String {
        self.0
            .lock()
            .map(|buffer| String::from_utf8_lossy(&buffer).into_owned())
            .unwrap_or_default()
    }
}

async fn with_captured_logs<T>(
    run: impl std::future::Future<Output = T>,
) -> (T, String) {
    let logs = LogBuffer::default();
    let writer = logs.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_ansi(false)
        .with_max_level(tracing::Level::TRACE)
        .with_span_events(FmtSpan::NEW)
        .with_writer(move || writer.clone())
        .finish();
    let output = run.with_subscriber(subscriber).await;
    (output, logs.contents())
}

#[rocket::get("/datafix")]
fn datafix_route(_claims: DatafixClaims) {}

#[rocket::async_test]
async fn third_party_token_request_keeps_credentials_out_of_logs_and_errors() {
    start_keycloak_stub();

    let (result, logs) =
        with_captured_logs(get_third_party_client_access_token(
            CLIENT_ID.to_string(),
            CLIENT_SECRET.to_string(),
            "tenant".to_string(),
        ))
        .await;

    let error = result.expect_err("malformed token response");
    for text in [error.to_string(), format!("{error:?}")] {
        assert!(!text.contains(ACCESS_TOKEN), "token in error: {text}");
        assert!(!text.contains(CLIENT_SECRET), "secret in error: {text}");
    }
    assert!(logs.contains("get_credentials_inner"), "{logs}");
    for value in [CLIENT_SECRET, ACCESS_TOKEN] {
        assert!(!logs.contains(value), "{value} in logs: {logs}");
    }
}

#[rocket::async_test]
async fn datafix_guard_keeps_client_secret_out_of_logs() {
    start_keycloak_stub();
    let client = Client::untracked(
        rocket::build()
            .manage(LastDatafixAccessToken::init())
            .mount("/", rocket::routes![datafix_route]),
    )
    .await
    .expect("rocket client");

    let (status, logs) = with_captured_logs(async {
        client
            .get("/datafix")
            .header(Header::new("tenant-id", "tenant"))
            .header(Header::new("event-id", "event"))
            .header(Header::new(
                "authorization",
                format!("{CLIENT_ID}:{CLIENT_SECRET}"),
            ))
            .dispatch()
            .await
            .status()
    })
    .await;

    assert_eq!(status, Status::Unauthorized);
    assert!(logs.contains("request_access_token"), "{logs}");
    for value in [CLIENT_SECRET, ACCESS_TOKEN] {
        assert!(!logs.contains(value), "{value} in logs: {logs}");
    }
}
