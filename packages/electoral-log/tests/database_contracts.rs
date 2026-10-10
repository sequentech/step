// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

#[path = "support/immudb.rs"]
mod immudb;

use anyhow::{Context, Result};
use electoral_log::{
    ElectoralLogMessage, ElectoralLogVarCharColumn, SqlCompOperators, WhereClauseBTreeMap,
};
use immudb::{DatabaseServer, DATABASE};
use immudb_rs::TxMode;
use std::{collections::HashMap, time::Duration};

const DATABASE_TEST_TIMEOUT: Duration = Duration::from_secs(120);

#[tokio::test]
async fn a_conflicting_commit_retries_the_whole_batch_without_duplicate_audit_entries() -> Result<()>
{
    tokio::time::timeout(DATABASE_TEST_TIMEOUT, async {
        let server = DatabaseServer::start().await?;
        let mut reader = server.client().await?;
        reader.upsert_electoral_log_db(DATABASE).await?;
        let attempts = std::cell::Cell::new(0);
        let messages: Vec<_> = (0..3).map(message).collect();
        electoral_log::retry_electoral_log_transaction(|| {
            let attempt = attempts.get() + 1;
            attempts.set(attempt);
            let server = &server;
            let messages = &messages;
            async move {
                let mut writer = server.client().await?;
                writer.open_session(DATABASE).await?;
                let tx = writer.new_tx(TxMode::ReadWrite).await?;
                writer
                    .insert_electoral_log_messages_batch(&tx, messages)
                    .await?;
                if attempt == 1 {
                    // Commit a real competing insert after the batch has taken
                    // its snapshot. No timing or scheduler luck is involved.
                    let mut competitor = server.client().await?;
                    competitor
                        .insert_electoral_log_messages(DATABASE, &vec![message(99)])
                        .await?;
                }
                let result = writer.commit(&tx).await;
                writer.close_session().await?;
                result.context("committing test audit batch")
            }
        })
        .await?;
        assert_eq!(
            attempts.get(),
            2,
            "the fixture must force exactly one read conflict"
        );
        let rows = reader.get_electoral_log_messages(DATABASE).await?;
        assert_eq!(rows.len(), 4);
        let mut ballots: Vec<_> = rows
            .iter()
            .map(|row| row.ballot_id.as_deref().unwrap())
            .collect();
        ballots.sort_unstable();
        assert_eq!(ballots, ["ballot0", "ballot1", "ballot2", "ballot99"]);
        for input in &messages {
            let row = rows
                .iter()
                .find(|row| row.ballot_id == input.ballot_id)
                .unwrap();
            let mut expected = input.clone();
            expected.id = row.id;
            assert_eq!(
                row, &expected,
                "the retry must preserve the prepared audit message"
            );
        }
        Ok(())
    })
    .await
    .context("conflicting audit transaction timed out")?
}

fn message(index: i64) -> ElectoralLogMessage {
    ElectoralLogMessage {
        id: 0,
        created: 1_700_000_000_000_000 + index,
        statement_timestamp: 1_700_000_000_000_000 + index,
        sender_pk: "synthetic-public-key".into(),
        statement_kind: "CastVote".into(),
        message: vec![0, 127, 255],
        version: "2".into(),
        user_id: Some(format!("voter{index}")),
        username: Some("O'Brien; SELECT 'literal'".into()),
        election_id: Some("election".into()),
        area_id: Some("area".into()),
        ballot_id: Some(format!("ballot{index}")),
    }
}

