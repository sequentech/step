// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::vault;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use serde::{Deserialize, Serialize};

const SECRET_KEY_PREFIX: &str = "voter-information-letter";

#[derive(Clone, Deserialize, Serialize)]
pub struct VoterInformationLetterSecret {
    pub voter_password: String,
    pub pdf_password: String,
}

pub fn secret_key(task_id: &str) -> String {
    format!("{SECRET_KEY_PREFIX}-{task_id}")
}

pub async fn read_secret(
    transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    task_id: &str,
) -> Result<Option<VoterInformationLetterSecret>> {
    let value = vault::read_secret(
        transaction,
        tenant_id,
        Some(election_event_id),
        &secret_key(task_id),
    )
    .await?;

    value
        .map(|value| {
            serde_json::from_str(&value)
                .with_context(|| "Failed to deserialize Voter Information Letter secret")
        })
        .transpose()
}

pub async fn save_secret(
    transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    task_id: &str,
    secret: &VoterInformationLetterSecret,
) -> Result<()> {
    let value = serde_json::to_string(secret)
        .map_err(|error| anyhow!("Failed to serialize Voter Information Letter secret: {error}"))?;
    vault::save_secret(
        transaction,
        tenant_id,
        Some(election_event_id),
        &secret_key(task_id),
        &value,
    )
    .await
}
