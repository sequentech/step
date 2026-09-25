// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

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
}