#[tokio::test]
async fn database_lifecycle_and_both_pagination_strategies_preserve_all_metadata() -> Result<()> {
    tokio::time::timeout(DATABASE_TEST_TIMEOUT, async {
        let server = DatabaseServer::start().await?;
        let mut client = server.client().await?;
        assert!(!client.has_database(DATABASE).await?);
        client.upsert_electoral_log_db(DATABASE).await?;
        client.upsert_electoral_log_db(DATABASE).await?;

        // Cross the reader's 900-row boundary. Small write transactions match the
        // service's ordinary batches and stay below ImmuDB's transaction limit.
        let inputs: Vec<_> = (0..903).map(message).collect();
        for batch in inputs.chunks(40) {
            client
                .insert_electoral_log_messages(DATABASE, &batch.to_vec())
                .await?;
        }
        assert_eq!(
            client.count_electoral_log_messages(DATABASE, None).await?,
            903
        );

        let all = client.get_electoral_log_messages(DATABASE).await?;
        assert_eq!(all.len(), inputs.len());
        for (index, actual) in all.iter().enumerate() {
            let mut expected = inputs[index].clone();
            expected.id = (index + 1) as i64;
            assert_eq!(*actual, expected, "full reader changed row {index}");
        }

        let mut offset_rows = Vec::new();
        for offset in [0, 900, 1800] {
            offset_rows.extend(
                client
                    .get_electoral_log_messages_at_offset(DATABASE, 900, offset)
                    .await?,
            );
        }
        let expected_ids: Vec<i64> = (1..=903).collect();
        assert_eq!(
            offset_rows.iter().map(|row| row.id).collect::<Vec<_>>(),
            expected_ids
        );

        let first_page = client
            .get_electoral_log_messages_batch(DATABASE, 4, 0)
            .await?;
        assert_eq!(first_page, all[..4]);
        let next_page = client
            .get_electoral_log_messages_batch(DATABASE, 4, first_page[3].id)
            .await?;
        assert_eq!(next_page, all[4..8]);
        assert_eq!(
            client
                .get_electoral_log_messages_at_offset(DATABASE, 4, 4)
                .await?,
            all[4..8]
        );
        assert!(client
            .get_electoral_log_messages_batch(DATABASE, 4, 903)
            .await?
            .is_empty());

        // The explicit session API is used by the batch writers. Its insertion
        // path must preserve nullable metadata as well as the ordinary wrapper.
        client.open_session(DATABASE).await?;
        let transaction = client.new_tx(TxMode::ReadWrite).await?;
        let mut anonymous = message(904);
        anonymous.user_id = None;
        anonymous.username = None;
        anonymous.election_id = None;
        anonymous.area_id = None;
        anonymous.ballot_id = None;
        client
            .insert_electoral_log_messages_batch(&transaction, &[anonymous.clone()])
            .await?;
        client.commit(&transaction).await?;
        client.close_session().await?;
        anonymous.id = 904;
        assert_eq!(
            client
                .get_electoral_log_messages_batch(DATABASE, 4, 903)
                .await?,
            vec![anonymous]
        );

        client.delete_database(DATABASE).await?;
        client.delete_database(DATABASE).await?;
        assert!(!client.has_database(DATABASE).await?);
        Ok(())
    })
    .await
    .context("local ImmuDB integration exceeded two minutes")?
}

#[tokio::test]
async fn filters_bind_literal_values_and_accept_epoch_zero_and_multiple_sort_columns() -> Result<()>
{
    tokio::time::timeout(DATABASE_TEST_TIMEOUT, async {
        let server = DatabaseServer::start().await?;
        let mut client = server.client().await?;
        client.upsert_electoral_log_db(DATABASE).await?;
        let mut epoch = message(0);
        epoch.created = 0;
        client
            .insert_electoral_log_messages(DATABASE, &vec![epoch, message(1), message(2)])
            .await?;

        let at_epoch = client
            .get_electoral_log_messages_filtered::<String, String>(
                DATABASE,
                None,
                None,
                Some(0),
                None,
                None,
                None,
            )
            .await?;
        assert_eq!(
            at_epoch.iter().map(|row| row.id).collect::<Vec<_>>(),
            vec![1]
        );

        let filter = WhereClauseBTreeMap::from([
            (
                ElectoralLogVarCharColumn::StatementKind,
                (SqlCompOperators::Equal, "CastVote".into()),
            ),
            (
                ElectoralLogVarCharColumn::Username,
                (SqlCompOperators::Equal, "O'Brien; SELECT 'literal'".into()),
            ),
        ]);
        assert_eq!(
            client
                .count_electoral_log_messages(DATABASE, Some(filter.clone()))
                .await?,
            3
        );
        let rows = client
            .get_electoral_log_messages_filtered::<String, String>(
                DATABASE,
                Some(filter),
                Some(0),
                Some(1_700_000_000_000_001),
                Some(10),
                Some(0),
                Some(HashMap::from([
                    ("created".into(), "ASC".into()),
                    ("id".into(), "ASC".into()),
                ])),
            )
            .await?;
        assert_eq!(
            rows.iter().map(|row| row.id).collect::<Vec<_>>(),
            vec![1, 2]
        );

        let empty_filter = WhereClauseBTreeMap::from([(
            ElectoralLogVarCharColumn::UserId,
            (SqlCompOperators::Equal, "' OR 1=1 --".into()),
        )]);
        assert_eq!(
            client
                .count_electoral_log_messages(DATABASE, Some(empty_filter))
                .await?,
            0
        );
        Ok(())
    })
    .await
    .context("local ImmuDB integration exceeded two minutes")?
}

