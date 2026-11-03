// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Read-only access to restored local PostgreSQL copies and private output files,
//! shared by export-ballot-audit and audit-tally.

use crate::{digest, incomplete, invalid_input};
use anyhow::{bail, ensure, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    net::IpAddr,
    path::Path,
    time::Duration,
};
use tokio_postgres::{
    config::{Host, SslMode},
    types::ToSql,
    Client, Config, IsolationLevel, NoTls, Transaction,
};

const SNAPSHOT_SQL: &str = "SELECT jsonb_build_object(
    'database', current_database(),
    'snapshot', txid_current_snapshot()::text,
    'transaction_started_at', transaction_timestamp(),
    'server_version', current_setting('server_version'),
    'max_identifier_length', current_setting('max_identifier_length'),
    'read_only', current_setting('transaction_read_only'),
    'row_security', current_setting('row_security'),
    'isolation', current_setting('transaction_isolation')) AS data";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Connections {
    pub format_version: u32,
    pub hasura: ConnectionSource,
    pub keycloak: ConnectionSource,
    pub b3: ConnectionSource,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConnectionSource {
    pub connection_string: String,
    pub restore_reference: String,
}

/// Keeps only the outer message: source chains can echo credentials or row values.
pub fn into_invalid_input(error: anyhow::Error) -> anyhow::Error {
    invalid_input(error.to_string())
}

pub fn local_config(source: &ConnectionSource, application_name: &str) -> Result<Config> {
    ensure!(
        !source.restore_reference.trim().is_empty(),
        "restore reference is required"
    );
    let mut config: Config = source
        .connection_string
        .parse()
        .map_err(|_| anyhow::anyhow!("invalid database connection configuration"))?;
    ensure!(
        config.get_hosts().len() == 1,
        "exactly one explicit local database host is required"
    );
    ensure!(
        config.get_hostaddrs().is_empty(),
        "hostaddr overrides are not allowed"
    );
    ensure!(
        config.get_ports().len() <= 1,
        "multiple database ports are not allowed"
    );
    ensure!(
        config.get_options().is_none(),
        "custom database startup options are not allowed"
    );
    ensure!(
        config.get_dbname().is_some_and(|v| !v.is_empty()),
        "explicit database name is required"
    );
    ensure!(
        config.get_user().is_some_and(|v| !v.is_empty()),
        "explicit database user is required"
    );
    ensure!(
        config.get_ssl_mode() != SslMode::Require,
        "this local-only exporter does not support TLS-required connections"
    );
    match &config.get_hosts()[0] {
        Host::Tcp(host) => {
            let ip: IpAddr = host.parse().context("use a loopback IP, not a hostname")?;
            ensure!(ip.is_loopback(), "remote database connections are refused");
        }
        #[cfg(unix)]
        Host::Unix(path) => ensure!(path.is_absolute(), "Unix socket directory must be absolute"),
        #[allow(unreachable_patterns)]
        _ => bail!("unsupported local database host"),
    }
    config.ssl_mode(SslMode::Disable);
    config.connect_timeout(Duration::from_secs(10));
    config.application_name(application_name);
    config.options("-c default_transaction_read_only=on -c row_security=off");
    Ok(config)
}

pub async fn connect(config: Config) -> Result<Client> {
    let (client, connection) = config
        .connect(NoTls)
        .await
        .map_err(|_| invalid_input("local database connection failed"))?;
    tokio::spawn(async move {
        let _ = connection.await;
    });
    Ok(client)
}

pub fn snapshot_options(statement_timeout_seconds: u32) -> String {
    format!(
        "SET LOCAL statement_timeout = '{statement_timeout_seconds}s';
        SET LOCAL lock_timeout = '5s'; SET LOCAL row_security = off;
        SET LOCAL timezone = 'UTC'; SET LOCAL search_path = pg_catalog;"
    )
}

pub async fn start_snapshot(
    client: &mut Client,
    statement_timeout_seconds: u32,
) -> Result<Transaction<'_>> {
    let transaction = client
        .build_transaction()
        .isolation_level(IsolationLevel::RepeatableRead)
        .read_only(true)
        .start()
        .await
        .map_err(|_| invalid_input("cannot start read-only repeatable-read transaction"))?;
    transaction
        .batch_execute(&snapshot_options(statement_timeout_seconds))
        .await
        .map_err(|_| invalid_input("cannot set safe snapshot transaction options"))?;
    Ok(transaction)
}

