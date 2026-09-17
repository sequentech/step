// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
#![cfg(feature = "native")]

#[path = "support/postgres.rs"]
mod postgres;
use b4::{
    api_types::{ContentType, Message},
    db,
};

async fn database() -> (postgres::Postgres, db::DbPool) {
    let server = postgres::Postgres::start();
    let pool = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        db::init_db_with_params(&server.parameters()),
    )
    .await
    .unwrap()
    .unwrap();
    (server, pool)
}
fn inline(batch: i32) -> Message {
    Message {
        id: "server-assigned".into(),
        timestamp: 1_700_000_123,
        size: 3,
        content_type: ContentType::Inline {
            data: vec![0, 255, 16],
        },
        sender_pk: "synthetic-trustee".into(),
        statement_kind: "Ballots".into(),
        batch,
        mix_number: 0,
    }
}
async fn insert(pool: &db::DbPool, board: &str, batch: i32) -> i64 {
    let message = inline(batch);
    db::insert_message(
        pool,
        board,
        &message,
        Some(&[0, 255, 16]),
        None,
        "1",
        &message.sender_pk,
        &message.statement_kind,
        batch,
        0,
    )
    .await
    .unwrap()
}

#[test]
fn board_names_reject_sql_and_path_delimiters_and_enforce_the_byte_limit() {
    for name in ["board", "poll-1_A", "élection"] {
        db::validate_board_name(name).unwrap();
    }
    db::validate_board_name(&"a".repeat(255)).unwrap();
    assert!(db::validate_board_name(&"a".repeat(256))
        .unwrap_err()
        .to_string()
        .contains("too long"));
    assert!(db::validate_board_name(&"é".repeat(128))
        .unwrap_err()
        .to_string()
        .contains("too long"));
    assert!(db::validate_board_name("")
        .unwrap_err()
        .to_string()
        .contains("cannot be empty"));
    for name in ["../board", "x/y", "x'y", "x\"y", "x;y", "x y", "x\ny"] {
        assert!(db::validate_board_name(name)
            .unwrap_err()
            .to_string()
            .contains("invalid characters"));
    }
}

#[tokio::test]
async fn boards_keep_metadata_and_duplicate_creation_leaves_the_original_intact() {
    let (_server, pool) = database().await;
    assert!(db::get_board(&pool, "poll").await.unwrap().is_none());
    let created = db::create_board(&pool, "poll").await.unwrap();
    assert_eq!(created.status, "active");
    assert!(db::create_board(&pool, "poll").await.is_err());
    let boards = db::list_boards(&pool).await.unwrap();
    assert_eq!(boards.len(), 1);
    assert_eq!(boards[0].name, "poll");
    assert_eq!(
        db::get_board(&pool, "poll").await.unwrap().unwrap().status,
        "active"
    );
    db::update_board_config_metadata(&pool, "poll", "cfg-7", 2, 3)
        .await
        .unwrap();
    let connection = pool.get().await.unwrap();
    let row = connection
        .query_one(
            "SELECT cfg_id, threshold_no, trustees_no FROM boards WHERE board_name='poll'",
            &[],
        )
        .await
        .unwrap();
    assert_eq!(row.get::<_, String>(0), "cfg-7");
    assert_eq!(row.get::<_, i32>(1), 2);
    assert_eq!(row.get::<_, i32>(2), 3);
}

#[tokio::test]
async fn inline_rows_preserve_binary_data_and_pagination_is_exclusive_and_ordered() {
    let (_server, pool) = database().await;
    db::create_board(&pool, "poll").await.unwrap();
    db::create_board(&pool, "other").await.unwrap();
    let first = insert(&pool, "poll", 1).await;
    let second = insert(&pool, "poll", 2).await;
    insert(&pool, "other", 1).await;
    let third = insert(&pool, "poll", 3).await;
    let message = db::get_message(&pool, "poll", first)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(message.timestamp, 1_700_000_123);
    assert_eq!(message.size, 3);
    assert_eq!(message.sender_pk, "synthetic-trustee");
    assert_eq!(message.statement_kind, "Ballots");
    assert_eq!(message.batch, 1);
    assert!(matches!(message.content_type, ContentType::Inline { data } if data == [0,255,16]));
    assert!(db::get_message(&pool, "other", first)
        .await
        .unwrap()
        .is_none());
    let (page, more) = db::get_messages_after(&pool, "poll", 0, 2).await.unwrap();
    assert_eq!(
        page.iter().map(|m| m.id.clone()).collect::<Vec<_>>(),
        [first.to_string(), second.to_string()]
    );
    assert!(more);
    let (page, more) = db::get_messages_after(&pool, "poll", second, 2)
        .await
        .unwrap();
    assert_eq!(
        page.iter().map(|m| m.id.clone()).collect::<Vec<_>>(),
        [third.to_string()]
    );
    assert!(!more);
    assert_eq!(db::list_messages(&pool, "poll").await.unwrap().len(), 3);
    let row = pool
        .get()
        .await
        .unwrap()
        .query_one(
            "SELECT message_count, batch_count FROM boards WHERE board_name='poll'",
            &[],
        )
        .await
        .unwrap();
    assert_eq!(row.get::<_, i32>(0), 3);
    assert_eq!(row.get::<_, i32>(1), 3);
}

