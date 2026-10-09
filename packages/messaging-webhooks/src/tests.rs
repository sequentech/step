// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The component against a loopback stand-in for harvest.

use super::*;
use messaging::test_server::{Recorded, Reply as Peer, TestServer};
use rocket::http::Header;
use rocket::local::asynchronous::Client;
use std::collections::BTreeSet;

const SECRET: &[u8] = b"synthetic-relay-secret";
const KEY: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn config(base_url: &str) -> Config {
    Config {
        callbacks_url: format!("{base_url}{CALLBACKS_PATH}"),
        relay_secret: SECRET.to_vec(),
        max_body_bytes: 1024,
        timeout: Duration::from_secs(5),
    }
}

async fn component(config: Config) -> Client {
    Client::tracked(build(config).expect("the component builds"))
        .await
        .expect("the component launches")
}

/// Harvest's HTTP 200 carrying `answer`.
fn harvest_answers(status: u16, content_type: &str, body: &str) -> Peer {
    Peer::Json(
        200,
        serde_json::to_value(RelayResponse {
            status,
            content_type: content_type.to_string(),
            body: body.to_string(),
        })
        .expect("a relay response"),
    )
}

fn envelope(recorded: &Recorded) -> CallbackEnvelope {
    serde_json::from_str(&recorded.body).expect("an envelope")
}

/// Status, content type and body of a response.
async fn answer(response: rocket::local::asynchronous::LocalResponse<'_>) -> (u16, String, String) {
    let status = response.status().code;
    let content_type = response
        .content_type()
        .map(|content_type| content_type.to_string())
        .unwrap_or_default();
    (
        status,
        content_type,
        response.into_string().await.unwrap_or_default(),
    )
}

#[rocket::async_test]
async fn only_the_listed_routes_exist_and_nothing_else_is_forwarded() {
    let harvest = TestServer::start(vec![]).await;
    let client = component(config(&harvest.base_url)).await;
    let routes: BTreeSet<_> = client
        .rocket()
        .routes()
        .map(|route| {
            (
                route.method.as_str().to_string(),
                route.uri.path().to_string(),
            )
        })
        .collect();
    let expected: BTreeSet<_> = [
        ("GET", "/webhooks/meta/<key>"),
        ("POST", "/webhooks/meta/<key>"),
        ("POST", "/webhooks/viber/<key>"),
        ("POST", "/webhooks/aws/<key>"),
        ("GET", "/healthz"),
    ]
    .into_iter()
    .map(|(method, path)| (method.to_string(), path.to_string()))
    .collect();
    assert_eq!(routes, expected);
    // The routes are the paths callback URLs are built with.
    for provider in [
        CallbackProvider::META,
        CallbackProvider::VIBER,
        CallbackProvider::AWS,
    ] {
        let path = provider.callback_url("", "<key>");
        assert!(routes
            .iter()
            .any(|(method, route)| method == "POST" && *route == path));
    }

    for (post, path) in [
        (true, CALLBACKS_PATH.to_string()),
        (true, format!("/webhooks/http/{KEY}")),
        (false, format!("/webhooks/viber/{KEY}")),
        (false, format!("/webhooks/aws/{KEY}")),
        (true, format!("/webhooks/meta/{KEY}/extra")),
        (true, "/".to_string()),
    ] {
        let response = if post {
            client.post(path.clone()).body("{}").dispatch().await
        } else {
            client.get(path.clone()).dispatch().await
        };
        assert_eq!(
            answer(response).await,
            (404, "text/plain; charset=utf-8".into(), String::new()),
            "{path}"
        );
    }
    assert!(harvest.requests().is_empty());
}

#[rocket::async_test]
async fn a_key_of_the_wrong_shape_is_not_forwarded() {
    let harvest = TestServer::start(vec![]).await;
    let client = component(config(&harvest.base_url)).await;
    for key in [
        KEY.to_uppercase(),
        KEY[..32].to_string(),
        format!("{KEY}0"),
        "..%2F..%2Fetc%2Fpasswd".to_string(),
    ] {
        let response = client
            .post(format!("/webhooks/meta/{key}"))
            .body("{}")
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::NotFound, "{key}");
        let response = client
            .get(format!("/webhooks/meta/{key}?hub.mode=subscribe"))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::NotFound, "{key}");
    }
    assert!(harvest.requests().is_empty());
}

