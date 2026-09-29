// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
#![cfg(feature = "native")]

#[path = "support/postgres.rs"]
mod postgres;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use b4::{api_types::*, db, handlers::*, state::AppState};

async fn fixture() -> (postgres::Postgres, AppState) {
    let server = postgres::Postgres::start();
    let pool = db::init_db_with_params(&server.parameters()).await.unwrap();
    // Presigning is local: no default credential chain, metadata service or S3
    // request is involved in these handler controls.
    let config = aws_sdk_s3::config::Builder::new()
        .behavior_version_latest()
        .region(aws_sdk_s3::config::Region::new("us-east-1"))
        .credentials_provider(aws_sdk_s3::config::Credentials::new(
            "fixture",
            "fixture-secret",
            None,
            None,
            "test",
        ))
        .endpoint_url("http://127.0.0.1:1")
        .force_path_style(true)
        .build();
    let state = AppState {
        db: pool,
        s3_client: aws_sdk_s3::Client::from_conf(config),
        bucket_name: "synthetic-bucket".into(),
    };
    (server, state)
}
fn metadata(size: usize) -> InitiateMessageRequest {
    InitiateMessageRequest {
        size,
        sender_pk: "test-key".into(),
        statement_kind: "Ballots".into(),
        batch: 1,
        mix_number: 0,
    }
}

