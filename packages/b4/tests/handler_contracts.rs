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
use b4::{
    api_types::*,
    db,
    handlers::*,
    messages::{
        artifact::Configuration,
        message::{Message as BoardMessage, Signer},
        newtypes::{CiphertextsHash, ConfigurationHash},
        protocol_manager::ProtocolManager,
        statement::Statement,
    },
    state::AppState,
};
use std::marker::PhantomData;
use strand::{
    backend::ristretto::RistrettoCtx,
    serialization::StrandSerialize,
    signature::{StrandSignaturePk, StrandSignatureSk},
};

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

type Manager = ProtocolManager<RistrettoCtx>;
fn signer(seed: u8) -> Manager {
    // Synthetic, fixed PKCS#8 Ed25519 seeds; never production keys.
    let mut der = vec![
        0x30, 0x2e, 0x02, 0x01, 0x00, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x04, 0x22, 0x04,
        0x20,
    ];
    der.extend([seed; 32]);
    Manager::new(StrandSignatureSk::from_der(&der).unwrap())
}
fn public_key(signer: &Manager) -> StrandSignaturePk {
    StrandSignaturePk::from_sk(&signer.signing_key).unwrap()
}

/// A protocol manager and two trustees, with the configuration they form.
struct Members {
    manager: Manager,
    trustees: [Manager; 2],
    configuration: Configuration<RistrettoCtx>,
}
fn members() -> Members {
    let (manager, trustees) = (signer(1), [signer(2), signer(3)]);
    let configuration = Configuration::new(
        0x0102,
        public_key(&manager),
        trustees.iter().map(public_key).collect(),
        2,
        PhantomData,
    );
    Members {
        manager,
        trustees,
        configuration,
    }
}
fn trustee_message(members: &Members, trustee: usize) -> BoardMessage {
    BoardMessage::configuration_msg(&members.configuration, &members.trustees[trustee]).unwrap()
}

async fn provisioned_board(state: &AppState, board: &str) -> (Members, i64) {
    db::create_board(&state.db, board).await.unwrap();
    let members = members();
    let bootstrap = BoardMessage::bootstrap_msg(&members.configuration, &members.manager).unwrap();
    let id = db::insert_configuration(&state.db, board, &bootstrap)
        .await
        .unwrap();
    (members, id)
}

