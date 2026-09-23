// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
#![cfg(feature = "client")]
#[path = "support/postgres.rs"]
mod postgres;

// Two boards isolate reads and deletes; every fixture uses these names.
const BOARD: &str = "poll";
const OTHER_BOARD: &str = "other";

#[cfg(feature = "client")]
#[tokio::test]
async fn postgres_client_creates_reads_and_deletes_only_its_named_board() {
    use b3::client::pgsql::{B3MessageRow, PgsqlB3Client};
    let server = postgres::Postgres::start();
    let connection = server.parameters();
    let mut client = PgsqlB3Client::new(&connection).await.unwrap();
    client.create_index_ine().await.unwrap();
    client.create_board_ine(BOARD).await.unwrap();
    client.create_board_ine(BOARD).await.unwrap();
    client.create_board_ine(OTHER_BOARD).await.unwrap();
    assert!(client
        .create_board_ine("poll; DROP TABLE boards")
        .await
        .is_err());
    assert_eq!(client.get_boards().await.unwrap().len(), 2);
    for valid in ["_poll_9$".to_string(), "a".repeat(63)] {
        client.create_board_ine(&valid).await.unwrap();
        assert_eq!(client.get_message_count(&valid).await.unwrap(), 0);
        client.delete_board(&valid).await.unwrap();
    }
    for invalid in [
        "",
        "9poll",
        "poll-name",
        "poll\"",
        "poll, other",
        "poll CROSS JOIN other",
        &"a".repeat(64),
    ] {
        assert_eq!(
            client
                .create_board_ine(invalid)
                .await
                .unwrap_err()
                .to_string(),
            "Invalid board SQL identifier"
        );
        assert_eq!(
            client
                .get_message_count(invalid)
                .await
                .unwrap_err()
                .to_string(),
            "Invalid board SQL identifier"
        );
        assert_eq!(
            client
                .get_messages(invalid, 0)
                .await
                .unwrap_err()
                .to_string(),
            "Invalid board SQL identifier"
        );
        assert_eq!(
            client
                .get_one_message(invalid, 1)
                .await
                .unwrap_err()
                .to_string(),
            "Invalid board SQL identifier"
        );
        assert_eq!(
            client.delete_board(invalid).await.unwrap_err().to_string(),
            "Invalid board SQL identifier"
        );
    }

    assert!(client.delete_board("poll, other").await.is_err());
    assert_eq!(client.get_message_count(BOARD).await.unwrap(), 0);
    assert_eq!(client.get_message_count(OTHER_BOARD).await.unwrap(), 0);

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
    assert_eq!(
        client
            .insert_messages("poll, other", &vec![row.clone()])
            .await
            .unwrap_err()
            .to_string(),
        "Invalid board SQL identifier"
    );
    client.insert_messages(BOARD, &vec![row]).await.unwrap();
    let messages = client.get_messages(BOARD, 0).await.unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].created, 123);
    assert_eq!(messages[0].statement_timestamp, 124);
    assert_eq!(messages[0].message, [0, 255, 16]);
    assert_eq!(messages[0].batch, 7);
    assert_eq!(client.get_message_count(BOARD).await.unwrap(), 1);
    assert!(client
        .get_messages(OTHER_BOARD, 0)
        .await
        .unwrap()
        .is_empty());
    assert!(client
        .get_messages(BOARD, messages[0].id)
        .await
        .unwrap()
        .is_empty());
    // A duplicate later in a batch must roll back its preceding successful insert.
    let mut next = messages[0].clone();
    next.batch = 8;
    assert!(client
        .insert_messages(BOARD, &vec![next, messages[0].clone()])
        .await
        .is_err());
    assert_eq!(client.get_message_count(BOARD).await.unwrap(), 1);
    client.delete_board(BOARD).await.unwrap();
    assert!(client.get_board(BOARD).await.unwrap().is_none());
    assert!(client.get_board(OTHER_BOARD).await.unwrap().is_some());
}
