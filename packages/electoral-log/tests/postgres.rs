// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Result};
use electoral_log::{adapters::postgres::PostgresStore, domain::*, service::BoardClient};
use std::sync::Arc;
use uuid::Uuid;

async fn setup() -> Result<(BoardClient, String)> {
    let config = std::env::var("ELECTORAL_LOG_TEST_DATABASE_URL")?.parse()?;
    let store = PostgresStore::new(config)?;
    store.initialize().await?;
    let client = BoardClient::new(Arc::new(store));
    let board = format!("test-{}", Uuid::new_v4());
    client.create_board(&board).await?;
    Ok((client, board))
}

fn entry(delivery: &str, created: i64, user: Option<&str>) -> LogEntry {
    LogEntry {
        delivery_id: delivery.into(),
        message: ElectoralLogMessage {
            id: 0,
            created,
            sender_pk: "sender".into(),
            statement_timestamp: created,
            statement_kind: "CastVote".into(),
            message: vec![0, 255, 128, 39, 0],
            version: "2".into(),
            user_id: user.map(str::to_owned),
            username: Some("O'Brien".into()),
            election_id: Some("election-a".into()),
            area_id: Some("area-a".into()),
            ballot_id: Some("00ff".into()),
        },
    }
}

#[tokio::test]
#[ignore = "requires ELECTORAL_LOG_TEST_DATABASE_URL"]
async fn raw_roundtrip_scope_isolation_and_deletion() -> Result<()> {
    let (client, board) = setup().await?;
    let other = format!("{board}-other");
    client.create_board(&other).await?;
    let original = entry("same-delivery", 0, None);
    client.append(&board, &[original.clone()]).await?;
    client.append(&other, &[original.clone()]).await?;
    let mut rows = client.query(&board, &LogQuery::default()).await?;
    assert_eq!(rows.len(), 1);
    assert!(rows[0].id > 0);
    rows[0].id = 0;
    assert_eq!(rows[0], original.message);
    client.delete_board(&board).await?;
    assert!(!client.has_board(&board).await?);
    assert_eq!(client.count(&board, &LogQuery::default()).await?, 0);
    assert_eq!(client.count(&other, &LogQuery::default()).await?, 1);
    assert!(client.append(&board, &[original]).await.is_err());
    client.delete_board(&other).await?;
    Ok(())
}

#[tokio::test]
#[ignore = "requires ELECTORAL_LOG_TEST_DATABASE_URL"]
async fn retries_are_atomic_and_distinct_deliveries_survive() -> Result<()> {
    let (client, board) = setup().await?;
    let entries = vec![entry("one", 1, Some("user")), entry("two", 1, Some("user"))];
    let (left, right) = tokio::join!(
        client.append(&board, &entries),
        client.append(&board, &entries)
    );
    left?;
    right?;
    assert_eq!(client.count(&board, &LogQuery::default()).await?, 2);
    // A failure after an earlier insert must not commit that earlier insert.
    let mut invalid = entry("", 2, None);
    assert!(client
        .append(&board, &[entry("three", 2, None), invalid.clone()])
        .await
        .is_err());
    assert_eq!(client.count(&board, &LogQuery::default()).await?, 2);
    // Parsing an import fails after an earlier row has been written.
    let mut import = vec![
        Ok(entry("import", 3, None)),
        Err(anyhow!("invalid CSV row")),
    ]
    .into_iter();
    assert!(client.append_iter(&board, &mut import).await.is_err());
    assert_eq!(client.count(&board, &LogQuery::default()).await?, 2);
    invalid.delivery_id = "three".into();
    client.append(&board, &[invalid]).await?;
    assert_eq!(client.count(&board, &LogQuery::default()).await?, 3);
    // Cursor safety depends on serializing writes before allocating row IDs.
    // NO KEY UPDATE permits the FK check but conflicts with the adapter's lock.
    let mut config: tokio_postgres::Config =
        std::env::var("ELECTORAL_LOG_TEST_DATABASE_URL")?.parse()?;
    let (mut connection, driver) = config.connect(tokio_postgres::NoTls).await?;
    tokio::spawn(async move {
        driver.await.unwrap();
    });
    let tx = connection.transaction().await?;
    tx.query_one(
        "SELECT board_name FROM electoral_log_boards WHERE board_name = $1 FOR NO KEY UPDATE",
        &[&board],
    )
    .await?;
    config.options("-c lock_timeout=100ms");
    let blocked = BoardClient::new(Arc::new(PostgresStore::new(config)?));
    let error = blocked
        .append(&board, &[entry("locked", 4, None)])
        .await
        .unwrap_err();
    assert_eq!(
        error
            .downcast_ref::<tokio_postgres::Error>()
            .and_then(|error| error.code()),
        Some(&tokio_postgres::error::SqlState::LOCK_NOT_AVAILABLE)
    );
    tx.rollback().await?;
    blocked.append(&board, &[entry("locked", 4, None)]).await?;
    client.delete_board(&board).await?;
    Ok(())
}