#[tokio::test]
async fn legacy_client_rows_and_s3_rows_map_consistently_in_all_read_paths() {
    let (_server, pool) = database().await;
    db::create_board(&pool, "poll").await.unwrap();
    let connection = pool.get().await.unwrap();
    // Older clients populate `message`, without HTTP storage metadata. Exercise
    // the compatibility row shape directly, independently of the HTTP writer.
    let id: i64 = connection.query_one("INSERT INTO messages (board_name, sender_pk, statement_kind, batch, mix_number, version, message) VALUES ('poll','legacy','Shares',1,0,'1',$1) RETURNING id", &[&vec![7u8,8,9,10]]).await.unwrap().get(0);
    drop(connection);
    let reads = [
        db::get_message(&pool, "poll", id).await.unwrap().unwrap(),
        db::list_messages(&pool, "poll").await.unwrap().remove(0),
        db::get_messages_after(&pool, "poll", 0, 10)
            .await
            .unwrap()
            .0
            .remove(0),
    ];
    for message in reads {
        assert_eq!(message.size, 4);
        assert_eq!(message.timestamp, 0);
        assert!(matches!(message.content_type,ContentType::Inline { data } if data == [7,8,9,10]));
    }
    let mut message = inline(2);
    message.size = 123;
    message.content_type = ContentType::S3 {
        key: "poll/object".into(),
    };
    let id = db::insert_message(
        &pool,
        "poll",
        &message,
        None,
        Some("poll/object"),
        "1",
        "sender",
        "Mix",
        2,
        1,
    )
    .await
    .unwrap();
    let stored = db::get_message(&pool, "poll", id).await.unwrap().unwrap();
    assert_eq!(stored.size, 123);
    assert!(matches!(stored.content_type,ContentType::S3 { key } if key == "poll/object"));
}

#[cfg(feature = "client")]
#[tokio::test]
async fn postgres_client_creates_reads_and_deletes_only_its_named_board() {
    use b4::client::pgsql::{B3MessageRow, PgsqlB3Client, PgsqlConnectionParams};
    let server = postgres::Postgres::start();
    let params = server.parameters();
    let connection = PgsqlConnectionParams::new(
        &params.host,
        u32::from(params.port),
        &params.username,
        &params.password,
    )
    .with_database(&params.database);
    let mut client = PgsqlB3Client::new(&connection).await.unwrap();
    client.create_index_ine().await.unwrap();
    client.create_board_ine("poll").await.unwrap();
    client.create_board_ine("poll").await.unwrap();
    client.create_board_ine("other").await.unwrap();
    assert!(client
        .create_board_ine("poll; DROP TABLE boards")
        .await
        .is_err());
    assert_eq!(client.get_boards().await.unwrap().len(), 2);
    let row = B3MessageRow {
        id: 0,
        created: 123,
        sender_pk: "synthetic-key".into(),
        statement_timestamp: 124,
        statement_kind: "Shares".into(),
        batch: 7,
        mix_number: 0,
        message: vec![0, 255, 16],
        version: "1".into(),
    };
    client.insert_messages("poll", &vec![row]).await.unwrap();
    let messages = client.get_messages("poll", 0).await.unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].created, 123);
    assert_eq!(messages[0].statement_timestamp, 124);
    assert_eq!(messages[0].message, [0, 255, 16]);
    assert_eq!(messages[0].batch, 7);
    assert_eq!(client.get_message_count("poll").await.unwrap(), 1);
    assert!(client.get_messages("other", 0).await.unwrap().is_empty());
    assert!(client
        .get_messages("poll", messages[0].id)
        .await
        .unwrap()
        .is_empty());
    // A duplicate later in a batch must roll back its preceding successful insert.
    let mut next = messages[0].clone();
    next.batch = 8;
    assert!(client
        .insert_messages("poll", &vec![next, messages[0].clone()])
        .await
        .is_err());
    assert_eq!(client.get_message_count("poll").await.unwrap(), 1);
    client.delete_board("poll").await.unwrap();
    assert!(client.get_board("poll").await.unwrap().is_none());
    assert!(client.get_board("other").await.unwrap().is_some());
}
