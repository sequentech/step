// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::election::get_elections;
use crate::postgres::protocol_board::get_event_protocol_boards;
use crate::services::electoral_log_board::{
    get_election_board, get_event_board, get_protocol_manager_secret_path,
};
use crate::services::protocol_board::{board_handle, manager_key_vault_path};
use crate::services::vault;
use anyhow::{anyhow, ensure, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use deadpool_postgres::Transaction;
use protocol_board::BoardName;
use regex::Regex;
use sequent_core::util::{aws::get_max_upload_size, temp_path::generate_temp_file};
use std::sync::LazyLock;
use tempfile::TempPath;

pub static HEADER_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-zA-Z0-9._-]+$").expect("Failed to build header regex"));
pub static ELECTION_ID_COL_NAME: &str = "election_id";
pub static ID_COL_NAME: &str = "id";
pub static CREATED_COL_NAME: &str = "created";
pub static SENDER_PK_COL_NAME: &str = "sender_pk";
pub static STATEMENT_TIMESTAMP_COL_NAME: &str = "statement_timestamp";
pub static STATEMENT_COL_NAME: &str = "statement_kind";
pub static MESSAGE_COL_NAME: &str = "message";
pub static VERSION_COL_NAME: &str = "version";

pub const PROTOCOL_ARCHIVE_HEADERS: [&str; 2] = ["board_name", "canonical_message"];

fn finish(writer: csv::Writer<tempfile::NamedTempFile>) -> Result<TempPath> {
    let path = writer.into_inner()?.into_temp_path();
    ensure!(
        path.metadata()?.len() <= get_max_upload_size()? as u64,
        "board archive exceeds upload size limit"
    );
    Ok(path)
}

pub async fn read_election_event_boards(
    transaction: &Transaction<'_>,
    tenant_id: &str,
    event_id: &str,
    include_tally: bool,
) -> Result<TempPath> {
    let mut writer = csv::Writer::from_writer(generate_temp_file("export-boards-", ".csv")?);
    writer.write_record(PROTOCOL_ARCHIVE_HEADERS)?;
    for row in get_event_protocol_boards(transaction, tenant_id, event_id)
        .await?
        .into_iter()
        .filter(|row| include_tally || row.parent_id.is_none())
    {
        let name = BoardName::parse(&row.name)?;
        for message in board_handle(&name)?.export_messages().await? {
            writer.write_record([name.to_string(), STANDARD.encode(message)])?;
        }
    }
    writer.flush()?;
    finish(writer)
}

pub async fn read_protocol_manager_keys(
    transaction: &Transaction<'_>,
    tenant_id: &str,
    event_id: &str,
    boards: &[sequent_core::types::protocol_board::ProtocolBoard],
) -> Result<TempPath> {
    let mut writer = csv::Writer::from_writer(generate_temp_file("export-protocol-keys-", ".csv")?);
    writer.write_record(["election_id", "key"])?;
    let slug = std::env::var("ENV_SLUG").context("missing ENV_SLUG")?;
    let mut identities = vec![(String::new(), get_event_board(tenant_id, event_id, &slug))];
    identities.extend(
        get_elections(transaction, tenant_id, event_id)
            .await?
            .into_iter()
            .map(|e| (e.id.clone(), get_election_board(tenant_id, &e.id, &slug))),
    );
    for (id, name) in identities {
        let key = vault::read_secret(
            transaction,
            tenant_id,
            Some(event_id),
            &get_protocol_manager_secret_path(&name),
        )
        .await?
        .ok_or_else(|| anyhow!("audit signing identity is missing"))?;
        writer.write_record([id, key])?;
    }
    // Native secrets travel only with their exported board metadata. Selective
    // publication/log exports still carry the audit identities above.
    for row in boards.iter().filter(|row| row.parent_id.is_none()) {
        ensure!(
            row.tenant_id == tenant_id && row.election_event_id == event_id,
            "protocol manager export scope mismatch"
        );
        let name = BoardName::parse(&row.name)?;
        let key = vault::read_secret(
            transaction,
            tenant_id,
            Some(event_id),
            &manager_key_vault_path(&name),
        )
        .await?
        .ok_or_else(|| anyhow!("protocol manager identity is missing"))?;
        writer.write_record([format!("protocol:{}", row.name), key])?;
    }
    writer.flush()?;
    finish(writer)
}