#[tokio::test]
async fn a_failed_write_never_commits_an_earlier_row_from_the_same_batch() -> Result<()> {
    tokio::time::timeout(DATABASE_TEST_TIMEOUT, async {
        let server = DatabaseServer::start().await?;
        let mut client = server.client().await?;
        client.upsert_electoral_log_db(DATABASE).await?;
        let mut invalid = message(1);
        invalid.statement_kind = "x".repeat(41); // The schema caps this field at 40 bytes.

        assert!(client
            .insert_electoral_log_messages(DATABASE, &vec![message(0), invalid])
            .await
            .is_err());
        assert_eq!(
            client.count_electoral_log_messages(DATABASE, None).await?,
            0
        );

        // Failure must close the session so the same client can perform the next
        // independent operation without carrying a half-failed transaction.
        client
            .insert_electoral_log_messages(DATABASE, &vec![message(2)])
            .await?;
        assert_eq!(
            client.count_electoral_log_messages(DATABASE, None).await?,
            1
        );
        Ok(())
    })
    .await
    .context("local ImmuDB integration exceeded two minutes")?
}

#[tokio::test]
async fn helper_binary_uses_explicit_and_environment_configuration() -> Result<()> {
    tokio::time::timeout(DATABASE_TEST_TIMEOUT, async {
        for use_environment in [false, true] {
            // Deleted ImmuDB database names cannot be reused in this server
            // version. Each configuration mode gets its own server lifecycle.
            let server = DatabaseServer::start().await?;
            let mut client = server.client().await?;
            let mut command = tokio::process::Command::new(env!("CARGO_BIN_EXE_bb_helper"));
            command.kill_on_drop(true);
            for variable in [
                "IMMUDB_SERVER_URL",
                "IMMUDB_BOARD_DBNAME",
                "IMMUDB_USERNAME",
                "IMMUDB_PASSWORD",
            ] {
                command.env_remove(variable);
            }
            command.args(["--cache-dir", "/unused", "--log-level", "off"]);
            if use_environment {
                command
                    .env("IMMUDB_SERVER_URL", &server.url)
                    .env("IMMUDB_BOARD_DBNAME", DATABASE)
                    .env("IMMUDB_USERNAME", immudb::USERNAME)
                    .env("IMMUDB_PASSWORD", server.password.as_str());
            } else {
                command.args([
                    "--server-url",
                    &server.url,
                    "--board-dbname",
                    DATABASE,
                    "--username",
                    immudb::USERNAME,
                    "--password",
                    server.password.as_str(),
                ]);
            }
            let output = command
                .args(["upsert-board-db", "delete-board-db"])
                .output()
                .await?;
            assert!(output.status.success(), "{output:?}");
            assert!(!client.has_database(DATABASE).await?);
        }
        Ok(())
    })
    .await
    .context("local ImmuDB integration exceeded two minutes")?
}