#[rocket::async_test]
async fn a_body_over_the_limit_is_refused_unread() {
    let harvest = TestServer::start(vec![harvest_answers(200, "text/plain", "")]).await;
    let client = component(Config {
        max_body_bytes: 16,
        ..config(&harvest.base_url)
    })
    .await;
    for provider in ["meta", "viber", "aws"] {
        let response = client
            .post(format!("/webhooks/{provider}/{KEY}"))
            .body("x".repeat(17))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::PayloadTooLarge, "{provider}");
    }
    assert!(harvest.requests().is_empty());

    let response = client
        .post(format!("/webhooks/viber/{KEY}"))
        .body("x".repeat(16))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(
        envelope(&harvest.requests()[0]).body().unwrap(),
        b"x".repeat(16)
    );
}

#[rocket::async_test]
async fn only_allowlisted_headers_are_forwarded() {
    let harvest = TestServer::start(vec![harvest_answers(200, "text/plain", "")]).await;
    let client = component(config(&harvest.base_url)).await;
    let response = client
        .post(format!("/webhooks/viber/{KEY}"))
        .header(ContentType::JSON)
        .header(Header::new("Authorization", "Basic cmVwb3J0czpzZWNyZXQ="))
        .header(Header::new("Cookie", "session=1"))
        .header(Header::new("X-Forwarded-For", "203.0.113.7"))
        .header(Header::new("X-Hub-Signature-256", "sha256=00"))
        .header(Header::new("X-Amz-Sns-Message-Type", "Notification"))
        .header(Header::new(RELAY_SIGNATURE_HEADER, "t=1,v1=00"))
        .body("{}")
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);
    let forwarded = envelope(&harvest.requests()[0]);
    assert_eq!(
        forwarded
            .headers
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        vec![
            "authorization",
            "content-type",
            "x-amz-sns-message-type",
            "x-hub-signature-256"
        ]
    );
    assert_eq!(
        forwarded.header("authorization"),
        Some("Basic cmVwb3J0czpzZWNyZXQ=")
    );
    assert_eq!(forwarded.provider, CallbackProvider::VIBER);
}

#[rocket::async_test]
async fn the_envelope_carries_the_exact_bytes_under_a_valid_relay_signature() {
    let harvest = TestServer::start(vec![harvest_answers(200, "text/plain", "")]).await;
    let client = component(config(&harvest.base_url)).await;
    // Meta escapes slashes and non-ASCII characters; its signature covers
    // these exact bytes, and so must the relayed body.
    let body = "{\"object\":\"page\",\"entry\":[{\"id\":\"1\",\"text\":\"a\\/b \\u00f1 ñ 😀\"}]}\n";
    let response = client
        .post(format!("/webhooks/meta/{KEY}?ignored=1"))
        .header(ContentType::JSON)
        .header(Header::new("X-Hub-Signature-256", "sha256=abc"))
        .body(body)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);

    let requests = harvest.requests();
    assert_eq!(requests.len(), 1);
    let request = &requests[0];
    assert_eq!(
        (request.method.as_str(), request.path.as_str()),
        ("POST", CALLBACKS_PATH)
    );
    assert_eq!(request.header("content-type"), Some("application/json"));
    let now = Utc::now().timestamp();
    assert_eq!(
        relay::verify(
            SECRET,
            request.header(RELAY_SIGNATURE_HEADER),
            request.body.as_bytes(),
            now
        ),
        Ok(())
    );
    assert!(relay::verify(
        b"another secret",
        request.header(RELAY_SIGNATURE_HEADER),
        request.body.as_bytes(),
        now
    )
    .is_err());
    let forwarded = envelope(request);
    assert_eq!(forwarded.version, relay::ENVELOPE_VERSION);
    assert_eq!(forwarded.provider, CallbackProvider::META);
    assert_eq!(forwarded.method, CallbackMethod::POST);
    assert_eq!(forwarded.key, KEY);
    assert_eq!(forwarded.query, "ignored=1");
    assert_eq!(forwarded.body().unwrap(), body.as_bytes());
    assert_eq!(forwarded.header("x-hub-signature-256"), Some("sha256=abc"));
    assert!((now - forwarded.received_at.timestamp()).abs() < 60);
}

