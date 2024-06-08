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
use tracing::instrument;
use uuid::Uuid;

#[instrument(err)]
pub async fn import_areas_task(
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

    // TODO: remove
    let contests = export_contests(&hasura_transaction, &tenant_id, &election_event_id).await?;

    let mut temp_file = get_document_as_temp_file(&tenant_id, &document).await?;
    temp_file.rewind()?;
    // Read the first line of the file to get the columns
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b',')
        .has_headers(false)
        .from_reader(temp_file);

    let headers = StringRecord::from(vec![
        "EMB_ID",
        "CTRY_CODE",
        "EMB_CODE",
        "AREANAME",
        "isdelete",
    ]);

    let mut areas: Vec<Area> = vec![];
    let mut area_contests: Vec<AreaContest> = vec![];

    for result in rdr.records() {
        let record = result.with_context(|| "Error reading CSV record")?;
        let isdelete = record.get(4).unwrap_or("1");
        // Don't import deleted records
        if "0" != isdelete {
            continue;
        }
        if let Some(area_id) = record.get(0) {
            let area_name = record.get(3).map(|val| val.to_string());
            let new_area_id = Uuid::new_v4();
            areas.push(Area {
                id: new_area_id.to_string(),
                tenant_id: tenant_id.to_string(),
                election_event_id: election_event_id.to_string(),
                created_at: None,
                last_updated_at: None,
                labels: None,
                annotations: None,
                name: Some(area_id.to_string()),
                description: area_name,
                r#type: None,
                parent_id: None,
            });
            let new_area_contests: Vec<AreaContest> = contests
                .clone()
                .into_iter()
                .map(|contest| -> Result<AreaContest> {
                    Ok(AreaContest {
                        id: Uuid::new_v4(),
                        area_id: new_area_id.clone(),
                        contest_id: Uuid::parse_str(&contest.id)?,
                    })
                })
                .collect::<Result<Vec<AreaContest>>>()?;
            area_contests.extend(new_area_contests);
        };
    }

    insert_areas(&hasura_transaction, &areas).await?;
    insert_area_contests(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        &area_contests,
    )
    .await?;

    let _commit = hasura_transaction
        .commit()
        .await
        .map_err(|e| anyhow!("Commit failed: {}", e));

    Ok(())
}