#[tokio::test]
async fn tied_timestamps_have_stable_offset_pages_and_respect_explicit_id_order() -> Result<()> {
    tokio::time::timeout(DATABASE_TEST_TIMEOUT, async {
        let server = DatabaseServer::start().await?;
        let mut client = server.client().await?;
        client.upsert_electoral_log_db(DATABASE).await?;
        let inputs = vec![message(2), message(1), message(2), message(1)];
        client
            .insert_electoral_log_messages(DATABASE, &inputs)
            .await?;
        // IDs follow insertion order, not timestamp order. Each two-row page
        // has a tie, so the independent expected IDs distinguish both keys.
        for (offset, expected) in [(0, vec![2, 4]), (2, vec![1, 3])] {
            let rows = client
                .get_electoral_log_messages_filtered::<String, String>(
                    DATABASE,
                    None,
                    None,
                    None,
                    Some(2),
                    Some(offset),
                    Some(HashMap::from([("created".into(), "ASC".into())])),
                )
                .await?;
            assert_eq!(rows.iter().map(|row| row.id).collect::<Vec<_>>(), expected);
        }
        let rows = client
            .get_electoral_log_messages_filtered::<String, String>(
                DATABASE,
                None,
                None,
                None,
                Some(4),
                Some(0),
                Some(HashMap::from([
                    ("created".into(), "ASC".into()),
                    ("id".into(), "DESC".into()),
                ])),
            )
            .await?;
        assert_eq!(
            rows.iter().map(|row| row.id).collect::<Vec<_>>(),
            vec![4, 2, 3, 1]
        );
        Ok(())
    })
    .await
    .context("local ImmuDB integration exceeded two minutes")?
}

#[tokio::test]
async fn a_competing_listener_causes_a_bounded_retry_on_a_new_port() -> Result<()> {
    let occupied = std::net::TcpListener::bind(("127.0.0.1", 0))?;
    let port = occupied.local_addr()?.port();
    let server = tokio::time::timeout(
        DATABASE_TEST_TIMEOUT,
        DatabaseServer::start_with_first_port(Some(port)),
    )
    .await??;
    assert_ne!(server.url, format!("http://127.0.0.1:{port}"));
    let mut client = server.client().await?;
    client.upsert_electoral_log_db(DATABASE).await?;
    assert!(client.has_database(DATABASE).await?);
    Ok(())
}

#[tokio::test]
async fn explicit_blob_sort_uses_lexicographic_bytes() -> Result<()> {
    tokio::time::timeout(DATABASE_TEST_TIMEOUT, async {
        let server = DatabaseServer::start().await?;
        let mut client = server.client().await?;
        client.upsert_electoral_log_db(DATABASE).await?;
        let mut inputs = vec![message(0), message(1), message(2)];
        inputs[0].message = vec![255];
        inputs[1].message = vec![0, 1];
        inputs[2].message = vec![0];
        client
            .insert_electoral_log_messages(DATABASE, &inputs)
            .await?;
        let rows = client
            .get_electoral_log_messages_filtered::<String, String>(
                DATABASE,
                None,
                None,
                None,
                Some(3),
                None,
                Some(HashMap::from([("message".into(), "ASC".into())])),
            )
            .await?;
        // Payload order differs from both insertion order and timestamp order.
        assert_eq!(rows.iter().map(|row| row.id).collect::<Vec<_>>(), [3, 2, 1]);
        Ok(())
    })
    .await
    .context("local ImmuDB blob sort exceeded two minutes")?
}

// A delivery receipt and its rows must commit together, including a delivery
// that expands into multiple signed audit messages.
#[tokio::test]
async fn redelivery_after_a_lost_commit_response_keeps_exactly_one_copy() -> Result<()> {
    tokio::time::timeout(DATABASE_TEST_TIMEOUT, async {
        let server = DatabaseServer::start().await?;
        let mut reader = server.client().await?;
        reader.upsert_electoral_log_db(DATABASE).await?;
        let receipt = "a".repeat(64);
        let payload = "b".repeat(64);
        for attempt in 0..2 {
            let mut writer = server.client().await?;
            writer.open_session(DATABASE).await?;
            let tx = writer.new_tx(TxMode::ReadWrite).await?;
            // Regenerated Keycloak signatures/timestamps need not be identical:
            // the immutable input identity, not regenerated bytes, deduplicates.
            let inserted = writer
                .insert_electoral_log_delivery(
                    &tx,
                    &receipt,
                    &payload,
                    &[message(attempt * 10), message(attempt * 10 + 1)],
                )
                .await?;
            writer.commit(&tx).await?;
            writer.close_session().await?;
            if attempt == 0 {
                assert!(inserted);
                // Replay below deliberately disregards this confirmed outcome,
                // as a caller with a lost response would have to do.
            } else {
                assert!(!inserted, "redelivery must observe the committed receipt");
            }
        }
        let rows = reader.get_electoral_log_messages(DATABASE).await?;
        assert_eq!(rows.len(), 2);
        assert_eq!(
            rows.iter()
                .map(|row| row.ballot_id.as_deref().unwrap())
                .collect::<Vec<_>>(),
            ["ballot0", "ballot1"]
        );
        Ok(())
    })
    .await
    .context("receipt redelivery timed out")?
}

