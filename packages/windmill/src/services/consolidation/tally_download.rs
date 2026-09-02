// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Retrieval of the original tally archive of a tally session. Used by the
//! template and PDF rendering tasks; independent of the Miru integration.

use crate::postgres::document::get_document;
use crate::postgres::results_event::get_results_event_by_id;
use crate::postgres::tally_session_execution::get_last_tally_session_execution;
use crate::services::documents::get_document_as_temp_file;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::types::results::ResultDocumentType;
use tempfile::NamedTempFile;
use tracing::instrument;

#[instrument(skip(hasura_transaction), err)]
pub async fn download_tally_tar_gz_to_file(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    tally_session_id: &str,
) -> Result<NamedTempFile> {
    let tally_session_execution = get_last_tally_session_execution(
        hasura_transaction,
        tenant_id,
        election_event_id,
        tally_session_id,
    )
    .await
    .with_context(|| "Error fetching tally session executions")?
    .ok_or(anyhow!("No tally session execution found"))?;

    let results_event_id = tally_session_execution
        .results_event_id
        .clone()
        .ok_or_else(|| anyhow!("Missing results_event_id in tally session execution"))?;

    let result_event = get_results_event_by_id(
        hasura_transaction,
        tenant_id,
        election_event_id,
        &results_event_id,
    )
    .await
    .with_context(|| "Error fetching results event")?;

    let document_type = ResultDocumentType::TarGzOriginal;
    let document_id = result_event
        .documents
        .ok_or_else(|| anyhow!("Missing documents in results_event"))?
        .get_document_by_type(&document_type)
        .ok_or_else(|| anyhow!(format!("Missing {:?} in results_event", document_type)))?;

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
