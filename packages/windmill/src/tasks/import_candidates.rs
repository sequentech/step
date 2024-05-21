// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::area::insert_areas;
use crate::postgres::area_contest::insert_area_contests;
use crate::postgres::contest::export_contests;
use crate::services::import_election_event::AreaContest;
use crate::{
    postgres::document::get_document,
    services::{database::get_hasura_pool, documents::get_document_as_temp_file},
};
use anyhow::{anyhow, Context, Result};
use csv::StringRecord;
use deadpool_postgres::Client as DbClient;
use sequent_core::types::hasura::core::Area;
use std::io::Seek;
use uuid::Uuid;

pub async fn import_candidates_task(
    tenant_id: String,
    election_event_id: String,
    document_id: String,
) -> Result<()> {
    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|err| anyhow!("Error getting hasura db pool: {err}"))?;

    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|err| anyhow!("Error starting hasura transaction: {err}"))?;

    let document = get_document(&hasura_transaction, &tenant_id, None, &document_id)
        .await
        .with_context(|| "Error obtaining the document")?
        .ok_or(anyhow!("document not found"))?;

    let _commit = hasura_transaction
        .commit()
        .await
        .map_err(|e| anyhow!("Commit failed: {}", e));

    Ok(())
}
