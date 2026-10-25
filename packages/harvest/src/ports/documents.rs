// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use deadpool_postgres::Transaction;
use sequent_core::types::hasura::core::Document;
use tempfile::NamedTempFile;

/// The object storage that holds uploaded documents.
#[rocket::async_trait]
pub trait DocumentStorage: Send + Sync {
    /// Store a new private event JSON document with a server-selected name.
    async fn save_json(
        &self,
        _transaction: &Transaction<'_>,
        _tenant_id: &str,
        _event_id: &str,
        _name: &str,
        _content: &[u8],
    ) -> anyhow::Result<Document> {
        anyhow::bail!("JSON export storage is unavailable")
    }

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
