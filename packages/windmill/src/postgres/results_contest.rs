// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::Result;
use deadpool_postgres::Transaction;
use sequent_core::types::results::ResultDocuments;
use tracing::instrument;

#[instrument(skip(hasura_transaction), err)]
pub async fn update_results_contest_documents(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    results_id: &str,
    election_event_id: &str,
    election_id: &str,
    contest_id: &str,
    documents: &ResultDocuments,
) -> Result<()> {
    Ok(())
}
