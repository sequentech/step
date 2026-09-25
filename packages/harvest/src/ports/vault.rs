// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use deadpool_postgres::Transaction;
use std::collections::{HashMap, HashSet};

/// Secrets kept in the vault: the key voters' encrypted attributes use,
/// document passwords and report key pairs.
#[rocket::async_trait]
pub trait SecretVault: Send + Sync {
    async fn encrypt_secret_attributes(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        user_id: &str,
        secret_names: &HashSet<String>,
        values: HashMap<String, Option<Vec<String>>>,
    ) -> anyhow::Result<HashMap<String, Vec<String>>>;
    async fn decrypt_secret_attribute(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        user_id: &str,
        attribute_name: &str,
        values: &[String],
    ) -> anyhow::Result<Vec<String>>;
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
