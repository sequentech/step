// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::ports::vault::SecretVault;
use deadpool_postgres::Transaction;
use std::collections::{HashMap, HashSet};
use windmill::services::document_password::save_password;
use windmill::services::reports_vault::get_report_key_pair;
use windmill::services::voter_secret_attributes::{
    decrypt_attribute_values, encrypt_secret_attribute_map,
};

/// The vault Windmill configures from the environment.
pub struct WindmillVault;

#[rocket::async_trait]
impl SecretVault for WindmillVault {
    async fn encrypt_secret_attributes(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        user_id: &str,
        secret_names: &HashSet<String>,
        values: HashMap<String, Option<Vec<String>>>,
    ) -> anyhow::Result<HashMap<String, Vec<String>>> {
        encrypt_secret_attribute_map(
            tenant_id,
            election_event_id,
            user_id,
            secret_names,
            values,
        )
        .await
    }

    async fn decrypt_secret_attribute(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        user_id: &str,
        attribute_name: &str,
        values: &[String],
    ) -> anyhow::Result<Vec<String>> {
        decrypt_attribute_values(
            tenant_id,
            election_event_id,
            user_id,
            attribute_name,
            values,
        )
        .await
    }

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