#[tokio::test]
#[ignore = "requires ELECTORAL_LOG_TEST_DATABASE_URL"]
async fn filters_count_order_and_cursor_cover_every_row() -> Result<()> {
    let (client, board) = setup().await?;
    let entries: Vec<_> = (0..7)
        .map(|n| entry(&format!("delivery-{n}"), n, Some("O'Brien")))
        .collect();
    client.append(&board, &entries).await?;
    let mut query = LogQuery::default();
    query.filters.push(Filter::Text(
        TextColumn::UserId,
        SqlCompOperators::Equal,
        "O'Brien".into(),
    ));
    query.filters.push(Filter::Number(
        NumberColumn::Created,
        NumberComparison::GreaterThanOrEqual,
        0,
    ));
    query.filters.push(Filter::Number(
        NumberColumn::Created,
        NumberComparison::LessThan,
        3,
    ));
    query.order = vec![(OrderColumn::Created, SortDirection::Desc)];
    query.limit = 2;
    assert_eq!(client.count(&board, &query).await?, 3);
    let rows = client.query(&board, &query).await?;
    assert_eq!(
        rows.iter().map(|r| r.created).collect::<Vec<_>>(),
        vec![2, 1]
    );
    query.offset = 2;
    assert_eq!(client.query(&board, &query).await?[0].created, 0);
    query.filters.push(Filter::Text(
        TextColumn::Username,
        SqlCompOperators::Equal,
        "' OR TRUE --".into(),
    ));
    assert_eq!(client.count(&board, &query).await?, 0);
    let zero = client
        .get_electoral_log_messages_filtered::<String, String>(
            &board,
            None,
            Some(0),
            Some(0),
            Some(10),
            Some(0),
            None,
        )
        .await?;
    assert_eq!(
        zero.iter().map(|row| row.created).collect::<Vec<_>>(),
        vec![0]
    );
    let invalid_order = std::collections::HashMap::from([(
        "id; DROP TABLE electoral_log_messages".to_owned(),
        "asc".to_owned(),
    )]);
    assert!(client
        .get_electoral_log_messages_filtered(
            &board,
            None,
            None,
            None,
            None,
            None,
            Some(invalid_order)
        )
        .await
        .is_err());
    let mut ids = Vec::new();
    let mut cursor = 0;
    loop {
        let rows = client
            .get_electoral_log_messages_batch(&board, 2, cursor)
            .await?;
        if rows.is_empty() {
            break;
        }
        cursor = rows.last().unwrap().id;
        ids.extend(rows.into_iter().map(|row| row.id));
        assert!(ids.len() <= 7, "cursor must advance without repeating rows");
    }
    assert_eq!(ids.len(), 7);
    assert!(ids.windows(2).all(|w| w[0] < w[1]));
    assert_eq!(
        client
            .get_electoral_log_messages_at_offset(&board, 2, 2)
            .await?
            .iter()
            .map(|r| r.id)
            .collect::<Vec<_>>(),
        ids[2..4]
    );
    client.delete_board(&board).await?;
    Ok(())
}

