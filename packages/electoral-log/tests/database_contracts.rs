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
        client
            .insert_electoral_log_messages(DATABASE, &vec![message(0), message(1), message(2)])
            .await?;

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
            command.args(["--cache-dir", "/unused", "--log-level", "off"]);
            if use_environment {
                command
                    .env("IMMUDB_SERVER_URL", &server.url)
                    .env("IMMUDB_BOARD_DBNAME", DATABASE)
                    .env("IMMUDB_USERNAME", immudb::USERNAME)
                    .env("IMMUDB_PASSWORD", immudb::PASSWORD);
            } else {
                command.args([
                    "--server-url",
                    &server.url,
                    "--board-dbname",
                    DATABASE,
                    "--username",
                    immudb::USERNAME,
                    "--password",
                    immudb::PASSWORD,
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