pub async fn rows(
    transaction: &Transaction<'_>,
    sql: &str,
    params: &[&(dyn ToSql + Sync)],
) -> Result<Vec<Value>> {
    transaction
        .query(sql, params)
        .await
        .map_err(|_| {
            invalid_input("snapshot query failed; check restored schema and read permissions")
        })?
        .into_iter()
        .map(|row| {
            row.try_get("data")
                .map_err(|_| invalid_input("snapshot row has an unexpected schema"))
        })
        .collect()
}

pub fn single_row(mut values: Vec<Value>, what: &str) -> Result<Value> {
    match values.len() {
        1 => Ok(values.remove(0)),
        0 => Err(incomplete(format!(
            "expected exactly one {what} row, found none"
        ))),
        found => bail!("expected exactly one {what} row, found {found}"),
    }
}

pub async fn one(
    transaction: &Transaction<'_>,
    what: &str,
    sql: &str,
    params: &[&(dyn ToSql + Sync)],
) -> Result<Value> {
    single_row(rows(transaction, sql, params).await?, what)
}

pub async fn snapshot_metadata(
    transaction: &Transaction<'_>,
    source: &ConnectionSource,
) -> Result<Value> {
    let mut metadata = one(transaction, "snapshot metadata", SNAPSHOT_SQL, &[]).await?;
    ensure!(
        metadata["read_only"] == "on"
            && metadata["isolation"] == "repeatable read"
            && metadata["row_security"] == "off",
        invalid_input("database did not establish the required read-only repeatable-read snapshot")
    );
    metadata["restore_reference"] = json!(source.restore_reference);
    Ok(metadata)
}

pub fn board_relation_name(board: &str) -> Result<String> {
    ensure!(
        !board.is_empty()
            && board
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_')
            && board
                .bytes()
                .next()
                .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_'),
        "invalid B3 board SQL identifier"
    );
    // Native CREATE uses an unquoted name: PostgreSQL folds ASCII case and
    // truncates it to 63 bytes. The index and blob directory keep the full name.
    Ok(board[..board.len().min(63)].to_ascii_lowercase())
}

pub fn board_identifier(board: &str) -> Result<String> {
    Ok(format!("public.\"{}\"", board_relation_name(board)?))
}

pub fn read_connections(path: &Path) -> Result<Connections> {
    let metadata = fs::symlink_metadata(path).context("cannot inspect private connections file")?;
    ensure!(
        metadata.is_file() && !metadata.file_type().is_symlink(),
        "connections must be a regular private file"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        ensure!(
            metadata.permissions().mode() & 0o077 == 0,
            "connections file must not grant group or other access; use mode 0600"
        );
    }
    #[cfg(not(unix))]
    bail!("private connection-file permissions require a Unix platform");
    let bytes = fs::read(path).context("cannot read private connections file")?;
    let connections: Connections = serde_json::from_slice(&bytes)
        .map_err(|_| anyhow::anyhow!("invalid private connections JSON"))?;
    ensure!(
        connections.format_version == 1,
        "unsupported connections format version"
    );
    Ok(connections)
}

pub fn new_private_directory(path: &Path) -> Result<()> {
    let mut builder = fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    #[cfg(not(unix))]
    bail!("private output permissions require a Unix platform");
    builder.create(path).context(
        "cannot create new private output directory; existing paths are never overwritten",
    )?;
    Ok(())
}

pub fn save_bytes(directory: &Path, name: &str, bytes: &[u8]) -> Result<Value> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(directory.join(name))
        .map_err(|_| invalid_input("cannot create new private export file"))?;
    file.write_all(bytes)
        .map_err(|_| invalid_input("cannot write export file"))?;
    file.sync_all()
        .map_err(|_| invalid_input("cannot sync export file"))?;
    Ok(json!({"sha512":digest(bytes)?, "bytes":bytes.len()}))
}

pub fn save_json<T: Serialize>(directory: &Path, name: &str, value: &T) -> Result<Value> {
    save_bytes(directory, name, &serde_json::to_vec_pretty(value)?)
}
