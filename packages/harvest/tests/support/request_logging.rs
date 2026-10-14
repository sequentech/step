// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use rocket::{http::Status, local::asynchronous::Client};
use std::sync::{Arc, Mutex};
use tracing::instrument::WithSubscriber;
use tracing_subscriber::{layer::SubscriberExt, Layer};

#[derive(Clone, Default)]
struct Events(Arc<Mutex<Vec<String>>>);

impl<S: tracing::Subscriber> Layer<S> for Events {
    fn on_new_span(
        &self,
        attributes: &tracing::span::Attributes<'_>,
        _: &tracing::span::Id,
        _: tracing_subscriber::layer::Context<'_, S>,
    ) {
        self.0
            .lock()
            .unwrap()
            .push(format!("span:{}", attributes.metadata().name()));
    }

    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _: tracing_subscriber::layer::Context<'_, S>,
    ) {
        struct Message<'a>(&'a mut String);
        impl tracing::field::Visit for Message<'_> {
            fn record_debug(
                &mut self,
                field: &tracing::field::Field,
                value: &dyn std::fmt::Debug,
            ) {
                if field.name() == "message" {
                    *self.0 = format!("{value:?}");
                }
            }
        }
        let mut message = String::new();
        event.record(&mut Message(&mut message));
        self.0.lock().unwrap().push(message);
    }
}

#[get("/trustee/boards?<fail>")]
async fn check(
    fail: Option<bool>,
    barrier: &rocket::State<tokio::sync::Barrier>,
) -> Status {
    if fail == Some(true) {
        return Status::Forbidden;
    }
    barrier.wait().await;
    tokio::task::yield_now().await;
    shared_lookup();
    tracing::warn!("check warning");
    tracing::error!("check error");
    Status::Ok
}

#[get("/other")]
async fn other(barrier: &rocket::State<tokio::sync::Barrier>) -> Status {
    barrier.wait().await;
    shared_lookup();
    Status::Ok
}

#[post("/trustee/boards/report")]
async fn report(barrier: &rocket::State<tokio::sync::Barrier>) -> Status {
    barrier.wait().await;
    shared_lookup();
    Status::Ok
}

#[tracing::instrument]
fn shared_lookup() {
    tracing::info!("routine lookup");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn only_trustee_checks_require_debug_logging() {
    const TEST: &str = "services::request_logging::tests::only_trustee_checks_require_debug_logging";
    if !crate::request_boundaries::is_isolated_child() {
        crate::request_boundaries::run_isolated(TEST, "http://127.0.0.1:9");
        return;
    }
    // Install the same log-to-tracing bridge production uses, before Rocket
    // tries installing its own logger. Each request has its own Tokio task.
    tracing_log::LogTracer::init().unwrap();
    let client = Arc::new(
        Client::tracked(
            rocket::build()
                .attach(fairing())
                .manage(tokio::sync::Barrier::new(3))
                .mount("/", routes![check, other, report]),
        )
        .await
        .unwrap(),
    );

    for (level, check_visible) in
        [(LevelFilter::INFO, false), (LevelFilter::DEBUG, true)]
    {
        let logs = Events::default();
        let subscriber = tracing_subscriber::registry().with(level).with(
            logs.clone().with_filter(
                tracing_subscriber::filter::dynamic_filter_fn(
                    move |metadata, _| enabled(metadata, level),
                ),
            ),
        );
        let dispatch = tracing::Dispatch::new(subscriber);
        let mut requests = Vec::new();
        for (path, post) in [
            ("/trustee/boards?refresh=1", false),
            ("/other", false),
            ("/trustee/boards/report", true),
        ] {
            let client = client.clone();
            requests.push(tokio::spawn(
                async move {
                    let request = if post {
                        client.post(path)
                    } else {
                        client.get(path)
                    };
                    let response = request.dispatch().await;
                    assert_eq!(response.status(), Status::Ok);
                    drop(response);
                    tracing::info!("after request");
                }
                .with_subscriber(dispatch.clone()),
            ));
        }
        for request in requests {
            tokio::time::timeout(std::time::Duration::from_secs(3), request)
                .await
                .unwrap()
                .unwrap();
        }
        let failed_client = client.clone();
        tokio::spawn(
            async move {
                assert_eq!(
                    failed_client
                        .get("/trustee/boards?fail=true")
                        .dispatch()
                        .await
                        .status(),
                    Status::Forbidden
                );
            }
            .with_subscriber(dispatch),
        )
        .await
        .unwrap();
        let messages = logs.0.lock().unwrap();
        assert_eq!(
            messages
                .iter()
                .filter(|message| message.as_str() == "span:shared_lookup")
                .count(),
            if check_visible { 3 } else { 2 },
            "{messages:?}"
        );
        assert_eq!(
            messages
                .iter()
                .filter(|message| message.as_str() == "after request")
                .count(),
            3,
            "{messages:?}"
        );
        assert!(messages.iter().any(|message| message.as_str() == "trustee board check failed"), "{messages:?}");
        assert!(CHECKS.lock().unwrap().is_empty());
        assert_eq!(
            messages
                .iter()
                .filter(|message| message.contains("routine lookup"))
                .count(),
            if check_visible { 3 } else { 2 },
            "{messages:?}"
        );
        assert_eq!(
            messages
                .iter()
                .any(|message| message
                    .starts_with("GET /trustee/boards?refresh=")),
            check_visible,
            "{messages:?}"
        );
        assert!(
            messages
                .iter()
                .any(|message| message.starts_with("GET /other:")),
            "{messages:?}"
        );
        assert!(
            messages
                .iter()
                .any(|message| message
                    .starts_with("POST /trustee/boards/report:")),
            "{messages:?}"
        );
        assert!(
            messages.iter().any(|message| message == "check warning"),
            "{messages:?}"
        );
        assert!(
            messages.iter().any(|message| message == "check error"),
            "{messages:?}"
        );
    }
}
