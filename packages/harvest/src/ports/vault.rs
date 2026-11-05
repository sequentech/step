// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use deadpool_postgres::Transaction;

/// Document passwords and report key pairs kept in the vault.
#[rocket::async_trait]
pub trait SecretVault: Send + Sync {
    /// Returns the id of the stored secret.
    async fn save_document_password(
        &self,
        transaction: &Transaction<'_>,
        tenant_id: &str,
        election_event_id: Option<&str>,
        document_id: &str,
        password: &str,
    ) -> anyhow::Result<String>;
    async fn check_report_password(
        &self,
        transaction: &Transaction<'_>,
        tenant_id: String,
        election_event_id: String,
        report_id: Option<String>,
        password: String,
    ) -> anyhow::Result<()>;
}
