// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
//! Read-only acquisition from a disposable, locally restored ImmuDB copy.
use anyhow::{ensure, Context, Result};
use ballot_audit::{invalid_input, ExitStatus};
use base64::{engine::general_purpose::STANDARD_NO_PAD, Engine};
use clap::Parser;
use electoral_log::messages::{
    message::Message,
    statement::{StatementBody, StatementType},
};
use immudb_rs::{sql_value::Value, Client, NamedParam, Row, SqlValue, TxMode};
use serde_json::json;
use std::{fs, io::Write, net::SocketAddr, path::PathBuf};
use strand::serialization::StrandDeserialize;
use tokio_stream::StreamExt;
use uuid::Uuid;

// Only CastVote statements are read; other kinds carry unrelated voter and
// administrator data that no audit command consumes.
const CAST_VOTE_SQL: &str = "SELECT id, message FROM electoral_log_messages
    WHERE statement_kind = @statement_kind AND id > @last_id ORDER BY id LIMIT 500;";
const ELECTION_CAST_VOTE_SQL: &str = "SELECT id, message FROM electoral_log_messages
    WHERE statement_kind = @statement_kind AND election_id = @election_id AND id > @last_id
    ORDER BY id LIMIT 500;";

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
    /// Election event whose CastVote statements are exported.
    #[arg(long)]
    election_event_id: String,
    /// Restrict the export to one election. Omit it for event-wide cast-vote history audits.
    #[arg(long)]
    election_id: Option<String>,
    /// New private output directory; existing paths are never overwritten.
    #[arg(long)]
    output: PathBuf,
}

#[derive(Debug)]
struct ScopedLog {
    messages: Vec<String>,
    other_event_messages: usize,
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

fn validate_uuid(value: &str) -> Result<()> {
    ensure!(
        Uuid::parse_str(value)
            .context("invalid scope UUID")?
            .to_string()
            == value,
        "scope UUIDs must use canonical lowercase spelling"
    );
    Ok(())
}

fn snapshot_query(election_id: Option<&str>) -> &'static str {
    match election_id {
        Some(_) => ELECTION_CAST_VOTE_SQL,
        None => CAST_VOTE_SQL,
    }
}

fn snapshot_params(last_id: i64, election_id: Option<&str>) -> Vec<NamedParam> {
    let param = |name: &str, value: Value| NamedParam {
        name: name.into(),
        value: Some(SqlValue { value: Some(value) }),
    };
    let mut params = vec![
        param("last_id", Value::N(last_id)),
        param(
            "statement_kind",
            Value::S(StatementType::CastVote.to_string()),
        ),
    ];
    if let Some(id) = election_id {
        params.push(param("election_id", Value::S(id.into())));
    }
    params
}

fn extract_row(row: &Row, previous: i64) -> Result<(i64, Vec<u8>)> {
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
        Some(Value::Bs(bytes)) if !bytes.is_empty() => bytes.clone(),
        _ => anyhow::bail!("missing electoral-log message bytes"),
    };
    Ok((id, bytes))
}

async fn read_snapshot(
    client: &mut Client,
    transaction: &String,
    election_id: Option<&str>,
) -> Result<Vec<Vec<u8>>> {
    let mut last = 0;
    let mut result = Vec::new();
    loop {
        let mut stream = client
            .tx_sql_query(
                snapshot_query(election_id),
                transaction,
                snapshot_params(last, election_id),
            )
            .await?
            .into_inner();
        let mut count = 0;
        while let Some(batch) = stream.next().await {
            for row in batch?.rows {
                let (id, bytes) = extract_row(&row, last)?;
                last = id;
                result.push(bytes);
                count += 1;
            }
        }
        if count == 0 {
            break;
        }
    }
    Ok(result)
}

