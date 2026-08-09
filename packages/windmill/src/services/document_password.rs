// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::vault;
use anyhow::{Context, Result};
use deadpool_postgres::Transaction;
use serde::{Deserialize, Serialize};
use tracing::instrument;

const SECRET_KEY_PREFIX: &str = "document-password";

#[derive(Clone, Deserialize, Serialize)]
pub struct DocumentPasswordSecret {
    pub password: String,
}

pub fn secret_key(document_id: &str) -> String {
    format!("{SECRET_KEY_PREFIX}-{document_id}")
}

#[instrument(skip_all, err)]
pub async fn save_password(
    transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: Option<&str>,
    document_id: &str,
    password: &str,
) -> Result<String> {
    let value = serde_json::to_string(&DocumentPasswordSecret {
        password: password.to_string(),
    })
    .context("Failed to serialize document password secret")?;
    let secret = vault::save_secret_and_return(
        transaction,
        tenant_id,
        election_event_id,
        &secret_key(document_id),
        &value,
    )
    .await
    .context("Failed to store document password secret")?;

    Ok(secret.id)
}

#[instrument(skip_all, err)]
pub async fn read_password(
    transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: Option<&str>,
    document_id: &str,
    secret_id: &str,
) -> Result<Option<DocumentPasswordSecret>> {
    let Some((secret, value)) =
        vault::read_secret_by_id(transaction, tenant_id, election_event_id, secret_id).await?
    else {
        return Ok(None);
    };

    if secret.key != secret_key(document_id) {
        return Ok(None);
    }

    serde_json::from_str(&value)
        .context("Failed to deserialize document password secret")
        .map(Some)
}

#[cfg(test)]
mod tests {
    use super::secret_key;

    #[test]
    fn scopes_the_secret_key_to_the_document() {
        assert_eq!("document-password-document-id", secret_key("document-id"));
    }
}