#[rocket::async_test]
async fn the_meta_handshake_query_goes_raw_and_the_challenge_comes_back_as_text() {
    let harvest = TestServer::start(vec![harvest_answers(
        200,
        "text/plain; charset=utf-8",
        "1158201444",
    )])
    .await;
    let client = component(config(&harvest.base_url)).await;
    let query = "hub.mode=subscribe&hub.verify_token=a%2Bb+c&hub.challenge=1158201444";
    let response = client
        .get(format!("/webhooks/meta/{KEY}?{query}"))
        .dispatch()
        .await;
    assert_eq!(
        answer(response).await,
        (200, "text/plain; charset=utf-8".into(), "1158201444".into())
    );
    let forwarded = envelope(&harvest.requests()[0]);
    assert_eq!(forwarded.method, CallbackMethod::GET);
    assert_eq!(forwarded.query, query);
    assert!(forwarded.body().unwrap().is_empty());
}

#[rocket::async_test]
async fn harvests_answer_reaches_the_provider_as_is() {
    let harvest = TestServer::start(vec![
        harvest_answers(401, "text/plain; charset=utf-8", ""),
        harvest_answers(404, "text/plain; charset=utf-8", ""),
        harvest_answers(200, "application/json", "{\"ok\":true}"),
        harvest_answers(200, "not a content type", "x"),
    ])
    .await;
    let client = component(config(&harvest.base_url)).await;
    let mut answers = vec![];
    for _ in 0..4 {
        let response = client
            .post(format!("/webhooks/aws/{KEY}"))
            .header(ContentType::Plain)
            .header(Header::new(
                "x-amz-sns-message-type",
                "SubscriptionConfirmation",
            ))
            .body("{\"Type\":\"SubscriptionConfirmation\"}")
            .dispatch()
            .await;
        answers.push(answer(response).await);
    }
    assert_eq!(
        answers,
        vec![
            (401, "text/plain; charset=utf-8".into(), String::new()),
            (404, "text/plain; charset=utf-8".into(), String::new()),
            (200, "application/json".into(), "{\"ok\":true}".into()),
            (200, "text/plain; charset=utf-8".into(), "x".into()),
        ]
    );
    // SNS posts text/plain; it is relayed like any other body.
    let forwarded = envelope(&harvest.requests()[0]);
    assert_eq!(forwarded.provider, CallbackProvider::AWS);
    assert_eq!(
        forwarded.header("content-type"),
        Some("text/plain; charset=utf-8")
    );
    assert_eq!(
        forwarded.header("x-amz-sns-message-type"),
        Some("SubscriptionConfirmation")
    );
}

#[rocket::async_test]
async fn without_a_usable_answer_from_harvest_the_provider_gets_503() {
    let harvest = TestServer::start(vec![
        // Harvest refused the relay signature, or failed.
        Peer::Json(401, serde_json::json!({"message": "Unauthorized"})),
        Peer::Json(500, serde_json::json!({})),
        // An HTTP 200 that is not a relay response.
        Peer::Raw(200, "text/html", "<html></html>".into()),
        harvest_answers(42, "text/plain", ""),
        // The connection closes without an answer.
        Peer::Drop,
    ])
    .await;
    let client = component(config(&harvest.base_url)).await;
    for attempt in 0..5 {
        let response = client
            .post(format!("/webhooks/meta/{KEY}"))
            .body("{}")
            .dispatch()
            .await;
        assert_eq!(
            answer(response).await,
            (503, "text/plain; charset=utf-8".into(), String::new()),
            "{attempt}"
        );
    }
    assert_eq!(harvest.requests().len(), 5);

    // Nothing listens.
    let client = component(config("http://127.0.0.1:1")).await;
    let response = client
        .post(format!("/webhooks/viber/{KEY}"))
        .body("{}")
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::ServiceUnavailable);
}