/// The kind and election columns are unsigned row metadata; each selected row
/// must agree with its signed statement. Rows of another event are dropped.
fn scope_messages(
    rows: Vec<Vec<u8>>,
    election_event_id: &str,
    election_id: Option<&str>,
) -> Result<ScopedLog> {
    let mut messages = Vec::with_capacity(rows.len());
    let mut other_event_messages = 0;
    for bytes in rows {
        let message = Message::strand_deserialize(&bytes)
            .map_err(|_| invalid_input("invalid native electoral-log message bytes"))?;
        let election = match &message.statement.body {
            StatementBody::CastVote(election, ..)
            | StatementBody::CastVoteWithChannel(election, ..) => election,
            _ => anyhow::bail!("electoral-log CastVote row holds another signed statement"),
        };
        ensure!(
            matches!(message.statement.head.kind, StatementType::CastVote),
            "electoral-log CastVote kind/body mismatch"
        );
        if let Some(id) = election_id {
            ensure!(
                election.0.as_deref() == Some(id),
                "electoral-log election column differs from its signed statement"
            );
        }
        if message.statement.head.event.0 != election_event_id {
            other_event_messages += 1;
            continue;
        }
        messages.push(STANDARD_NO_PAD.encode(bytes));
    }
    Ok(ScopedLog {
        messages,
        other_event_messages,
    })
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
/// Keeps only the outer message: source chains can echo connection details.
fn into_invalid_input(error: anyhow::Error) -> anyhow::Error {
    invalid_input(error.to_string())
}

async fn run(args: Args) -> Result<()> {
    validate_endpoint(&args.server_url).map_err(into_invalid_input)?;
    validate_uuid(&args.election_event_id).map_err(into_invalid_input)?;
    if let Some(id) = &args.election_id {
        validate_uuid(id).map_err(into_invalid_input)?;
    }
    if args.database.is_empty() {
        return Err(invalid_input("database name is required"));
    }
    if args.output.exists() {
        return Err(invalid_input("output directory already exists"));
    }
    let password = std::env::var(&args.password_env)
        .map_err(|_| invalid_input("local ImmuDB password environment variable is missing"))?;
    // No tracing subscriber: existing client debug logs may contain messages/session data.
    let mut client = Client::new(&args.server_url, &args.username, &password)
        .await
        .map_err(|_| invalid_input("cannot connect to local restored ImmuDB"))?;
    client
        .open_session(&args.database)
        .await
        .map_err(|_| invalid_input("cannot open local restored database session"))?;
    let transaction = match client.new_tx(TxMode::ReadOnly).await {
        Ok(transaction) => transaction,
        Err(_) => {
            let _ = client.close_session().await;
            return Err(invalid_input("cannot start read-only ImmuDB transaction"));
        }
    };
    let election_id = args.election_id.as_deref();
    let result = read_snapshot(&mut client, &transaction, election_id).await;
    let rollback = client.rollback(&transaction).await;
    let close = client.close_session().await;
    let rows = result.map_err(|_| invalid_input("cannot read complete electoral-log snapshot"))?;
    rollback.map_err(|_| invalid_input("cannot close read-only transaction"))?;
    close.map_err(|_| invalid_input("cannot close local database session"))?;
    let scoped = scope_messages(rows, &args.election_event_id, election_id)?;
    let bytes = serde_json::to_vec_pretty(&scoped.messages)?;
    let provenance = json!({"format_version": 1, "source": "local_restored_immudb", "database": args.database, "read_only_transaction": true,
        "filter": {"statement_kind": StatementType::CastVote.to_string(), "election_event_id": args.election_event_id, "election_id": args.election_id},
        "message_count": scoped.messages.len(), "other_event_messages_excluded": scoped.other_event_messages,
        "electoral_log_sha512": ballot_audit::digest(&bytes)?, "limits": ["Acquisition checksum is not an independently authenticated historical anchor.", "Signed message verification is separate from ImmuDB transaction inclusion and consistency proofs.", "A local ImmuDB process and disposable restored copy are required; raw storage files are not read directly.", "This snapshot is not automatically consistent with PostgreSQL or frozen eligibility snapshots.", "Rows are selected by the unsigned statement_kind and election_id columns; a row whose column metadata was altered is not exported."]});
    let mut builder = fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder
        .create(&args.output)
        .map_err(|_| invalid_input("cannot create new private output directory"))?;
    write_private(&args.output.join("electoral-log.json"), &bytes)
        .map_err(|_| invalid_input("cannot write electoral-log export"))?;
    write_private(
        &args.output.join("electoral-log-provenance.json"),
        &serde_json::to_vec_pretty(&provenance)?,
    )
    .map_err(|_| invalid_input("cannot write electoral-log provenance"))?;
    println!(
        "Exported {} raw CastVote electoral-log message records from a local read-only snapshot.",
        scoped.messages.len()
    );
    Ok(())
}
#[tokio::main]
async fn main() {
    if let Err(error) = run(Args::parse()).await {
        eprintln!("Electoral-log export failed: {error}");
        std::process::exit(ExitStatus::of(&error).code());
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use electoral_log::messages::{
        message::SigningData,
        newtypes::{
            CastVoteErrorString, CastVoteHash, ElectionIdString, EventIdString, PseudonymHash,
            VoterCountryString, VoterIpString, VotingChannelString,
        },
    };
    use strand::{serialization::StrandSerialize, signature::StrandSignatureSk};

    const EVENT: &str = "00000000-0000-4000-8000-000000000020";
    const OTHER_EVENT: &str = "00000000-0000-4000-8000-000000000021";
    const ELECTION: &str = "00000000-0000-4000-8000-000000000030";

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

    fn signing() -> SigningData {
        let key = StrandSignatureSk::gen().unwrap();
        SigningData::new(key.clone(), "synthetic", key)
    }

    fn cast_vote(event: &str, election: &str) -> Vec<u8> {
        Message::cast_vote_with_channel_message(
            EventIdString(event.into()),
            ElectionIdString(Some(election.into())),
            PseudonymHash::new([1; 64]),
            CastVoteHash::new([2; 64]),
            &signing(),
            VoterIpString("".into()),
            VoterCountryString("".into()),
            VotingChannelString("ONLINE".into()),
            None,
            None,
            "area".into(),
        )
        .unwrap()
        .strand_serialize()
        .unwrap()
    }

    fn cast_vote_error(event: &str, election: &str) -> Vec<u8> {
        Message::cast_vote_error_message(
            EventIdString(event.into()),
            ElectionIdString(Some(election.into())),
            PseudonymHash::new([1; 64]),
            CastVoteErrorString("synthetic".into()),
            &signing(),
            VoterIpString("".into()),
            VoterCountryString("".into()),
            None,
            "area".into(),
        )
        .unwrap()
        .strand_serialize()
        .unwrap()
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
    fn scope_ids_must_be_canonical_uuids() {
        assert!(validate_uuid(EVENT).is_ok());
        let lettered = "0000000a-0000-4000-8000-00000000000b";
        assert!(validate_uuid(lettered).is_ok());
        for id in [
            "",
            "not-a-uuid",
            &lettered.to_uppercase(),
            &EVENT.replace('-', ""),
        ] {
            assert!(validate_uuid(id).is_err());
        }
    }

    #[test]
    fn query_selects_only_cast_votes_of_the_requested_election() {
        assert!(snapshot_query(None).contains("statement_kind = @statement_kind"));
        assert!(!snapshot_query(None).contains("election_id"));
        let filtered = snapshot_query(Some(ELECTION));
        assert!(filtered.contains("statement_kind = @statement_kind"));
        assert!(filtered.contains("election_id = @election_id"));
        let params = snapshot_params(7, Some(ELECTION));
        let value = |name: &str| {
            params
                .iter()
                .find(|param| param.name == name)
                .and_then(|param| param.value.clone())
                .and_then(|value| value.value)
        };
        assert_eq!(value("last_id"), Some(Value::N(7)));
        assert_eq!(value("statement_kind"), Some(Value::S("CastVote".into())));
        assert_eq!(value("election_id"), Some(Value::S(ELECTION.into())));
        assert_eq!(snapshot_params(0, None).len(), 2);
    }

    #[test]
    fn preserves_original_message_bytes() {
        assert_eq!(
            extract_row(&row(1, Value::Bs(vec![0, 255, 3])), 0).unwrap(),
            (1, vec![0, 255, 3])
        );
    }

    #[test]
    fn rejects_duplicate_reordered_or_empty_rows() {
        assert!(extract_row(&row(1, Value::Bs(vec![1])), 1).is_err());
        assert!(extract_row(&row(0, Value::Bs(vec![1])), 1).is_err());
        assert!(extract_row(&row(2, Value::Bs(vec![])), 1).is_err());
        assert!(extract_row(&row(2, Value::S("not bytes".into())), 1).is_err());
    }

    #[test]
    fn keeps_only_event_cast_votes_and_counts_other_events() {
        let own = cast_vote(EVENT, ELECTION);
        let scoped = scope_messages(
            vec![own.clone(), cast_vote(OTHER_EVENT, ELECTION)],
            EVENT,
            Some(ELECTION),
        )
        .unwrap();
        assert_eq!(scoped.messages, vec![STANDARD_NO_PAD.encode(own)]);
        assert_eq!(scoped.other_event_messages, 1);
    }

    #[test]
    fn row_metadata_contradicting_signed_statement_fails_closed() {
        let error =
            scope_messages(vec![cast_vote_error(EVENT, ELECTION)], EVENT, None).unwrap_err();
        assert_eq!(ExitStatus::of(&error), ExitStatus::VerificationFailed);
        let other_election = "00000000-0000-4000-8000-000000000031";
        let error = scope_messages(
            vec![cast_vote(EVENT, other_election)],
            EVENT,
            Some(ELECTION),
        )
        .unwrap_err();
        assert_eq!(ExitStatus::of(&error), ExitStatus::VerificationFailed);
        let error = scope_messages(vec![vec![0, 255, 3]], EVENT, None).unwrap_err();
        assert_eq!(ExitStatus::of(&error), ExitStatus::InvalidInput);
    }

    #[test]
    fn acquisition_errors_keep_only_the_outer_message() {
        let error = into_invalid_input(
            anyhow::anyhow!("secret-detail").context("cannot connect to local restored ImmuDB"),
        );
        assert_eq!(ExitStatus::of(&error), ExitStatus::InvalidInput);
        assert_eq!(error.to_string(), "cannot connect to local restored ImmuDB");
        assert!(!format!("{error:#}").contains("secret-detail"));
    }

    async fn create_table(client: &mut Client) -> Result<()> {
        client
            .sql_exec(
                "CREATE TABLE electoral_log_messages (id INTEGER, statement_kind VARCHAR[40],
                    election_id VARCHAR[64], message BLOB, PRIMARY KEY id);",
                vec![],
            )
            .await?;
        Ok(())
    }

    /// Requires a fresh disposable local ImmuDB on 127.0.0.1:43322.
    #[tokio::test]
    #[ignore = "requires disposable local ImmuDB; creates synthetic test rows"]
    async fn exports_multiple_pages_in_read_only_snapshot() -> Result<()> {
        let password = std::env::var("BALLOT_AUDIT_TEST_IMMUDB_PASSWORD")?;
        let mut client = Client::new("http://127.0.0.1:43322", "immudb", &password).await?;
        client.open_session("defaultdb").await?;
        create_table(&mut client).await?;
        for first in (1..=1001).step_by(100) {
            let values = (first..=(first + 99).min(1001))
                .map(|id| format!("({id}, 'CastVote', '{ELECTION}', x'00ff03')"))
                .collect::<Vec<_>>()
                .join(",");
            client
                .sql_exec(
                    &format!("INSERT INTO electoral_log_messages(id,statement_kind,election_id,message) VALUES {values};"),
                    vec![],
                )
                .await?;
        }
        client
            .sql_exec(
                &format!("INSERT INTO electoral_log_messages(id,statement_kind,election_id,message) VALUES
                    (1002, 'KeycloakUserEvent', '{ELECTION}', x'01'), (1003, 'CastVote', 'other', x'02');"),
                vec![],
            )
            .await?;
        let transaction = client.new_tx(TxMode::ReadOnly).await?;
        let rows = read_snapshot(&mut client, &transaction, Some(ELECTION)).await?;
        assert_eq!(rows.len(), 1001);
        assert!(rows.iter().all(|row| row == &vec![0, 255, 3]));
        assert_eq!(
            read_snapshot(&mut client, &transaction, None).await?.len(),
            1002
        );
        client
            .rollback(&transaction)
            .await
            .context("close successful read snapshot")?;
        let transaction = client.new_tx(TxMode::ReadOnly).await?;
        assert!(client
            .tx_sql_exec(
                "INSERT INTO electoral_log_messages(id,statement_kind,message) VALUES (1004,'CastVote',x'01');",
                &transaction,
                vec![]
            )
            .await
            .is_err());
        // ImmuDB aborts the transaction after the rejected write.
        let _ = client.rollback(&transaction).await;
        client.close_session().await?;
        Ok(())
    }

    /// Seed only synthetic signed fixtures on a fresh disposable test server.
    #[tokio::test]
    #[ignore = "requires disposable local ImmuDB and explicit synthetic fixture/output paths"]
    async fn export_signed_pipeline_fixture() -> Result<()> {
        let input = std::env::var("BALLOT_AUDIT_SIGNED_LOG_FIXTURE")?;
        let output = PathBuf::from(std::env::var("BALLOT_AUDIT_LOG_EXPORT_OUTPUT")?);
        let event = std::env::var("BALLOT_AUDIT_LOG_EXPORT_EVENT")?;
        let password = std::env::var("BALLOT_AUDIT_TEST_IMMUDB_PASSWORD")?;
        let encoded: Vec<String> = serde_json::from_slice(&fs::read(input)?)?;
        let mut client = Client::new("http://127.0.0.1:43322", "immudb", &password).await?;
        client.open_session("defaultdb").await?;
        create_table(&mut client).await?;
        let mut expected = Vec::new();
        for (index, message) in encoded.iter().enumerate() {
            let bytes = STANDARD_NO_PAD.decode(message)?;
            // Require native message structure; downstream preparation verifies signatures.
            let decoded = Message::strand_deserialize(&bytes)?;
            let kind = decoded.statement.head.kind.to_string();
            if matches!(decoded.statement.head.kind, StatementType::CastVote)
                && decoded.statement.head.event.0 == event
            {
                expected.push(message.clone());
            }
            let param = |name: &str, value: Value| NamedParam {
                name: name.into(),
                value: Some(SqlValue { value: Some(value) }),
            };
            let election = decoded.election_id.clone().map_or(Value::Null(0), Value::S);
            client
                .sql_exec(
                    "INSERT INTO electoral_log_messages(id,statement_kind,election_id,message)
                        VALUES (@id,@statement_kind,@election_id,@message);",
                    vec![
                        param("id", Value::N((index + 1).try_into()?)),
                        param("statement_kind", Value::S(kind)),
                        param("election_id", election),
                        param("message", Value::Bs(bytes)),
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
            election_event_id: event,
            election_id: None,
            output: output.clone(),
        })
        .await?;
        let actual: Vec<String> =
            serde_json::from_slice(&fs::read(output.join("electoral-log.json"))?)?;
        assert_eq!(actual, expected);
        let provenance: serde_json::Value =
            serde_json::from_slice(&fs::read(output.join("electoral-log-provenance.json"))?)?;
        assert_eq!(provenance["message_count"], expected.len());
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
        Ok(())
    }
}
