// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
//! Read-only acquisition from a disposable, locally restored ImmuDB copy.
use anyhow::{ensure, Context, Result};
use base64::{engine::general_purpose::STANDARD_NO_PAD, Engine};
use clap::Parser;
use immudb_rs::{sql_value::Value, Client, NamedParam, Row, SqlValue, TxMode};
use serde_json::json;
use std::{fs, io::Write, net::SocketAddr, path::PathBuf};
use tokio_stream::StreamExt;
#[derive(Parser)]
struct Args {
    /// Loopback HTTP endpoint of a disposable restored ImmuDB, e.g. http://127.0.0.1:3322.
    #[arg(long)]
    server_url: String,
    /// Exact restored electoral-log database, established independently from ceremony records.
    #[arg(long)]
    database: String,
    #[arg(long)]
    username: String,
    /// Environment variable holding the local read-only account password, not the password itself.
    #[arg(long, default_value = "BALLOT_AUDIT_IMMUDB_PASSWORD")]
    password_env: String,
    /// New private output directory; existing paths are never overwritten.
    #[arg(long)]
    output: PathBuf,
}
fn validate_endpoint(value: &str) -> Result<()> {
    let address = value
        .strip_prefix("http://")
        .context("use a local HTTP endpoint")?;
    let address: SocketAddr = address
        .parse()
        .context("endpoint must be a literal loopback address and port")?;
    ensure!(
        address.ip().is_loopback() && address.port() != 0,
        "endpoint must be loopback with a nonzero port"
    );
    Ok(())
}
fn extract_row(row: &Row, previous: i64) -> Result<(i64, String)> {
    ensure!(
        row.values.len() == 2,
        "unexpected electoral-log result columns"
    );
    let id = match row.values[0].value {
        Some(Value::N(id)) => id,
        _ => anyhow::bail!("invalid electoral-log row ID"),
    };
    ensure!(
        id > previous,
        "electoral-log row IDs are not strictly increasing"
    );
    let bytes = match &row.values[1].value {
        Some(Value::Bs(bytes)) if !bytes.is_empty() => bytes,
        _ => anyhow::bail!("missing electoral-log message bytes"),
    };
    Ok((id, STANDARD_NO_PAD.encode(bytes)))
}
async fn read_snapshot(client: &mut Client, transaction: &String) -> Result<Vec<String>> {
    let mut last = 0;
    let mut result = Vec::new();
    loop {
        let params = vec![NamedParam {
            name: "last_id".into(),
            value: Some(SqlValue {
                value: Some(Value::N(last)),
            }),
        }];
        let mut stream = client.tx_sql_query("SELECT id, message FROM electoral_log_messages WHERE id > @last_id ORDER BY id LIMIT 500;", transaction, params).await?.into_inner();
        let mut count = 0;
        while let Some(batch) = stream.next().await {
            for row in batch?.rows {
                let (id, encoded) = extract_row(&row, last)?;
                last = id;
                result.push(encoded);
                count += 1;
            }
        }
        if count == 0 {
            break;
        }
    }
    Ok(result)
}
fn write_private(path: &std::path::Path, bytes: &[u8]) -> Result<()> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path)?.write_all(bytes)?;
    Ok(())
}
async fn run(args: Args) -> Result<()> {
    validate_endpoint(&args.server_url)?;
    ensure!(!args.database.is_empty(), "database name is required");
    ensure!(!args.output.exists(), "output directory already exists");
    let password = std::env::var(&args.password_env)
        .context("local ImmuDB password environment variable is missing")?;
    // No tracing subscriber: existing client debug logs may contain messages/session data.
    let mut client = Client::new(&args.server_url, &args.username, &password)
        .await
        .context("cannot connect to local restored ImmuDB")?;
    client
        .open_session(&args.database)
        .await
        .context("cannot open local restored database session")?;
    let transaction = match client.new_tx(TxMode::ReadOnly).await {
        Ok(transaction) => transaction,
        Err(error) => {
            let _ = client.close_session().await;
            return Err(error).context("cannot start read-only ImmuDB transaction");
        }
    };
    let result = read_snapshot(&mut client, &transaction).await;
    let rollback = client.rollback(&transaction).await;
    let close = client.close_session().await;
    let messages = result.context("cannot read complete electoral-log snapshot")?;
    rollback.context("cannot close read-only transaction")?;
    close.context("cannot close local database session")?;
    let bytes = serde_json::to_vec_pretty(&messages)?;
    let provenance = json!({"format_version": 1, "source": "local_restored_immudb", "database": args.database, "read_only_transaction": true, "message_count": messages.len(), "electoral_log_sha512": ballot_audit::digest(&bytes)?, "limits": ["Acquisition checksum is not an independently authenticated historical anchor.", "Signed message verification is separate from ImmuDB transaction inclusion and consistency proofs.", "A local ImmuDB process and disposable restored copy are required; raw storage files are not read directly.", "This snapshot is not automatically consistent with PostgreSQL or frozen eligibility snapshots."]});
    let mut builder = fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(&args.output)?;
    write_private(&args.output.join("electoral-log.json"), &bytes)?;
    write_private(
        &args.output.join("electoral-log-provenance.json"),
        &serde_json::to_vec_pretty(&provenance)?,
    )?;
    println!(
        "Exported {} raw electoral-log message records from a local read-only snapshot.",
        messages.len()
    );
    Ok(())
}
#[tokio::main]
async fn main() {
    if let Err(error) = run(Args::parse()).await {
        eprintln!("Electoral-log export failed: {error}");
        std::process::exit(1);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn row(id: i64, value: Value) -> Row {
        Row {
            columns: vec!["id".into(), "message".into()],
            values: vec![
                SqlValue {
                    value: Some(Value::N(id)),
                },
                SqlValue { value: Some(value) },
            ],
        }
    }
    #[test]
    fn accepts_only_literal_loopback_endpoints() {
        for endpoint in ["http://127.0.0.1:3322", "http://[::1]:3322"] {
            assert!(validate_endpoint(endpoint).is_ok());
        }
        for endpoint in [
            "https://127.0.0.1:3322",
            "http://example.com:3322",
            "http://localhost:3322",
            "http://10.0.0.1:3322",
            "http://127.0.0.1:0",
            "http://127.0.0.1:3322/path",
            "http://user:secret@127.0.0.1:3322",
        ] {
            assert!(validate_endpoint(endpoint).is_err());
        }
    }
    #[test]
    fn preserves_original_message_bytes() {
        assert_eq!(
            extract_row(&row(1, Value::Bs(vec![0, 255, 3])), 0).unwrap(),
            (1, "AP8D".into())
        );
    }
    #[test]
    fn rejects_duplicate_reordered_or_empty_rows() {
        assert!(extract_row(&row(1, Value::Bs(vec![1])), 1).is_err());
        assert!(extract_row(&row(0, Value::Bs(vec![1])), 1).is_err());
        assert!(extract_row(&row(2, Value::Bs(vec![])), 1).is_err());
        assert!(extract_row(&row(2, Value::S("not bytes".into())), 1).is_err());
    }
    /// Requires a fresh disposable local ImmuDB on 127.0.0.1:43322.
    #[tokio::test]
    #[ignore = "requires disposable local ImmuDB; creates synthetic test rows"]
    async fn exports_multiple_pages_in_read_only_snapshot() -> Result<()> {
        let password = std::env::var("BALLOT_AUDIT_TEST_IMMUDB_PASSWORD")?;
        let mut client = Client::new("http://127.0.0.1:43322", "immudb", &password).await?;
        client.open_session("defaultdb").await?;
        client
            .sql_exec(
                "CREATE TABLE electoral_log_messages (id INTEGER, message BLOB, PRIMARY KEY id);",
                vec![],
            )
            .await?;
        for first in (1..=1001).step_by(100) {
            let values = (first..=(first + 99).min(1001))
                .map(|id| format!("({id}, x'00ff03')"))
                .collect::<Vec<_>>()
                .join(",");
            client
                .sql_exec(
                    &format!("INSERT INTO electoral_log_messages(id,message) VALUES {values};"),
                    vec![],
                )
                .await?;
        }
        let transaction = client.new_tx(TxMode::ReadOnly).await?;
        let rows = read_snapshot(&mut client, &transaction).await?;
        assert_eq!(rows.len(), 1001);
        assert!(rows.iter().all(|row| row == "AP8D"));
        client
            .rollback(&transaction)
            .await
            .context("close successful read snapshot")?;
        let transaction = client.new_tx(TxMode::ReadOnly).await?;
        assert!(client
            .tx_sql_exec(
                "INSERT INTO electoral_log_messages(id,message) VALUES (1002,x'01');",
                &transaction,
                vec![]
            )
            .await
            .is_err());
        // ImmuDB aborts the transaction after the rejected write.
        let _ = client.rollback(&transaction).await;
        client.close_session().await?;
        let output =
            std::env::temp_dir().join(format!("ballot-audit-immudb-{}", std::process::id()));
        run(Args {
            server_url: "http://127.0.0.1:43322".into(),
            database: "defaultdb".into(),
            username: "immudb".into(),
            password_env: "BALLOT_AUDIT_TEST_IMMUDB_PASSWORD".into(),
            output: output.clone(),
        })
        .await?;
        let exported: Vec<String> =
            serde_json::from_slice(&fs::read(output.join("electoral-log.json"))?)?;
        assert_eq!(exported, rows);
        let provenance: serde_json::Value =
            serde_json::from_slice(&fs::read(output.join("electoral-log-provenance.json"))?)?;
        assert_eq!(provenance["message_count"], 1001);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(output.join("electoral-log.json"))?
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
        }
        fs::remove_dir_all(output)?;
        Ok(())
    }
    /// Seed only synthetic signed fixtures on a fresh disposable test server.
    #[tokio::test]
    #[ignore = "requires disposable local ImmuDB and explicit synthetic fixture/output paths"]
    async fn export_signed_pipeline_fixture() -> Result<()> {
        let input = std::env::var("BALLOT_AUDIT_SIGNED_LOG_FIXTURE")?;
        let output = PathBuf::from(std::env::var("BALLOT_AUDIT_LOG_EXPORT_OUTPUT")?);
        let password = std::env::var("BALLOT_AUDIT_TEST_IMMUDB_PASSWORD")?;
        let encoded: Vec<String> = serde_json::from_slice(&fs::read(input)?)?;
        let mut client = Client::new("http://127.0.0.1:43322", "immudb", &password).await?;
        client.open_session("defaultdb").await?;
        client
            .sql_exec(
                "CREATE TABLE electoral_log_messages (id INTEGER, message BLOB, PRIMARY KEY id);",
                vec![],
            )
            .await?;
        for (index, message) in encoded.iter().enumerate() {
            let bytes = STANDARD_NO_PAD.decode(message)?;
            // Require native message structure; downstream preparation verifies signatures.
            let _: electoral_log::messages::message::Message =
                strand::serialization::StrandDeserialize::strand_deserialize(&bytes)?;
            client
                .sql_exec(
                    "INSERT INTO electoral_log_messages(id,message) VALUES (@id,@message);",
                    vec![
                        NamedParam {
                            name: "id".into(),
                            value: Some(SqlValue {
                                value: Some(Value::N((index + 1).try_into()?)),
                            }),
                        },
                        NamedParam {
                            name: "message".into(),
                            value: Some(SqlValue {
                                value: Some(Value::Bs(bytes)),
                            }),
                        },
                    ],
                )
                .await?;
        }
        client.close_session().await?;
        run(Args {
            server_url: "http://127.0.0.1:43322".into(),
            database: "defaultdb".into(),
            username: "immudb".into(),
            password_env: "BALLOT_AUDIT_TEST_IMMUDB_PASSWORD".into(),
            output: output.clone(),
        })
        .await?;
        let actual: Vec<String> =
            serde_json::from_slice(&fs::read(output.join("electoral-log.json"))?)?;
        assert_eq!(actual, encoded);
        Ok(())
    }
}
