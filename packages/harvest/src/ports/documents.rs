// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use deadpool_postgres::Transaction;
use sequent_core::types::hasura::core::Document;
use tempfile::NamedTempFile;

/// The object storage that holds uploaded documents.
#[rocket::async_trait]
pub trait DocumentStorage: Send + Sync {
    async fn download(
        &self,
        tenant_id: &str,
        document: &Document,
    ) -> anyhow::Result<NamedTempFile>;

    /// A short-lived URL of the document's object, or None when the
    /// document's row is gone.
    async fn url(
        &self,
        hasura_transaction: &Transaction<'_>,
        tenant_id: &str,
        election_event_id: Option<&str>,
        document_id: &str,
    ) -> anyhow::Result<Option<String>>;
}
