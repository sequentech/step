// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::ports::documents::DocumentStorage;
use deadpool_postgres::Transaction;
use sequent_core::types::hasura::core::Document;
use tempfile::NamedTempFile;
use windmill::services::documents::{
    get_document_as_temp_file, get_document_url,
};

/// The S3 buckets Windmill configures from the environment.
pub struct S3DocumentStorage;

#[rocket::async_trait]
impl DocumentStorage for S3DocumentStorage {
    async fn save_json(
        &self,
        transaction: &Transaction<'_>,
        tenant_id: &str,
        event_id: &str,
        name: &str,
        content: &[u8],
    ) -> anyhow::Result<Document> {
        use std::io::Write;
        let mut file = NamedTempFile::new()?;
        file.write_all(content)?;
        windmill::services::documents::upload_and_return_document(
            transaction,
            file.path()
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("invalid temporary path"))?,
            content.len() as u64,
            "application/json",
            tenant_id,
            Some(event_id.to_string()),
            name,
            None,
            false,
        )
        .await
    }

    async fn download(
        &self,
        tenant_id: &str,
        document: &Document,
    ) -> anyhow::Result<NamedTempFile> {
        get_document_as_temp_file(tenant_id, document).await
    }

    async fn url(
        &self,
        hasura_transaction: &Transaction<'_>,
        tenant_id: &str,
        election_event_id: Option<&str>,
        document_id: &str,
    ) -> anyhow::Result<Option<String>> {
        get_document_url(
            hasura_transaction,
            tenant_id,
            election_event_id,
            document_id,
        )
        .await
    }
}
