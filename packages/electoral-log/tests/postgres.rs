// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Result};
use electoral_log::{
    adapters::postgres::{LogScope, PostgresStore},
    domain::*,
    service::BoardClient,
};
use std::sync::Arc;
use trellis::journal::INSERT_CHUNK;
use uuid::Uuid;

/// Applying the schema takes locks that deadlock with the appends of tests running
/// at the same time, so each test binary applies it once.
static SCHEMA: tokio::sync::OnceCell<()> = tokio::sync::OnceCell::const_new();

async fn initialized(store: &PostgresStore) -> Result<()> {
    SCHEMA.get_or_try_init(|| store.initialize()).await?;
    Ok(())
}

async fn setup() -> Result<(BoardClient, String)> {
    let config = std::env::var("ELECTORAL_LOG_TEST_DATABASE_URL")?.parse()?;
    let store = PostgresStore::new(config)?;
    initialized(&store).await?;
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

async fn trellis_store() -> Result<(PostgresStore, String, tokio_postgres::Client)> {
    use electoral_log::ports::ElectoralLogStore;
    let url = std::env::var("ELECTORAL_LOG_TEST_DATABASE_URL")?;
    let store = PostgresStore::new(url.parse()?)?;
    initialized(&store).await?;
    let board = format!("trellis-{}", Uuid::new_v4());
    store.create_board(&board).await?;
    let (db, connection) = tokio_postgres::connect(&url, tokio_postgres::NoTls).await?;
    tokio::spawn(connection);
    Ok((store, board, db))
}

async fn newest(client: &BoardClient, board: &str) -> Result<ElectoralLogMessage> {
    Ok(client
        .query(
            board,
            &LogQuery {
                limit: 1,
                ..LogQuery::default()
            },
        )
        .await?
        .remove(0))
}

#[tokio::test]
#[ignore = "requires ELECTORAL_LOG_TEST_DATABASE_URL"]
async fn trellis_atomic_append_proofs_and_board_recreation() -> Result<()> {
    use electoral_log::{
        ports::ElectoralLogStore,
        proofs::{leaf_hash, JournalError, LogIdentity},
    };
    let (store, board, _db) = trellis_store().await?;
    let client = BoardClient::new(Arc::new(store.clone()));
    let journal = store.journal();
    let empty = journal.checkpoint(&board).await?;
    assert_eq!(empty.tree_size, 0);
    assert_eq!(empty.root, trellis::rfc6962::empty_root());
    client.append(&board, &[entry("a", 1, None)]).await?;
    let first = newest(&client, &board).await?;
    // Proofs are available as soon as the append commits.
    let proof = store
        .record_proof(&journal, LogScope::Board(&board), first.id, None)
        .await?;
    let old = proof.inclusion.checkpoint.clone();
    assert_eq!(old.tree_size, 1);
    proof.verify(&old)?;
    assert!(matches!(
        store
            .record_proof(
                &journal,
                LogScope::Board(&board),
                first.id + 1_000_000,
                None
            )
            .await,
        Err(JournalError::NotFound(_))
    ));
    let bad = vec![entry("rolled-back", 2, None), entry("", 3, None)];
    assert!(client.append(&board, &bad).await.is_err());
    assert_eq!(journal.checkpoint(&board).await?, old);
    assert_eq!(client.count(&board, &LogQuery::default()).await?, 1);
    journal.consistency(&empty).await?.verify(&empty)?;
    // The empty checkpoint anchors every record through a proof-less link.
    let from_empty = store
        .record_proof(&journal, LogScope::Board(&board), first.id, Some(&empty))
        .await?;
    from_empty.verify(&empty)?;
    assert!(from_empty.verify(&old).is_err());
    let mut changed = proof.entry.clone();
    changed.message.username = Some("changed".into());
    assert!(proof
        .inclusion
        .verify(&leaf_hash(&LogIdentity::of(&old), &changed)?, &old)
        .is_err());
    let second = entry("b", 1, None);
    let (a, b) = tokio::join!(
        client.append(&board, std::slice::from_ref(&second)),
        client.append(&board, std::slice::from_ref(&second))
    );
    a?;
    b?;
    let current = journal.checkpoint(&board).await?;
    assert_eq!(current.tree_size, 2);
    let consistency = journal.consistency(&old).await?;
    consistency.verify(&old)?;
    let mut wrong = old.clone();
    wrong.tree_size += 1;
    assert!(consistency.verify(&wrong).is_err());
    // Readers keep no state: a new instance answers identically.
    let other = store.journal();
    assert_eq!(other.checkpoint(&board).await?, current);
    other.consistency(&old).await?.verify(&old)?;
    store.delete_board(&board).await?;
    store.create_board(&board).await?;
    assert!(matches!(
        journal.consistency(&old).await,
        Err(JournalError::Diverged(_))
    ));
    assert_eq!(journal.checkpoint(&board).await?.tree_size, 0);
    store.delete_board(&board).await?;
    Ok(())
}

/// Dense positions under concurrency, large batches, historical proofs and forks.
#[tokio::test]
#[ignore = "requires ELECTORAL_LOG_TEST_DATABASE_URL"]
async fn trellis_concurrency_history_and_forks() -> Result<()> {
    use electoral_log::{
        adapters::postgres::AUDIT_LOCK_NAMESPACE,
        ports::ElectoralLogStore,
        proofs::{Journal, JournalError},
    };
    use sha2::{Digest, Sha256};
    let (store, board, mut db) = trellis_store().await?;
    let client = Arc::new(BoardClient::new(Arc::new(store.clone())));
    let tasks: Vec<_> = (0..32)
        .map(|i| {
            let (client, board) = (client.clone(), board.clone());
            tokio::spawn(async move {
                client
                    .append(&board, &[entry(&format!("concurrent-{i}"), i, None)])
                    .await
            })
        })
        .collect();
    for task in tasks {
        task.await??;
    }
    let journal = store.journal();
    let early = journal.checkpoint(&board).await?;
    assert_eq!(early.tree_size, 32);
    assert!(store.audit(&board, &[]).await?.is_clean());
    // Crosses the insert chunk size, so one append writes leaves in several batches.
    let bulk: Vec<_> = (0..5_003)
        .map(|i| entry(&format!("bulk-{i}"), i, None))
        .collect();
    client.append(&board, &bulk).await?;
    let trusted = journal.checkpoint(&board).await?;
    assert_eq!(trusted.tree_size, 5_035);
    let report = store
        .audit(&board, &[early.clone(), trusted.clone()])
        .await?;
    assert!(report.is_clean(), "{:?}", report.findings());
    assert_eq!(report.published_checked, 2);
    journal.consistency(&early).await?.verify(&early)?;

    let mut forged = trusted.clone();
    forged.root = vec![7; 32];
    let mut beyond = trusted.clone();
    beyond.tree_size += 10;
    let mut other_log = trusted.clone();
    other_log.log_uid = electoral_log::proofs::Uuid::new_v4();
    for checkpoint in [&forged, &beyond, &other_log] {
        assert!(matches!(
            journal.consistency(checkpoint).await,
            Err(JournalError::Diverged(_))
        ));
    }

    // A record added after the trusted checkpoint verifies against it in one bundle.
    client
        .append(&board, &[entry("after-trusted", 1, None)])
        .await?;
    let latest = newest(&client, &board).await?;
    let anchored = store
        .record_proof(&journal, LogScope::Board(&board), latest.id, Some(&trusted))
        .await?;
    assert!(anchored.consistency.is_some());
    anchored.verify(&trusted)?;
    let unanchored = store
        .record_proof(&journal, LogScope::Board(&board), latest.id, None)
        .await?;
    assert!(unanchored.verify(&trusted).is_err());
    // A record the trusted checkpoint already covers is proven at that checkpoint.
    let oldest = client
        .query(
            &board,
            &LogQuery {
                limit: 1,
                order: vec![(OrderColumn::Id, SortDirection::Asc)],
                ..LogQuery::default()
            },
        )
        .await?
        .remove(0);
    let historical = store
        .record_proof(&journal, LogScope::Board(&board), oldest.id, Some(&early))
        .await?;
    assert!(historical.consistency.is_none());
    assert_eq!(historical.inclusion.checkpoint, early);
    historical.verify(&early)?;
    let mut other_board = early.clone();
    other_board.log_name = format!("{board}-other");
    let report = store
        .audit(
            &board,
            &[
                forged.clone(),
                early.clone(),
                beyond.clone(),
                other_log.clone(),
                other_board,
            ],
        )
        .await?;
    assert_eq!(report.published_checked, 5);
    let sizes: Vec<u64> = report
        .published_mismatches
        .iter()
        .map(|mismatch| mismatch.tree_size)
        .collect();
    assert_eq!(
        sizes,
        vec![
            forged.tree_size,
            beyond.tree_size,
            trusted.tree_size,
            early.tree_size
        ]
    );
    assert!(report.published_mismatches[3]
        .reason
        .contains("another board"));
    assert_eq!(report.findings().len(), 4, "{:?}", report.findings());
    // Audits of a board run one at a time: a second one waits for the first.
    let lock = format!("{AUDIT_LOCK_NAMESPACE}{board}");
    db.execute("SELECT pg_advisory_lock(hashtextextended($1, 0))", &[&lock])
        .await?;
    let waiting = {
        let (store, board) = (store.clone(), board.clone());
        tokio::spawn(async move { store.audit(&board, &[]).await })
    };
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    assert!(!waiting.is_finished());
    db.execute(
        "SELECT pg_advisory_unlock(hashtextextended($1, 0))",
        &[&lock],
    )
    .await?;
    assert!(waiting.await??.is_clean());

    // An append that fails after its first leaf batch was written stores nothing.
    let before_failure = journal.checkpoint(&board).await?;
    let stored = client.count(&board, &LogQuery::default()).await?;
    let mut failing: Vec<_> = (0..5_003)
        .map(|i| entry(&format!("failing-{i}"), i, None))
        .collect();
    failing.push(entry("", 0, None));
    assert!(client.append(&board, &failing).await.is_err());
    assert_eq!(journal.checkpoint(&board).await?, before_failure);
    assert_eq!(client.count(&board, &LogQuery::default()).await?, stored);
    assert!(matches!(
        store
            .record_proof(&journal, LogScope::Board(&board), latest.id, Some(&forged))
            .await,
        Err(JournalError::Diverged(_))
    ));

    // One journal append larger than the insert chunk.
    let log_name = format!("{board}-journal");
    db.execute("INSERT INTO trellis_logs (name) VALUES ($1)", &[&log_name])
        .await?;
    // More leaves and completed subtrees than one insert chunk.
    let leaves: Vec<(i64, Vec<u8>)> = (0..6_007_i64)
        .map(|i| (i, Sha256::digest(i.to_le_bytes()).to_vec()))
        .collect();
    let tx = db.transaction().await?;
    Journal::append_batch(&tx, &log_name, &leaves).await?;
    tx.commit().await?;
    let mut frontier = trellis::rfc6962::Frontier::new();
    for (_, data) in &leaves {
        frontier.push(data);
    }
    let appended = journal.checkpoint(&log_name).await?;
    assert_eq!(appended.tree_size, 6_007);
    assert_eq!(appended.root, frontier.root().to_vec());
    let tree = trellis::journal::audit_tree(&db, &log_name, &[3, 6_007]).await?;
    assert!(tree.is_clean());
    assert_eq!(tree.roots[&6_007], appended.root);
    assert_eq!(tree.roots.len(), 2);
    db.execute("DELETE FROM trellis_logs WHERE name = $1", &[&log_name])
        .await?;
    store.delete_board(&board).await?;
    Ok(())
}

/// The audit reports changed subtrees, roots and records instead of failing; a rebuild
/// repairs subtrees but keeps only a root of the leaves; and a legacy log must be
/// rebuilt before it is read or extended.
#[tokio::test]
#[ignore = "requires ELECTORAL_LOG_TEST_DATABASE_URL"]
async fn trellis_audit_tampering_and_rebuild() -> Result<()> {
    use electoral_log::{ports::ElectoralLogStore, proofs::JournalError};
    let (store, board, db) = trellis_store().await?;
    let client = BoardClient::new(Arc::new(store.clone()));
    let journal = store.journal();
    let entries: Vec<_> = (0..4)
        .map(|i| entry(&format!("record-{i}"), i, Some("voter")))
        .collect();
    client.append(&board, &entries[..2]).await?;
    let early = journal.checkpoint(&board).await?;
    client.append(&board, &entries[2..]).await?;
    let honest = journal.checkpoint(&board).await?;
    let published = [early.clone(), honest.clone()];
    assert!(store.audit(&board, &published).await?.is_clean());
    let ids: Vec<i64> = db
        .query(
            "SELECT id FROM electoral_log_messages WHERE board_name = $1 ORDER BY id",
            &[&board],
        )
        .await?
        .iter()
        .map(|row| row.get(0))
        .collect();

    // A changed subtree breaks proofs that use it and is reported by the audit. The
    // published checkpoints still match the leaves.
    db.execute(
        "UPDATE trellis_nodes n SET hash = decode(repeat('ab', 32), 'hex') FROM trellis_logs l \
         WHERE l.id = n.log_id AND l.name = $1 AND n.level = 1 AND n.idx = 0",
        &[&board],
    )
    .await?;
    let report = store.audit(&board, &published).await?;
    assert_eq!(report.tree.node_mismatches, 1);
    assert_eq!(report.tree.first_node_mismatches, vec![(1, 0)]);
    assert!(report.published_mismatches.is_empty());
    assert_eq!(report.published_checked, 2);
    assert!(!report.findings().is_empty());
    assert!(matches!(
        store
            .record_proof(&journal, LogScope::Board(&board), ids[3], None)
            .await,
        Err(JournalError::Corrupt(_))
    ));
    // The damaged subtree also changes the root at size 2, which is reported as damage,
    // not as a checkpoint outside the history.
    assert!(matches!(
        journal.consistency(&early).await,
        Err(JournalError::Corrupt(_))
    ));
    // The root is unchanged, so a rebuild may repair the subtrees.
    assert_eq!(journal.rebuild(&board).await?, honest);
    assert!(store.audit(&board, &published).await?.is_clean());

    // A changed root makes the log unreadable and is reported by the audit.
    db.execute(
        "UPDATE trellis_logs SET root = decode(repeat('cd', 32), 'hex') WHERE name = $1",
        &[&board],
    )
    .await?;
    let report = store.audit(&board, &published).await?;
    assert!(!report.tree.root_matches);
    assert!(!report.is_clean());
    assert!(report.published_mismatches.is_empty());
    assert!(matches!(
        journal.checkpoint(&board).await,
        Err(JournalError::Corrupt(_))
    ));
    assert!(matches!(
        journal.consistency(&honest).await,
        Err(JournalError::Corrupt(_))
    ));
    // A rebuild never adopts a root that the leaves do not have.
    let refused = journal.rebuild(&board).await.unwrap_err().to_string();
    assert!(refused.contains("is not a root of its leaves"), "{refused}");
    db.execute(
        "UPDATE trellis_logs SET root = $2 WHERE name = $1",
        &[&board, &honest.root],
    )
    .await?;
    assert!(store.audit(&board, &published).await?.is_clean());

    // Record-level tampering: a changed, a deleted and an unlogged record.
    db.execute(
        "UPDATE electoral_log_messages SET username = 'tampered' WHERE id = $1",
        &[&ids[0]],
    )
    .await?;
    db.execute(
        "DELETE FROM electoral_log_messages WHERE id = $1",
        &[&ids[1]],
    )
    .await?;
    let unlogged: i64 = db
        .query_one(
            "INSERT INTO electoral_log_messages (board_name, delivery_id, created, sender_pk, \
             statement_timestamp, statement_kind, message, version) \
             VALUES ($1, 'unlogged', 0, 'sender', 0, 'CastVote', '\\x00', '2') RETURNING id",
            &[&board],
        )
        .await?
        .get(0);
    let report = store.audit(&board, &published).await?;
    assert!(!report.is_clean());
    assert_eq!(report.hash_mismatches, vec![ids[0]]);
    assert_eq!(report.leaves_without_message, vec![ids[1]]);
    assert_eq!(report.messages_without_leaf, vec![unlogged]);
    assert!(report.tree.is_clean());
    assert!(report.published_mismatches.is_empty());
    // A stored record without a leaf is an integrity fault, not an unknown record.
    assert!(matches!(
        store
            .record_proof(&journal, LogScope::Board(&board), unlogged, None)
            .await,
        Err(JournalError::Corrupt(_))
    ));

    // A log from before subtrees were stored has the empty root: it serves no
    // checkpoint and takes no append until it is rebuilt.
    db.execute(
        "UPDATE trellis_logs SET \
         root = decode('e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855', 'hex') \
         WHERE name = $1",
        &[&board],
    )
    .await?;
    db.execute(
        "DELETE FROM trellis_nodes n USING trellis_logs l WHERE l.id = n.log_id AND l.name = $1",
        &[&board],
    )
    .await?;
    let unread = journal.checkpoint(&board).await.unwrap_err().to_string();
    assert!(unread.contains("must be rebuilt"), "{unread}");
    let stored = client.count(&board, &LogQuery::default()).await?;
    let error = format!(
        "{:#}",
        client
            .append(&board, &[entry("before-rebuild", 9, None)])
            .await
            .unwrap_err()
    );
    assert!(error.contains("must be rebuilt"), "{error}");
    assert_eq!(client.count(&board, &LogQuery::default()).await?, stored);
    assert_eq!(journal.rebuild(&board).await?, honest);
    client
        .append(&board, &[entry("after-rebuild", 9, None)])
        .await?;
    let after = journal.checkpoint(&board).await?;
    assert_eq!(after.tree_size, 5);
    journal.consistency(&honest).await?.verify(&honest)?;

    // A log extended without updating its root and subtrees, as an older writer
    // would, is repaired by a rebuild: its stored root is still a root of its leaves.
    let log_id: i64 = db
        .query_one("SELECT id FROM trellis_logs WHERE name = $1", &[&board])
        .await?
        .get(0);
    db.execute(
        "INSERT INTO trellis_leaves (log_id, leaf_index, source_id, hash) \
         VALUES ($1, 5, -1, decode(repeat('ef', 32), 'hex'))",
        &[&log_id],
    )
    .await?;
    db.execute("UPDATE trellis_logs SET size = 6 WHERE id = $1", &[&log_id])
        .await?;
    assert!(matches!(
        journal.checkpoint(&board).await,
        Err(JournalError::Corrupt(_))
    ));
    assert_eq!(journal.rebuild(&board).await?.tree_size, 6);
    journal.consistency(&after).await?.verify(&after)?;
    store.delete_board(&board).await?;
    Ok(())
}

/// Leaves moved out of record order are reported even when the tree was recomputed to
/// match them.
#[tokio::test]
#[ignore = "requires ELECTORAL_LOG_TEST_DATABASE_URL"]
async fn trellis_audit_detects_reordered_leaves() -> Result<()> {
    use electoral_log::ports::ElectoralLogStore;
    let (store, board, db) = trellis_store().await?;
    let client = BoardClient::new(Arc::new(store.clone()));
    let journal = store.journal();
    let entries: Vec<_> = (0..3)
        .map(|i| entry(&format!("ordered-{i}"), i, None))
        .collect();
    client.append(&board, &entries).await?;
    let before = journal.checkpoint(&board).await?;
    assert!(store
        .audit(&board, std::slice::from_ref(&before))
        .await?
        .is_clean());
    let log_id: i64 = db
        .query_one("SELECT id FROM trellis_logs WHERE name = $1", &[&board])
        .await?
        .get(0);
    // Swap the first two leaves, record IDs and hashes together.
    db.execute(
        "UPDATE trellis_leaves SET source_id = -source_id \
         WHERE log_id = $1 AND leaf_index IN (0, 1)",
        &[&log_id],
    )
    .await?;
    db.execute(
        "UPDATE trellis_leaves l SET source_id = -o.source_id, hash = o.hash \
         FROM trellis_leaves o WHERE l.log_id = $1 AND o.log_id = $1 \
         AND l.leaf_index IN (0, 1) AND o.leaf_index = 1 - l.leaf_index",
        &[&log_id],
    )
    .await?;
    // Recompute a consistent tree over the reordered leaves, as a database owner could.
    db.execute(
        "UPDATE trellis_logs SET root = \
         decode('e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855', 'hex') \
         WHERE id = $1",
        &[&log_id],
    )
    .await?;
    db.execute("DELETE FROM trellis_nodes WHERE log_id = $1", &[&log_id])
        .await?;
    journal.rebuild(&board).await?;
    let report = store.audit(&board, std::slice::from_ref(&before)).await?;
    assert!(report.tree.is_clean());
    assert_eq!(report.published_mismatches.len(), 1);
    assert_eq!(report.hash_mismatch_count, 0);
    assert_eq!(report.leaves_out_of_order, 1);
    assert!(!report.is_clean());
    assert!(report
        .findings()
        .iter()
        .any(|finding| finding.contains("record order")));
    store.delete_board(&board).await?;
    Ok(())
}

#[tokio::test]
#[ignore = "requires ELECTORAL_LOG_TEST_DATABASE_URL"]
async fn unfiltered_counts_come_from_the_journal() -> Result<()> {
    let (store, board, db) = trellis_store().await?;
    let client = BoardClient::new(Arc::new(store.clone()));
    let entries: Vec<_> = (0..3)
        .map(|n| entry(&format!("delivery-{n}"), n, Some("user")))
        .collect();
    client.append(&board, &entries).await?;
    client.append(&board, &entries).await?;
    let everything = LogQuery {
        filters: vec![Filter::Number(
            NumberColumn::Created,
            NumberComparison::GreaterThanOrEqual,
            0,
        )],
        ..LogQuery::default()
    };
    assert_eq!(client.count(&board, &LogQuery::default()).await?, 3);
    assert_eq!(client.count(&board, &everything).await?, 3);
    // A row written around the journal is listed and audited, but not counted as a
    // committed record.
    db.execute(
        "INSERT INTO electoral_log_messages (board_name, delivery_id, created, sender_pk, \
         statement_timestamp, statement_kind, message, version) \
         VALUES ($1, 'outside', 0, 'sender', 0, 'CastVote', '\\x00', '1')",
        &[&board],
    )
    .await?;
    assert_eq!(client.count(&board, &LogQuery::default()).await?, 3);
    assert_eq!(client.count(&board, &everything).await?, 4);
    assert_eq!(client.query(&board, &LogQuery::default()).await?.len(), 4);
    assert!(!store.audit(&board, &[]).await?.is_clean());
    client.delete_board(&board).await?;
    assert_eq!(client.count(&board, &LogQuery::default()).await?, 0);
    assert_eq!(
        client.count("missing-board", &LogQuery::default()).await?,
        0
    );
    Ok(())
}

#[tokio::test]
#[ignore = "requires ELECTORAL_LOG_TEST_DATABASE_URL"]
async fn repeated_deliveries_in_one_append_store_the_first() -> Result<()> {
    let (store, board, _db) = trellis_store().await?;
    let client = BoardClient::new(Arc::new(store.clone()));
    client.append(&board, &[entry("kept", 1, None)]).await?;
    client
        .append(
            &board,
            &[
                entry("first", 2, Some("first")),
                entry("kept", 3, None),
                entry("first", 4, Some("second")),
                entry("last", 5, None),
            ],
        )
        .await?;
    let rows = client
        .query(
            &board,
            &LogQuery {
                order: vec![(OrderColumn::Id, SortDirection::Asc)],
                ..LogQuery::default()
            },
        )
        .await?;
    let created: Vec<_> = rows.iter().map(|row| row.created).collect();
    assert_eq!(created, vec![1, 2, 5]);
    assert_eq!(rows[1].user_id.as_deref(), Some("first"));
    // A repeat in a later chunk of the same append meets the copy stored by the first.
    let mut chunks: Vec<_> = (0..INSERT_CHUNK)
        .map(|n| entry(&format!("chunk-{n}"), 6, None))
        .collect();
    chunks.push(entry("chunk-0", 7, None));
    client.append(&board, &chunks).await?;
    let repeated = LogQuery {
        filters: vec![Filter::Number(
            NumberColumn::Created,
            NumberComparison::Equal,
            7,
        )],
        ..LogQuery::default()
    };
    assert_eq!(client.count(&board, &repeated).await?, 0);
    let records = 3 + i64::try_from(INSERT_CHUNK)?;
    assert_eq!(client.count(&board, &LogQuery::default()).await?, records);
    let report = store.audit(&board, &[]).await?;
    assert!(report.is_clean(), "{:?}", report.findings());
    assert_eq!(report.leaves, records);
    client.delete_board(&board).await?;
    Ok(())
}
