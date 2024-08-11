// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::area::get_area_by_id;
use crate::postgres::document::get_document;
use crate::postgres::election::export_elections;
use crate::postgres::election_event::get_election_event_by_election_area;
use crate::postgres::results_event::get_results_event_by_id;
use crate::postgres::tally_session_execution::get_tally_session_executions;
use crate::services::ceremonies::velvet_tally::generate_initial_state;
use crate::services::compress::decompress_file;
use crate::services::consolidation::eml_generator::ValidateAnnotations;
use crate::services::database::get_hasura_pool;
use crate::services::documents::get_document_as_temp_file;
use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use deadpool_postgres::Client as DbClient;
use deadpool_postgres::Transaction;
use sequent_core::types::hasura::core::Area;
use sequent_core::types::hasura::core::Election;
use sequent_core::util::date_time::get_system_timezone;
use std::collections::HashMap;
use tempfile::NamedTempFile;
use tracing::instrument;
use velvet::pipes::generate_reports::ReportData;

use super::transmission_package::create_transmission_package;

#[instrument(skip(hasura_transaction), err)]
pub async fn download_to_file(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    tally_session_id: &str,
) -> Result<NamedTempFile> {
    let tally_session_executions = get_tally_session_executions(
        hasura_transaction,
        tenant_id,
        election_event_id,
        tally_session_id,
    )
    .await
    .with_context(|| "Error fetching tally session executions")?;

    // the first execution is the latest one
    let tally_session_execution = tally_session_executions
        .first()
        .ok_or_else(|| anyhow!("No tally session executions found"))?;

    let results_event_id = tally_session_execution
        .results_event_id
        .clone()
        .ok_or_else(|| anyhow!("Missing results_event_id in tally session execution"))?;

    let results_event = get_results_event_by_id(
        hasura_transaction,
        tenant_id,
        election_event_id,
        &results_event_id,
    )
    .await
    .with_context(|| "Error fetching results event")?;

    let document_id = results_event
        .documents
        .ok_or_else(|| anyhow!("Missing documents in results_event"))?
        .tar_gz
        .ok_or_else(|| anyhow!("Missing tar_gz in results_event"))?;

    let document = get_document(
        hasura_transaction,
        tenant_id,
        Some(election_event_id.to_string()),
        &document_id,
    )
    .await?
    .ok_or_else(|| anyhow!("Can't find document {}", document_id))?;

    get_document_as_temp_file(tenant_id, &document).await
}

#[instrument(err)]
pub async fn send_eml_service(
    tenant_id: &str,
    election_id: &str,
    area_id: &str,
    tally_session_id: &str,
) -> Result<()> {
    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .with_context(|| "Error acquiring hasura connection pool")?;
    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .with_context(|| "Error acquiring hasura transaction")?;
    let election_event =
        get_election_event_by_election_area(&hasura_transaction, tenant_id, election_id, area_id)
            .await
            .with_context(|| "Error fetching election event")?;
    let tar_gz_file = download_to_file(
        &hasura_transaction,
        tenant_id,
        &election_event.id,
        tally_session_id,
    )
    .await?;

    let tally_path = decompress_file(tar_gz_file.path())?;

    let state = generate_initial_state(&tally_path.into_path())?;

    let results = state.get_results()?;

    let tally_id = 1;
    let transaction_id = 1;
    let time_zone = get_system_timezone();
    let now_utc = Utc::now();

    let election_event_annotations = election_event.get_valid_annotations()?;
    let elections_map: HashMap<String, Election> =
        export_elections(&hasura_transaction, tenant_id, &election_event.id)
            .await
            .with_context(|| "Error fetching elections")?
            .into_iter()
            .map(|election| (election.id.clone(), election))
            .collect();
    let area = get_area_by_id(&hasura_transaction, tenant_id, &area_id)
        .await
        .with_context(|| format!("Error fetching area {}", area_id))?
        .ok_or_else(|| anyhow!("Can't find area {}", area_id))?;
    let area_annotations = area.get_valid_annotations()?;
    for result in results {
        if result.election_id != election_id {
            continue;
        }
        let Some(basic_area) = result.area.clone() else {
            continue;
        };
        if basic_area.id != area_id {
            continue;
        }
        let election = elections_map
            .get(&result.election_id)
            .ok_or_else(|| anyhow!("Can't find election {}", &result.election_id))?;
        let election_annotations = election.get_valid_annotations()?;
        for report_computed in result.reports {
            let report: ReportData = report_computed.into();
            let transmission_package_file = create_transmission_package(
                tally_id,
                transaction_id,
                time_zone.clone(),
                now_utc.clone(),
                &election_event_annotations,
                &election_annotations,
                &report,
            )
            .await?;
        }
    }

    hasura_transaction
        .commit()
        .await
        .with_context(|| "error comitting transaction")?;
    Ok(())
}
