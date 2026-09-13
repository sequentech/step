// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use serial_test::serial;

const BOARD_DB: &'static str = "testdb";

async fn set_up() -> BoardClient {
    let mut b = BoardClient::new("http://localhost:3322", "immudb", "immudb")
        .await
        .unwrap();

    // In case the previous test did not clean up properly
    b.delete_database(BOARD_DB).await.unwrap();
    b.upsert_electoral_log_db(BOARD_DB).await.unwrap();

    b
}

async fn tear_down(mut b: BoardClient) {
    b.delete_database(BOARD_DB).await.unwrap();
}

#[tokio::test]
#[ignore]
#[serial]
pub async fn test_message_create_retrieve() {
    let mut b = set_up().await;
    let electoral_log_message = ElectoralLogMessage {
        id: 1,
        created: 555,
        sender_pk: "".to_string(),
        statement_timestamp: 0,
        statement_kind: "".to_string(),
        message: vec![],
        version: "".to_string(),
        user_id: None,
        username: None,
        election_id: None,
        area_id: None,
        ballot_id: None,
    };
    let messages = vec![electoral_log_message];

    b.insert_electoral_log_messages(BOARD_DB, &messages)
        .await
        .unwrap();

    let ret = b.get_electoral_log_messages(BOARD_DB).await.unwrap();
    assert_eq!(messages, ret);

    let cols_match = BTreeMap::from([
        (
            ElectoralLogVarCharColumn::StatementKind,
            (SqlCompOperators::Equal, "".to_string()),
        ),
        (
            ElectoralLogVarCharColumn::SenderPk,
            (SqlCompOperators::Equal, "".to_string()),
        ),
    ]);
    let ret = b
        .get_electoral_log_messages_filtered::<String, String>(
            BOARD_DB,
            Some(cols_match.clone()),
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .unwrap();
    assert_eq!(messages, ret);
    let ret = b
        .get_electoral_log_messages_filtered::<String, String>(
            BOARD_DB,
            Some(cols_match.clone()),
            Some(1i64),
            None,
            None,
            None,
            None,
        )
        .await
        .unwrap();
    assert_eq!(messages, ret);
    let ret = b
        .get_electoral_log_messages_filtered::<String, String>(
            BOARD_DB,
            Some(cols_match.clone()),
            None,
            Some(556i64),
            None,
            None,
            None,
        )
        .await
        .unwrap();
    assert_eq!(messages, ret);
    let ret = b
        .get_electoral_log_messages_filtered::<String, String>(
            BOARD_DB,
            Some(cols_match.clone()),
            Some(1i64),
            Some(556i64),
            None,
            None,
            None,
        )
        .await
        .unwrap();
    assert_eq!(messages, ret);
    let ret = b
        .get_electoral_log_messages_filtered::<String, String>(
            BOARD_DB,
            Some(cols_match),
            Some(556i64),
            Some(666i64),
            None,
            None,
            None,
        )
        .await
        .unwrap();
    assert_eq!(ret.len(), 0);

    tear_down(b).await;
}
