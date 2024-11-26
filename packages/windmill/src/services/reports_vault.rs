// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::vault::{save_secret, vault::read_secret};
use anyhow::anyhow;
use tracing::instrument;

pub fn get_report_secret_key(
    tenant_id: &str,
    election_event_id: &str,
    report_id: Option<String>,
) -> String {
    format!(
        "tenant-{}-event-{}-report_id-{}",
        tenant_id,
        election_event_id,
        report_id.unwrap_or_else(|| "default".to_string())
    )
}

#[instrument(err)]
pub async fn get_password(
    tenant_id: String,
    election_event_id: String,
    report_id: Option<String>,
) -> Result<Option<String>, anyhow::Error> {
    let secret_key = get_report_secret_key(&tenant_id, &election_event_id, report_id);

    let existing_key = read_secret(secret_key.clone())
        .await
        .map_err(|err| anyhow!("Error reading secret for key '{}': {:?}", secret_key, err))?;

    Ok(existing_key)
}

#[instrument(err)]
pub async fn get_report_key_pair(
    tenant_id: String,
    election_event_id: String,
    report_id: Option<String>,
    password: String,
) -> Result<(), anyhow::Error> {
    let secret_key = get_report_secret_key(&tenant_id, &election_event_id, report_id);

    save_secret(secret_key.clone(), password.clone()).await?;

    Ok(())
}