/// Confirms an inline upload that claims to be something else than it is.
async fn confirm(
    state: &AppState,
    board: &str,
    data: Vec<u8>,
) -> Result<Json<ConfirmMessageResponse>, StatusCode> {
    confirm_message(
        State(state.clone()),
        Path((board.into(), uuid::Uuid::new_v4().to_string())),
        Json(ConfirmMessageRequest {
            data: Some(data),
            sender_pk: "claimed-sender".into(),
            statement_kind: "Ballots".into(),
            batch: 99,
            mix_number: 7,
        }),
    )
    .await
}
async fn confirm_message_bytes(
    state: &AppState,
    board: &str,
    message: &BoardMessage,
) -> Result<Json<ConfirmMessageResponse>, StatusCode> {
    confirm(state, board, message.strand_serialize().unwrap()).await
}
fn multi_confirmation(
    board: &str,
    confirmations: Vec<Option<Vec<u8>>>,
) -> ConfirmMessagesMultiRequest {
    ConfirmMessagesMultiRequest {
        requests: vec![BoardConfirmRequest {
            board: board.into(),
            confirmations: confirmations
                .into_iter()
                .map(|data| MessageConfirmation {
                    message_id: uuid::Uuid::new_v4().to_string(),
                    data,
                })
                .collect(),
        }],
    }
}
async fn stored(state: &AppState, board: &str) -> usize {
    db::list_messages(&state.db, board).await.unwrap().len()
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
    let (members, configuration_id) = provisioned_board(&state, "poll").await;
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
    let message = trustee_message(&members, 0);
    let data = message.strand_serialize().unwrap();
    let confirmation = ConfirmMessageRequest {
        data: Some(data.clone()),
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
            last_id: Some(configuration_id),
            limit: Some(1),
        }),
    )
    .await
    .unwrap()
    .0;
    assert_eq!(page.messages.len(), 1);
    assert!(page.messages[0].download_url.is_none());
    assert!(
        matches!(&page.messages[0].message.content_type, ContentType::Inline { data: stored } if stored == &data)
    );
    let id = page.messages[0].message.id.clone();
    let single = get_message(State(state.clone()), Path(("poll".into(), id)))
        .await
        .unwrap()
        .0;
    assert_eq!(single.message.size, data.len());
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
    assert_eq!(list.messages.len(), 2);
    let multi = get_messages_multi(
        State(state),
        Json(GetMessagesMultiRequest {
            requests: vec![
                BoardMessageRequest {
                    board: "poll".into(),
                    last_id: configuration_id,
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

#[tokio::test]
async fn uploads_need_exactly_one_valid_provisioned_configuration() {
    let (_server, state) = fixture().await;
    let members = members();
    let message = trustee_message(&members, 0);
    let bootstrap = BoardMessage::bootstrap_msg(&members.configuration, &members.manager).unwrap();
    db::create_board(&state.db, "poll").await.unwrap();

    // Without a configuration nothing can be posted.
    assert_eq!(
        confirm_message_bytes(&state, "poll", &message)
            .await
            .unwrap_err(),
        StatusCode::PRECONDITION_FAILED
    );

    // A configuration that cannot be decoded is not a configuration.
    let garbage = Message {
        id: "garbage".into(),
        timestamp: 1_700_000_000,
        size: 3,
        content_type: ContentType::Inline {
            data: vec![0, 255, 16],
        },
        sender_pk: "garbage-sender".into(),
        statement_kind: "Configuration".into(),
        batch: 0,
        mix_number: 0,
    };
    db::insert_message(
        &state.db,
        "poll",
        &garbage,
        Some(&[0, 255, 16]),
        None,
        "1",
        "garbage-sender",
        "Configuration",
        0,
        0,
    )
    .await
    .unwrap();
    assert_eq!(
        confirm_message_bytes(&state, "poll", &message)
            .await
            .unwrap_err(),
        StatusCode::PRECONDITION_FAILED
    );

    // Two configurations leave no single one to trust.
    db::insert_configuration(&state.db, "poll", &bootstrap)
        .await
        .unwrap();
    assert_eq!(
        confirm_message_bytes(&state, "poll", &message)
            .await
            .unwrap_err(),
        StatusCode::PRECONDITION_FAILED
    );
    assert_eq!(stored(&state, "poll").await, 2);

    // A configuration that is not signed by its own protocol manager is foreign.
    db::create_board(&state.db, "foreign").await.unwrap();
    let mut foreign = bootstrap.try_clone().unwrap();
    foreign.sender.pk = public_key(&signer(9));
    db::insert_configuration(&state.db, "foreign", &foreign)
        .await
        .unwrap();
    assert_eq!(
        confirm_message_bytes(&state, "foreign", &message)
            .await
            .unwrap_err(),
        StatusCode::PRECONDITION_FAILED
    );

    // One valid configuration accepts a member's message.
    let (members, _) = provisioned_board(&state, "valid").await;
    assert!(
        confirm_message_bytes(&state, "valid", &trustee_message(&members, 0))
            .await
            .unwrap()
            .0
            .success
    );
}

#[tokio::test]
async fn stored_messages_are_described_by_their_content_and_duplicates_are_refused() {
    let (_server, state) = fixture().await;
    let (members, _) = provisioned_board(&state, "poll").await;
    let message = trustee_message(&members, 0);
    assert!(
        confirm_message_bytes(&state, "poll", &message)
            .await
            .unwrap()
            .0
            .success
    );

    let rows = db::list_messages(&state.db, "poll").await.unwrap();
    let row = rows.last().unwrap();
    assert_eq!(
        row.sender_pk,
        message.sender.pk.to_der_b64_string().unwrap()
    );
    assert_eq!(row.statement_kind, "ConfigurationSigned");
    assert_eq!((row.batch, row.mix_number), (0, 0));
    assert_eq!(row.size, message.strand_serialize().unwrap().len());

    assert_eq!(
        confirm_message_bytes(&state, "poll", &message)
            .await
            .unwrap_err(),
        StatusCode::INTERNAL_SERVER_ERROR
    );
    assert_eq!(stored(&state, "poll").await, 2);
}

#[tokio::test]
async fn uploads_that_do_not_verify_against_the_configuration_are_rejected() {
    let (_server, state) = fixture().await;
    let (members, _) = provisioned_board(&state, "poll").await;
    let valid = trustee_message(&members, 0);

    let outsider = BoardMessage::configuration_msg(&members.configuration, &signer(9)).unwrap();
    let mut mismatched_sender = valid.try_clone().unwrap();
    mismatched_sender.sender.pk = public_key(&members.trustees[1]);
    for rejected in [&outsider, &mismatched_sender] {
        assert_eq!(
            confirm_message_bytes(&state, "poll", rejected)
                .await
                .unwrap_err(),
            StatusCode::FORBIDDEN
        );
    }

    // A configuration is provisioned separately, even when its manager signed it.
    let bootstrap = BoardMessage::bootstrap_msg(&members.configuration, &members.manager).unwrap();
    assert_eq!(
        confirm_message_bytes(&state, "poll", &bootstrap)
            .await
            .unwrap_err(),
        StatusCode::FORBIDDEN
    );

    assert_eq!(
        confirm(&state, "poll", vec![0, 255, 16]).await.unwrap_err(),
        StatusCode::BAD_REQUEST
    );

    // The row fields must fit the database columns.
    let cfg_hash = ConfigurationHash::from_configuration(&members.configuration).unwrap();
    let oversized_batch = members.trustees[0]
        .sign(
            Statement::MixSigned(
                17,
                cfg_hash,
                i32::MAX as u64 + 1,
                1,
                CiphertextsHash([4; 64]),
                CiphertextsHash([5; 64]),
            ),
            None,
        )
        .unwrap();
    assert_eq!(
        confirm_message_bytes(&state, "poll", &oversized_batch)
            .await
            .unwrap_err(),
        StatusCode::BAD_REQUEST
    );

    assert_eq!(stored(&state, "poll").await, 1);
    assert!(
        confirm_message_bytes(&state, "poll", &valid)
            .await
            .unwrap()
            .0
            .success
    );
}

#[tokio::test]
async fn a_multi_board_confirmation_stores_nothing_unless_every_message_verifies() {
    let (_server, state) = fixture().await;
    let (members, _) = provisioned_board(&state, "poll").await;
    let valid = trustee_message(&members, 0).strand_serialize().unwrap();
    let outsider = BoardMessage::configuration_msg(&members.configuration, &signer(9))
        .unwrap()
        .strand_serialize()
        .unwrap();

    assert_eq!(
        confirm_messages_multi(
            State(state.clone()),
            Json(multi_confirmation(
                "poll",
                vec![Some(valid.clone()), Some(outsider)]
            ))
        )
        .await
        .unwrap_err(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(stored(&state, "poll").await, 1);

    assert!(
        confirm_messages_multi(
            State(state.clone()),
            Json(multi_confirmation("poll", vec![Some(valid)]))
        )
        .await
        .unwrap()
        .0
        .success
    );
    assert_eq!(stored(&state, "poll").await, 2);

    db::create_board(&state.db, "empty").await.unwrap();
    assert_eq!(
        confirm_messages_multi(
            State(state.clone()),
            Json(multi_confirmation(
                "empty",
                vec![Some(
                    trustee_message(&members, 1).strand_serialize().unwrap()
                )]
            ))
        )
        .await
        .unwrap_err(),
        StatusCode::PRECONDITION_FAILED
    );
    assert_eq!(
        confirm_messages_multi(
            State(state),
            Json(multi_confirmation("absent", vec![Some(vec![1])]))
        )
        .await
        .unwrap_err(),
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn uploads_that_cannot_be_fetched_from_storage_are_rejected() {
    let (_server, state) = fixture().await;
    let (_, _) = provisioned_board(&state, "poll").await;
    assert_eq!(
        confirm_message(
            State(state.clone()),
            Path(("poll".into(), uuid::Uuid::new_v4().to_string())),
            Json(ConfirmMessageRequest {
                data: None,
                sender_pk: "claimed-sender".into(),
                statement_kind: "Ballots".into(),
                batch: 1,
                mix_number: 0,
            })
        )
        .await
        .unwrap_err(),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        confirm_messages_multi(
            State(state.clone()),
            Json(multi_confirmation("poll", vec![None]))
        )
        .await
        .unwrap_err(),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(stored(&state, "poll").await, 1);
}

#[tokio::test]
async fn an_unreadable_configuration_is_a_server_error_and_not_a_missing_one() {
    let (_server, state) = fixture().await;
    let (members, _) = provisioned_board(&state, "poll").await;
    state
        .db
        .get()
        .await
        .unwrap()
        .batch_execute("DROP TABLE messages")
        .await
        .unwrap();
    assert_eq!(
        confirm_message_bytes(&state, "poll", &trustee_message(&members, 0))
            .await
            .unwrap_err(),
        StatusCode::INTERNAL_SERVER_ERROR
    );
}

#[tokio::test]
async fn only_configuration_messages_can_be_provisioned() {
    let (_server, state) = fixture().await;
    let (members, _) = provisioned_board(&state, "poll").await;
    assert!(
        db::insert_configuration(&state.db, "poll", &trustee_message(&members, 0))
            .await
            .unwrap_err()
            .to_string()
            .contains("Expected message to be a configuration")
    );
    let provisioned = db::get_configuration_messages(&state.db, "poll")
        .await
        .unwrap();
    let bootstrap = BoardMessage::bootstrap_msg(&members.configuration, &members.manager).unwrap();
    assert_eq!(provisioned, [bootstrap.strand_serialize().unwrap()]);
}
