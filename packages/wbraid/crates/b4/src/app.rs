// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The Axum application — routes and CORS over an [`AppState`] — shared by the
//! `b4v6` binary and the handler tests.

use axum::{
    routing::{get, post},
    Router,
};
use tower_http::cors::{Any, CorsLayer};

use crate::{handlers, state::AppState};

pub fn router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        // Board management
        .route("/boards", post(handlers::create_board))
        .route("/boards", get(handlers::list_boards))
        .route("/boards/:board", get(handlers::get_board))
        // Message operations (board-specific)
        .route("/boards/:board/messages/list", get(handlers::list_messages))
        .route("/boards/:board/messages", get(handlers::get_messages))
        .route("/boards/:board/messages/:id", get(handlers::get_message))
        // POST - S3 two-step flow
        .route(
            "/boards/:board/messages/initiate",
            post(handlers::initiate_message),
        )
        .route(
            "/boards/:board/messages/:id/confirm",
            post(handlers::confirm_message),
        )
        .layer(cors)
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use aws_sdk_s3::config::{BehaviorVersion, Credentials, Region};
    use aws_sdk_s3::Client as S3Client;
    use axum::body::{to_bytes, Body};
    use axum::http::{header, Request, StatusCode};
    use serde::de::DeserializeOwned;
    use serde_json::json;
    use tower::ServiceExt;
    use uuid::Uuid;

    use super::*;
    use crate::api_types::{
        BoardResponse, ContentType, GetBlobsResponse, InitiateMessageResponse, ListBlobsResponse,
    };
    use crate::db;

    const BUCKET: &str = "test-bucket";

    /// An S3 client that can presign — a local computation — with no S3 to talk
    /// to: static credentials, a fixed region, a LocalStack-shaped endpoint.
    fn offline_s3_client() -> S3Client {
        let conf = aws_sdk_s3::Config::builder()
            .behavior_version(BehaviorVersion::latest())
            .region(Region::new("us-east-1"))
            .credentials_provider(Credentials::new("test", "test", None, None, "test"))
            .endpoint_url("http://localhost:4566")
            .force_path_style(true)
            .build();
        S3Client::from_conf(conf)
    }

    /// The router over a fresh database file, removed again on drop.
    struct TestApp {
        app: Router,
        db_path: PathBuf,
    }

    impl Drop for TestApp {
        fn drop(&mut self) {
            for suffix in ["", "-wal", "-shm"] {
                let _ = std::fs::remove_file(format!("{}{}", self.db_path.display(), suffix));
            }
        }
    }

    async fn test_app() -> TestApp {
        let db_path = std::env::temp_dir().join(format!("b4-app-test-{}.db", Uuid::new_v4()));
        let db = db::BoardDb::open_url(&format!("sqlite:{}?mode=rwc", db_path.display()))
            .await
            .unwrap();
        let state = AppState::new(db, offline_s3_client(), BUCKET.to_string());
        TestApp {
            app: router(state),
            db_path,
        }
    }

    async fn call(
        app: &Router,
        method: &str,
        uri: &str,
        body: Option<serde_json::Value>,
    ) -> (StatusCode, Vec<u8>) {
        let mut request = Request::builder().method(method).uri(uri);
        let body = match body {
            Some(json) => {
                request = request.header(header::CONTENT_TYPE, "application/json");
                Body::from(json.to_string())
            }
            None => Body::empty(),
        };
        let response = app
            .clone()
            .oneshot(request.body(body).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        (status, bytes.to_vec())
    }

    async fn call_ok<T: DeserializeOwned>(
        app: &Router,
        method: &str,
        uri: &str,
        body: Option<serde_json::Value>,
    ) -> T {
        let (status, bytes) = call(app, method, uri, body).await;
        assert_eq!(
            status,
            StatusCode::OK,
            "{method} {uri}: {}",
            String::from_utf8_lossy(&bytes)
        );
        serde_json::from_slice(&bytes).unwrap()
    }

    async fn create_board(app: &Router, name: &str) -> BoardResponse {
        call_ok(app, "POST", "/boards", Some(json!({ "name": name }))).await
    }

    async fn initiate(
        app: &Router,
        board: &str,
        body: serde_json::Value,
    ) -> InitiateMessageResponse {
        call_ok(
            app,
            "POST",
            &format!("/boards/{board}/messages/initiate"),
            Some(body),
        )
        .await
    }

    #[tokio::test]
    async fn initiate_always_offers_s3() {
        let t = test_app().await;
        create_board(&t.app, "dkg1").await;

        // MAX_INLINE_MESSAGE_SIZE is 0: the smallest body and a large one are
        // both staged, which the trustee mailbox depends on (§6.4).
        for size in [1usize, 16 * 1024 * 1024] {
            let resp = initiate(&t.app, "dkg1", json!({ "size": size })).await;
            assert!(resp.should_upload, "a {size}-byte body was offered inline");
            let url = resp.upload_url.expect("a presigned upload url");
            let key = format!("/{BUCKET}/dkg1/messages/{}", resp.message_id);
            assert!(url.contains(&key), "{url} does not address {key}");
        }
    }

    #[tokio::test]
    async fn initiate_on_an_unknown_board_is_not_found() {
        let t = test_app().await;

        let (status, _) = call(
            &t.app,
            "POST",
            "/boards/nope/messages/initiate",
            Some(json!({ "size": 1 })),
        )
        .await;

        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn inline_and_staged_confirms_are_served_back_in_order() {
        let t = test_app().await;
        create_board(&t.app, "dkg1").await;

        // Confirm stores whatever the client sends — inline bytes here, although
        // initiate offered S3. The braid transports never take this path; b4
        // serves it back like any other row.
        let inline = initiate(&t.app, "dkg1", json!({ "size": 3 })).await;
        let (status, _) = call(
            &t.app,
            "POST",
            &format!("/boards/dkg1/messages/{}/confirm", inline.message_id),
            Some(json!({ "data": [1, 2, 3], "version": "1" })),
        )
        .await;
        assert_eq!(status, StatusCode::OK);

        let staged = initiate(&t.app, "dkg1", json!({ "size": 3 })).await;
        let (status, _) = call(
            &t.app,
            "POST",
            &format!("/boards/dkg1/messages/{}/confirm", staged.message_id),
            Some(json!({ "data": null, "version": "1" })),
        )
        .await;
        assert_eq!(status, StatusCode::OK);

        let listed: ListBlobsResponse =
            call_ok(&t.app, "GET", "/boards/dkg1/messages/list", None).await;
        assert_eq!(listed.messages.len(), 2);

        let fetched: GetBlobsResponse = call_ok(&t.app, "GET", "/boards/dkg1/messages", None).await;
        assert_eq!(fetched.messages.len(), 2);

        let first = &fetched.messages[0];
        match &first.message.content_type {
            ContentType::Inline { data } => assert_eq!(data, &[1, 2, 3]),
            other => panic!("expected the inline message first, got {other:?}"),
        }
        assert!(first.download_url.is_none());
        assert_eq!(first.message.version, "1");

        let second = &fetched.messages[1];
        let expected_key = format!("dkg1/messages/{}", staged.message_id);
        match &second.message.content_type {
            ContentType::S3 { key } => assert_eq!(key, &expected_key),
            other => panic!("expected the staged message second, got {other:?}"),
        }
        let url = second
            .download_url
            .as_deref()
            .expect("a presigned download url");
        assert!(url.contains(&expected_key), "{url}");
    }

    #[tokio::test]
    async fn board_names_outside_the_alphabet_are_rejected() {
        let t = test_app().await;

        let (status, _) = call(
            &t.app,
            "POST",
            "/boards",
            Some(json!({ "name": "bad/name" })),
        )
        .await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
    }
}
