// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::ports::documents::DocumentStorage;
use sequent_core::types::hasura::core::Document;
use tempfile::NamedTempFile;
use windmill::services::documents::get_document_as_temp_file;

/// The S3 buckets Windmill configures from the environment.
pub struct S3DocumentStorage;

#[rocket::async_trait]
impl DocumentStorage for S3DocumentStorage {
    async fn download(
        &self,
        tenant_id: &str,
        document: &Document,
    ) -> anyhow::Result<NamedTempFile> {
        get_document_as_temp_file(tenant_id, document).await
    }
}