#[tokio::test]
async fn rolled_back_delivery_can_retry_but_reused_id_cannot_replace_committed_input() -> Result<()>
{
    tokio::time::timeout(DATABASE_TEST_TIMEOUT, async {
        let server = DatabaseServer::start().await?;
        let mut reader = server.client().await?;
        reader.upsert_electoral_log_db(DATABASE).await?;
        let id = "c".repeat(64);
        let payload = "d".repeat(64);
        let mut writer = server.client().await?;
        writer.open_session(DATABASE).await?;
        let tx = writer.new_tx(TxMode::ReadWrite).await?;
        assert!(
            writer
                .insert_electoral_log_delivery(&tx, &id, &payload, &[message(1)])
                .await?
        );
        // Closing an uncommitted session must not leave a receipt without rows.
        writer.close_session().await?;
        assert!(reader
            .get_electoral_log_messages(DATABASE)
            .await?
            .is_empty());
        let mut writer = server.client().await?;
        writer.open_session(DATABASE).await?;
        let tx = writer.new_tx(TxMode::ReadWrite).await?;
        assert!(
            writer
                .insert_electoral_log_delivery(&tx, &id, &payload, &[message(1)])
                .await?
        );
        writer.commit(&tx).await?;
        writer.close_session().await?;
        let mut writer = server.client().await?;
        writer.open_session(DATABASE).await?;
        let tx = writer.new_tx(TxMode::ReadWrite).await?;
        let error = writer
            .insert_electoral_log_delivery(&tx, &id, &"e".repeat(64), &[message(2)])
            .await
            .unwrap_err();
        assert_eq!(
            error.to_string(),
            "electoral log delivery ID reused with different input"
        );
        writer.close_session().await?;
        let rows = reader.get_electoral_log_messages(DATABASE).await?;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].ballot_id.as_deref(), Some("ballot1"));
        Ok(())
    })
    .await
    .context("rolled-back receipt test timed out")?
}

#[tokio::test]
async fn concurrent_redeliveries_resolve_to_one_committed_receipt_and_message() -> Result<()> {
    tokio::time::timeout(DATABASE_TEST_TIMEOUT, async {
        let server = DatabaseServer::start().await?;
        let mut reader = server.client().await?;
        reader.upsert_electoral_log_db(DATABASE).await?;
        let id = "f".repeat(64);
        let payload = "1".repeat(64);
        let attempts = std::cell::Cell::new(0);
        electoral_log::retry_electoral_log_transaction(|| {
            let server = &server;
            let id = &id;
            let payload = &payload;
            let attempt = attempts.get() + 1;
            attempts.set(attempt);
            async move {
                let mut writer = server.client().await?;
                writer.open_session(DATABASE).await?;
                let tx = writer.new_tx(TxMode::ReadWrite).await?;
                let inserted = writer
                    .insert_electoral_log_delivery(&tx, id, payload, &[message(1)])
                    .await?;
                if attempt == 1 {
                    let mut competitor = server.client().await?;
                    competitor.open_session(DATABASE).await?;
                    let competing_tx = competitor.new_tx(TxMode::ReadWrite).await?;
                    assert!(
                        competitor
                            .insert_electoral_log_delivery(
                                &competing_tx,
                                id,
                                payload,
                                &[message(1)]
                            )
                            .await?
                    );
                    competitor.commit(&competing_tx).await?;
                    competitor.close_session().await?;
                } else {
                    assert!(!inserted, "fresh snapshot must see the winning receipt");
                }
                let result = writer.commit(&tx).await;
                writer.close_session().await?;
                result.context("committing competing delivery")
            }
        })
        .await?;
        assert_eq!(attempts.get(), 2);
        let rows = reader.get_electoral_log_messages(DATABASE).await?;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].ballot_id.as_deref(), Some("ballot1"));
        Ok(())
    })
    .await
    .context("concurrent receipt test timed out")?
}
