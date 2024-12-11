use crate::services::consolidation::aes_256_cbc_encrypt::encrypt_file_aes_256_cbc;
// SPDX-FileCopyrightText: 2024 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::reports::{get_report_by_type, ReportType};
use crate::services::database::get_hasura_pool;
use crate::services::reports::template_renderer::EReportEncryption;
use crate::services::reports_vault::get_report_secret_key;
use crate::services::vault;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Client as DbClient;
use deadpool_postgres::Transaction;
use tracing::instrument;

/// Encrypt all files in a directory
#[instrument(err, skip_all)]
pub async fn encrypt_directory_contents(
    tenant_id: &str,
    election_event_id: &str,
    report_type: ReportType,
    old_path: &String,
    new_path: &String,
) -> Result<String> {
    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .with_context(|| "Error acquiring hasura connection pool")?;

    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .with_context(|| "Error acquiring hasura transaction")?;

    let report = get_report_by_type(
        &hasura_transaction,
        tenant_id,
        election_event_id,
        &report_type.to_string(),
    )
    .await
    .map_err(|err| anyhow!("Error getting report: {err:?}"))?;

    let mut upload_path = old_path.clone();
    if let Some(report) = &report {
        if report.encryption_policy == EReportEncryption::ConfiguredPassword {
            let secret_key =
                get_report_secret_key(&tenant_id, &election_event_id, Some(report.id.clone()));

            let encryption_password = vault::read_secret(secret_key.clone())
                .await?
                .ok_or_else(|| anyhow!("Encryption password not found"))?;

            encrypt_file_aes_256_cbc(&old_path.as_str(), &new_path.as_str(), &encryption_password)
                .map_err(|err| anyhow!("Error encrypting file: {err:?}"))?;

            upload_path = new_path.to_string();
        };
    };

    Ok(upload_path)
}
