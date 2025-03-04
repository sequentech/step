// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::tally_session::get_tally_sessions_by_election_event_id;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;

use sequent_core::{types::hasura::core::TallySession, util::temp_path::generate_temp_file};
use tempfile::TempPath;
use tracing::{event, info, instrument, Level};

#[instrument(err)]
pub async fn export_tally_session(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<(String, TempPath)> {
    let event_tally_sessions: Vec<TallySession> = get_tally_sessions_by_election_event_id(
        hasura_transaction,
        tenant_id,
        election_event_id,
        false,
    )
    .await
    .map_err(|e| anyhow!("Error in get_tally_sessions_by_election_event_id: {e:?}"))?;

    let file_name = "export_tally_session".to_string();

    let mut writer = csv::WriterBuilder::new().delimiter(b',').from_writer(
        generate_temp_file(&file_name, ".csv").with_context(|| "Error creating temporary file")?,
    );

    writer.write_record(&[
        "id".to_string(),
        "tenant_id".to_string(),
        "election_event_id".to_string(),
        "created_at".to_string(),
        "last_updated_at".to_string(),
        "labels".to_string(),
        "annotations".to_string(),
        "election_ids".to_string(),
        "area_ids".to_string(),
        "is_execution_completed".to_string(),
        "keys_ceremony_id".to_string(),
        "execution_status".to_string(),
        "threshold".to_string(),
        "configuration".to_string(),
        "tally_type".to_string(),
        "permission_label".to_string(),
    ])?;

    for tally_session in event_tally_sessions {
        let values: Vec<String> = serde_json::to_value(tally_session)?
            .as_object()
            .ok_or_else(|| anyhow!("Failed to convert tally_session to JSON object"))?
            .values()
            .map(|value| value.to_string())
            .collect();

        writer.write_record(&values);
    }

    writer
        .flush()
        .with_context(|| "Error flushing CSV writer")?;

    let temp_path = writer
        .into_inner()
        .with_context(|| "Error getting inner writer")?
        .into_temp_path();

    Ok((file_name, temp_path))
}

#[instrument(err)]
pub async fn export_tally(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    tally_folder_name: &str,
) -> Result<Vec<(String, TempPath)>> {
    let tally_session_path =
        export_tally_session(&hasura_transaction, tenant_id, election_event_id)
            .await
            .map_err(|e| anyhow!("Error in export_tally_session: {e:?}"))?;

    Ok(vec![tally_session_path])
}
