// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Read a collector's JSON deliveries from stdin and append each atomically.
use anyhow::{ensure, Context, Result};
use electoral_log::{client::database_audit::*, BoardClient};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::io::{self, BufRead};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Delivery {
    tenant_id: String,
    election_event_id: String,
    table: DatabaseAuditTable,
    /// Stable source identity: for example the log file identity and byte range.
    delivery_id: String,
    rows: Vec<DatabaseAuditRow>,
}
#[tokio::main]
async fn main() -> Result<()> {
    let url = std::env::var("WBRAID_AUDIT_DATABASE_URL")
        .context("WBRAID_AUDIT_DATABASE_URL must be set")?;
    let mut client = BoardClient::new(&url).await?;
    for line in io::stdin().lock().lines() {
        let delivery: Delivery = serde_json::from_str(&line?)?;
        ensure!(
            !delivery.delivery_id.is_empty(),
            "source delivery identity is empty"
        );
        let board = database_audit_board(
            &delivery.tenant_id,
            &delivery.election_event_id,
            delivery.table,
        )?;
        let id = hex::encode(Sha256::digest(delivery.delivery_id.as_bytes()));
        let rows: Vec<_> = delivery
            .rows
            .into_iter()
            .map(|row| row.into_record(delivery.table))
            .collect::<Result<_>>()?;
        let hash = hex::encode(Sha256::digest(serde_json::to_vec(&rows)?));
        client.upsert_electoral_log_db(&board).await?;
        client.append_delivery(&board, &id, &hash, &rows).await?;
        println!("{id}");
    }
    Ok(())
}