#[tokio::test]
async fn board_handlers_distinguish_invalid_names_duplicates_and_absent_boards() {
    let (_server, state) = fixture().await;
    assert_eq!(
        get_board(State(state.clone()), Path("missing".into()))
            .await
            .unwrap_err(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        create_board(
            State(state.clone()),
            Json(CreateBoardRequest {
                name: "../poll".into()
            })
        )
        .await
        .unwrap_err(),
        StatusCode::BAD_REQUEST
    );
    let created = create_board(
        State(state.clone()),
        Json(CreateBoardRequest {
            name: "poll".into(),
        }),
    )
    .await
    .unwrap()
    .0;
    assert_eq!(created.name, "poll");
    assert_eq!(created.status, "active");
    assert_eq!(
        get_board(State(state.clone()), Path("poll".into()))
            .await
            .unwrap()
            .0
            .name,
        "poll"
    );
    assert_eq!(
        create_board(
            State(state.clone()),
            Json(CreateBoardRequest {
                name: "poll".into()
            })
        )
        .await
        .unwrap_err(),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(list_boards(State(state)).await.unwrap().0.boards.len(), 1);
}

#[tokio::test]
async fn inline_upload_is_readable_through_single_paginated_and_multi_board_handlers() {
    let (_server, state) = fixture().await;
    db::create_board(&state.db, "poll").await.unwrap();
    let upload = initiate_message(
        State(state.clone()),
        Path("poll".into()),
        Json(metadata(MAX_INLINE_MESSAGE_SIZE)),
    )
    .await
    .unwrap()
    .0;
    assert!(!upload.should_upload);
    assert!(upload.upload_url.is_none());
    assert!(uuid::Uuid::parse_str(&upload.message_id).is_ok());
    assert_eq!(
        initiate_message(
            State(state.clone()),
            Path("absent".into()),
            Json(metadata(0))
        )
        .await
        .unwrap_err(),
        StatusCode::NOT_FOUND
    );
    let confirmation = ConfirmMessageRequest {
        data: Some(vec![0, 255, 16]),
        sender_pk: "test-key".into(),
        statement_kind: "Ballots".into(),
        batch: 1,
        mix_number: 0,
    };
    assert!(
        confirm_message(
            State(state.clone()),
            Path(("poll".into(), upload.message_id)),
            Json(confirmation)
        )
        .await
        .unwrap()
        .0
        .success
    );
    let page = get_messages(
        State(state.clone()),
        Path("poll".into()),
        Query(GetMessagesQuery {
            last_id: Some(0),
            limit: Some(1),
        }),
    )
    .await
    .unwrap()
    .0;
    assert_eq!(page.messages.len(), 1);
    assert!(page.messages[0].download_url.is_none());
    assert!(
        matches!(&page.messages[0].message.content_type,ContentType::Inline { data } if data == &[0,255,16])
    );
    let id = page.messages[0].message.id.clone();
    let single = get_message(State(state.clone()), Path(("poll".into(), id)))
        .await
        .unwrap()
        .0;
    assert_eq!(single.message.size, 3);
    assert_eq!(
        get_message(
            State(state.clone()),
            Path(("poll".into(), "invalid".into()))
        )
        .await
        .unwrap_err(),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        get_message(State(state.clone()), Path(("poll".into(), "99999".into())))
            .await
            .unwrap_err(),
        StatusCode::NOT_FOUND
    );
    let list = list_messages(
        State(state.clone()),
        Path("poll".into()),
        Query(GetMessagesQuery {
            last_id: None,
            limit: None,
        }),
    )
    .await
    .unwrap()
    .0;
    assert_eq!(list.messages.len(), 1);
    let multi = get_messages_multi(
        State(state),
        Json(GetMessagesMultiRequest {
            requests: vec![
                BoardMessageRequest {
                    board: "poll".into(),
                    last_id: 0,
                    limit: Some(1),
                },
                BoardMessageRequest {
                    board: "missing".into(),
                    last_id: 0,
                    limit: None,
                },
            ],
        }),
    )
    .await
    .unwrap()
    .0;
    assert_eq!(multi.boards.len(), 2);
    assert_eq!(multi.boards[0].board, "poll");
    assert_eq!(multi.boards[0].messages.len(), 1);
    assert!(multi.boards[1].messages.is_empty());
}

#[tokio::test]
async fn database_errors_are_not_reported_as_missing_or_empty_results() {
    let (_server, state) = fixture().await;
    db::create_board(&state.db, "poll").await.unwrap();
    assert_eq!(
        list_boards(State(state.clone()))
            .await
            .unwrap()
            .0
            .boards
            .len(),
        1
    );
    state
        .db
        .get()
        .await
        .unwrap()
        .batch_execute("DROP TABLE boards")
        .await
        .unwrap();
    assert_eq!(
        list_boards(State(state.clone())).await.unwrap_err(),
        StatusCode::INTERNAL_SERVER_ERROR
    );
    assert_eq!(
        get_board(State(state.clone()), Path("poll".into()))
            .await
            .unwrap_err(),
        StatusCode::INTERNAL_SERVER_ERROR
    );
    assert_eq!(
        initiate_message(State(state), Path("poll".into()), Json(metadata(0)))
            .await
            .unwrap_err(),
        StatusCode::INTERNAL_SERVER_ERROR
    );
}

struct Expirations([Option<std::ffi::OsString>; 2]);
impl Expirations {
    fn save() -> Self {
        Self([
            std::env::var_os("AWS_S3_UPLOAD_EXPIRATION_SECS"),
            std::env::var_os("AWS_S3_FETCH_EXPIRATION_SECS"),
        ])
    }
}
impl Drop for Expirations {
    fn drop(&mut self) {
        for (name, previous) in [
            "AWS_S3_UPLOAD_EXPIRATION_SECS",
            "AWS_S3_FETCH_EXPIRATION_SECS",
        ]
        .into_iter()
        .zip(&self.0)
        {
            if let Some(value) = previous {
                std::env::set_var(name, value);
            } else {
                std::env::remove_var(name);
            }
        }
    }
}

#[tokio::test]
async fn large_uploads_use_local_presigning_and_bad_expiration_is_an_error() {
    let _restore = Expirations::save();
    let (_server, state) = fixture().await;
    db::create_board(&state.db, "poll").await.unwrap();
    std::env::set_var("AWS_S3_UPLOAD_EXPIRATION_SECS", "60");
    std::env::set_var("AWS_S3_FETCH_EXPIRATION_SECS", "90");
    let upload = initiate_message(
        State(state.clone()),
        Path("poll".into()),
        Json(metadata(MAX_INLINE_MESSAGE_SIZE + 1)),
    )
    .await
    .unwrap()
    .0;
    assert!(upload.should_upload);
    let url = upload.upload_url.unwrap();
    assert!(url.starts_with(&format!(
        "http://127.0.0.1:1/synthetic-bucket/poll/messages/{}?",
        upload.message_id
    )));
    assert!(url.contains("X-Amz-Expires=60"));
    let url = b4::s3::generate_download_url(&state.s3_client, "synthetic-bucket", "poll/a b")
        .await
        .unwrap();
    assert!(url.starts_with("http://127.0.0.1:1/synthetic-bucket/poll/a%20b?"));
    assert!(url.contains("X-Amz-Expires=90"));
    for invalid in ["not-a-number", "604801"] {
        std::env::set_var("AWS_S3_UPLOAD_EXPIRATION_SECS", invalid);
        assert_eq!(
            initiate_message(
                State(state.clone()),
                Path("poll".into()),
                Json(metadata(MAX_INLINE_MESSAGE_SIZE + 1))
            )
            .await
            .unwrap_err(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }
    std::env::remove_var("AWS_S3_FETCH_EXPIRATION_SECS");
    assert!(
        b4::s3::generate_download_url(&state.s3_client, "synthetic-bucket", "poll/a")
            .await
            .unwrap_err()
            .to_string()
            .contains("environment variable not found")
    );
}
