// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::{Client as DbClient, Pool, PoolError, Runtime, Transaction};
use std::convert::From;
use tracing::{event, instrument, Level};
use uuid::Uuid;

pub async fn publish_tally_sheet(
    transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    tally_sheet_id: &str,
) -> Result<()> {
    Ok(())
}
