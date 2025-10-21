// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::area::insert_areas;
use crate::postgres::area_contest::insert_area_contests;
use crate::postgres::contest::export_contests;
use crate::{postgres::document::get_document, services::documents::get_document_as_temp_file};
use anyhow::{anyhow, Context, Result};
use csv::StringRecord;
use deadpool_postgres::Transaction;
use sequent_core::ballot::{AreaPresentation, EarlyVotingPolicy};
use sequent_core::types::hasura::core::Area;
use sequent_core::types::hasura::core::AreaContest;
use sequent_core::util::integrity_check::{integrity_check, HashFileVerifyError};
use std::io::Seek;
use tracing::{error, info, instrument};
use uuid::Uuid;

#[instrument(err)]
pub async fn import_areas_task(
    hasura_transaction: &Transaction<'_>,
    tenant_id: String,
    election_event_id: String,
    document_id: String,
    sha256: Option<String>,
) -> Result<()> {
    let document = get_document(hasura_transaction, &tenant_id, None, &document_id)
        .await
        .with_context(|| "Error obtaining the document")?
        .ok_or(anyhow!("document not found"))?;

    // TODO: remove
    let contests = export_contests(hasura_transaction, &tenant_id, &election_event_id).await?;

    let mut temp_file = get_document_as_temp_file(&tenant_id, &document).await?;
    temp_file.rewind()?;

    match sha256 {
        Some(hash) if !hash.is_empty() => match integrity_check(&temp_file, hash) {
            Ok(_) => {
                info!("Hash verified !");
            }
            Err(HashFileVerifyError::HashMismatch(input_hash, gen_hash)) => {
                let err_str = format!("Failed to verify the integrity: Hash of voters file: {gen_hash} does not match with the input hash: {input_hash}");
                return Err(anyhow!(err_str));
            }
            Err(err) => {
                let err_str = format!("Failed to verify the integrity: {err:?}");
                error!("{err_str}");
                return Err(anyhow!(err_str));
            }
        },
        _ => {
            info!("No hash provided, skipping integrity check");
        }
    }

    // Read the first line of the file to get the columns
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b',')
        .has_headers(false)
        .from_reader(temp_file);

    // For reference:
    let _headers = StringRecord::from(vec![
        "EMB_ID",
        "CTRY_CODE",
        "EMB_CODE",
        "AREANAME",
        "isdelete",
        "EARLY_VOTING_POLICY",
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
            let early_voting_pol = record.get(5).map(|val| val.to_string());
            let early_voting_pol = match early_voting_pol {
                Some(early_voting_pol)
                    if early_voting_pol == EarlyVotingPolicy::AllowEarlyVoting.to_string() =>
                {
                    EarlyVotingPolicy::AllowEarlyVoting
                }
                _ => EarlyVotingPolicy::default(),
            };
            let presentation = serde_json::to_value(AreaPresentation {
                allow_early_voting: Some(early_voting_pol),
            })
            .map_err(|e| anyhow!("Error serializing AreaPresentation: {e:?}"))?;
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
                presentation: Some(presentation),
            });
            let new_area_contests: Vec<AreaContest> = contests
                .clone()
                .into_iter()
                .map(|contest| AreaContest {
                    id: Uuid::new_v4().to_string(),
                    area_id: new_area_id.to_string(),
                    contest_id: contest.id.clone(),
                })
                .collect();
            area_contests.extend(new_area_contests);
        };
    }

    insert_areas(hasura_transaction, &areas).await?;
    insert_area_contests(
        hasura_transaction,
        &tenant_id,
        &election_event_id,
        &area_contests,
    )
    .await?;

    Ok(())
}
