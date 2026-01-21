// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::area::{get_event_areas, insert_areas, upsert_area_parents};
use crate::postgres::area_contest::insert_area_contests;
use crate::postgres::contest::export_contests;
use crate::{
    postgres::document::get_document,
    services::{database::get_hasura_pool, documents::get_document_as_temp_file},
};
use anyhow::{anyhow, Context, Result};
use csv::StringRecord;
use deadpool_postgres::Client as DbClient;
use sequent_core::types::hasura::core::Area;
use sequent_core::types::hasura::core::AreaContest;
use std::collections::HashMap;
use std::io::Seek;
use tracing::instrument;
use uuid::Uuid;

#[instrument(err)]
pub async fn upsert_areas_task(
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

    let areas = get_event_areas(&hasura_transaction, &tenant_id, &election_event_id).await?;

    let areas_name_map: HashMap<String, Area> = areas
        .clone()
        .into_iter()
        .filter_map(|area| {
            if let Some(name) = area.name.clone() {
                Some((name, area.clone()))
            } else {
                None
            }
        })
        .collect();

    let mut temp_file = get_document_as_temp_file(&tenant_id, &document).await?;
    temp_file.rewind()?;
    // Read the first line of the file to get the columns
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b',')
        .has_headers(true)
        .from_reader(temp_file);

    let mut areas_to_modify: Vec<Area> = vec![];
    for result in rdr.records() {
        let record = result.with_context(|| "Error reading CSV record")?;
        let Some(area_name) = record.get(0) else {
            continue;
        };
        let Some(mut found_area_by_name) = areas_name_map.get(area_name) else {
            continue;
        };
        let parent_id: Option<String> = record
            .get(1)
            .map(|parent_name| areas_name_map.get(parent_name).map(|val| val.id.clone()))
            .flatten();
        let mut new_area = found_area_by_name.clone();
        new_area.parent_id = parent_id;
        areas_to_modify.push(new_area);
    }

    upsert_area_parents(&hasura_transaction, &areas_to_modify).await?;

    let _commit = hasura_transaction
        .commit()
        .await
        .map_err(|e| anyhow!("Commit failed: {}", e));

    Ok(())
}
