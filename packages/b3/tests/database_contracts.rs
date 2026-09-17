// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
#![cfg(feature = "client")]
#[path = "support/postgres.rs"]
mod postgres;

#[cfg(feature = "client")]
#[tokio::test]
async fn postgres_client_creates_reads_and_deletes_only_its_named_board() {
    use b3::client::pgsql::{B3MessageRow, PgsqlB3Client};
    let server = postgres::Postgres::start();
    let connection = server.parameters();
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