#[tokio::test]
#[ignore = "requires ELECTORAL_LOG_TEST_DATABASE_URL"]
async fn visibility_and_user_filters_apply_to_both_list_and_count() -> Result<()> {
    let (client, board) = setup().await?;
    let mut general = entry("general", 1, None);
    general.message.election_id = None;
    general.message.area_id = None;
    let mut other = entry("other", 2, Some("user"));
    other.message.election_id = Some("election-b".into());
    other.message.area_id = Some("area-b".into());
    client
        .append(&board, &[general, other, entry("match", 3, Some("user"))])
        .await?;
    let mut query = LogQuery::default();
    query.visibility = Some(LogVisibility {
        election_id: Some("election-a".into()),
        area_ids: vec![],
    });
    assert_eq!(client.count(&board, &query).await?, 2);
    assert_eq!(client.query(&board, &query).await?.len(), 2);
    query.only_with_user = true;
    assert_eq!(client.count(&board, &query).await?, 1);
    assert_eq!(client.query(&board, &query).await?[0].created, 3);
    query.visibility = Some(LogVisibility {
        election_id: None,
        area_ids: vec!["area-b".into()],
    });
    assert_eq!(client.query(&board, &query).await?[0].created, 2);
    query.visibility = Some(LogVisibility {
        election_id: None,
        area_ids: vec![],
    });
    assert_eq!(client.count(&board, &query).await?, 0);
    query.limit = -1;
    assert!(client.query(&board, &query).await.is_err());
    client.delete_board(&board).await?;
    Ok(())
}

#[tokio::test]
#[ignore = "requires ELECTORAL_LOG_TEST_DATABASE_URL"]
async fn trellis_atomic_append_proofs_and_restart() -> Result<()> {
    use electoral_log::{ports::ElectoralLogStore, proofs::leaf_hash};
    let store = PostgresStore::new(std::env::var("ELECTORAL_LOG_TEST_DATABASE_URL")?.parse()?)?;
    store.initialize().await?;
    let board = format!("trellis-{}", Uuid::new_v4());
    store.create_board(&board).await?;
    let client = BoardClient::new(Arc::new(store.clone()));
    let journal = store.journal();
    let (empty, committed) = journal.checkpoint(&board).await?;
    assert_eq!(committed, 0);
    client.append(&board, &[entry("a", 1, None)]).await?;
    let first = client.query(&board, &LogQuery::default()).await?.remove(0);
    assert!(store
        .record_proof(&journal, &board, first.id)
        .await?
        .is_none());
    let bad = vec![entry("rolled-back", 2, None), entry("", 3, None)];
    assert!(client.append(&board, &bad).await.is_err());
    assert_eq!(journal.checkpoint(&board).await?.1, 1);
    journal.process_once().await?;
    let proof = store
        .record_proof(&journal, &board, first.id)
        .await?
        .unwrap();
    let old = proof.inclusion.checkpoint.clone();
    proof.verify(&old)?;
    journal.consistency(&empty).await?.verify(&empty)?;
    let mut changed = proof.entry.clone();
    changed.message.username = Some("changed".into());
    assert!(proof
        .inclusion
        .verify(&leaf_hash(&board, &changed)?, &old)
        .is_err());
    let second = entry("b", 1, None);
    let (a, b) = tokio::join!(
        client.append(&board, std::slice::from_ref(&second)),
        client.append(&board, std::slice::from_ref(&second))
    );
    a?;
    b?;
    assert_eq!(journal.checkpoint(&board).await?.1, 2);
    journal.process_once().await?;
    let consistency = journal.consistency(&old).await?;
    consistency.verify(&old)?;
    let mut wrong = old.clone();
    wrong.tree_size += 1;
    assert!(consistency.verify(&wrong).is_err());
    let checkpoint = journal.checkpoint(&board).await?.0;
    let restarted = store.journal();
    restarted.process_once().await?;
    assert_eq!(restarted.checkpoint(&board).await?.0, checkpoint);
    restarted.consistency(&old).await?.verify(&old)?;
    // Refresh an already-running replica even when another replica consumed the last write.
    journal.process_once().await?;
    assert_eq!(journal.checkpoint(&board).await?.0, checkpoint);
    store.delete_board(&board).await?;
    store.create_board(&board).await?;
    restarted.process_once().await?;
    assert!(restarted.consistency(&old).await.is_err());
    assert_eq!(restarted.checkpoint(&board).await?.1, 0);
    store.delete_board(&board).await?;
    Ok(())
}
