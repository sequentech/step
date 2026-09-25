// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::ports::vault::SecretVault;
use deadpool_postgres::Transaction;
use windmill::services::document_password::save_password;
use windmill::services::reports_vault::get_report_key_pair;

/// The vault Windmill configures from the environment.
pub struct WindmillVault;

#[rocket::async_trait]
impl SecretVault for WindmillVault {
    async fn save_document_password(
        &self,
        transaction: &Transaction<'_>,
        tenant_id: &str,
        election_event_id: Option<&str>,
        document_id: &str,
        password: &str,
    ) -> anyhow::Result<String> {
        save_password(
            transaction,
            tenant_id,
            election_event_id,
            document_id,
            password,
        )
        .await
    }

    async fn check_report_password(
        &self,
        transaction: &Transaction<'_>,
        tenant_id: String,
        election_event_id: String,
        report_id: Option<String>,
        password: String,
    ) -> anyhow::Result<()> {
        get_report_key_pair(
            transaction,
            tenant_id,
            election_event_id,
            report_id,
            password,
        )
        .await
    }
}