#[rocket::async_test]
async fn a_harvest_slower_than_the_timeout_gives_503() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base_url = format!("http://{}", listener.local_addr().unwrap());
    let held = tokio::spawn(async move {
        // Accept and never answer.
        let (stream, _) = listener.accept().await.unwrap();
        tokio::time::sleep(Duration::from_secs(5)).await;
        drop(stream);
    });
    let client = component(Config {
        timeout: Duration::from_millis(200),
        ..config(&base_url)
    })
    .await;
    let started = Instant::now();
    let response = client
        .post(format!("/webhooks/aws/{KEY}"))
        .body("{}")
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::ServiceUnavailable);
    assert!(started.elapsed() < Duration::from_secs(4));
    held.abort();
}

#[rocket::async_test]
async fn liveness_never_forwards() {
    let harvest = TestServer::start(vec![]).await;
    let client = component(config(&harvest.base_url)).await;
    assert_eq!(client.get("/healthz").dispatch().await.status(), Status::Ok);
    assert!(harvest.requests().is_empty());
}

#[test]
fn configuration_comes_from_the_environment() {
    let vars = |values: &'static [(&'static str, &'static str)]| {
        move |name: &str| {
            values
                .iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| value.to_string())
        }
    };
    let config = Config::from_vars(vars(&[
        (HARVEST_DOMAIN_ENV, "harvest:8400"),
        (RELAY_SECRET_ENV, "s3cret"),
    ]))
    .unwrap();
    assert_eq!(
        config.callbacks_url,
        "http://harvest:8400/messaging/callbacks"
    );
    assert_eq!(config.relay_secret, b"s3cret");
    assert_eq!(config.max_body_bytes, relay::DEFAULT_MAX_BODY_BYTES);
    assert_eq!(config.timeout, HARVEST_TIMEOUT);
    assert!(!format!("{config:?}").contains("s3cret"));

    let config = Config::from_vars(vars(&[
        (HARVEST_DOMAIN_ENV, "https://harvest.internal/"),
        (RELAY_SECRET_ENV, "s3cret"),
        (relay::MAX_BODY_BYTES_ENV, "4096"),
    ]))
    .unwrap();
    assert_eq!(
        config.callbacks_url,
        "https://harvest.internal/messaging/callbacks"
    );
    assert_eq!(config.max_body_bytes, 4096);

    for (values, reason) in [
        (
            &[(RELAY_SECRET_ENV, "s3cret")][..],
            "HARVEST_DOMAIN is not set",
        ),
        (
            &[(HARVEST_DOMAIN_ENV, "harvest:8400")][..],
            "MESSAGING_RELAY_SECRET is not set",
        ),
        (
            &[
                (HARVEST_DOMAIN_ENV, "harvest:8400"),
                (RELAY_SECRET_ENV, "  "),
            ][..],
            "MESSAGING_RELAY_SECRET is not set",
        ),
        (
            &[
                (HARVEST_DOMAIN_ENV, "harvest:8400"),
                (RELAY_SECRET_ENV, "s3cret"),
                (relay::MAX_BODY_BYTES_ENV, "lots"),
            ][..],
            "MESSAGING_WEBHOOKS_MAX_BODY_BYTES must be a positive number of bytes",
        ),
    ] {
        let error = Config::from_vars(vars(values)).unwrap_err();
        assert_eq!(error.to_string(), reason);
    }
}

#[test]
fn reasons_for_503_name_what_went_wrong() {
    assert_eq!(Unavailable::Unreachable.to_string(), "Harvest unreachable");
    assert_eq!(Unavailable::TimedOut.to_string(), "Harvest timed out");
    assert_eq!(
        Unavailable::Refused(401).to_string(),
        "Harvest refused the callback (HTTP 401)"
    );
    assert_eq!(
        Unavailable::BadAnswer.to_string(),
        "Harvest's answer is not a relay response"
    );
}
