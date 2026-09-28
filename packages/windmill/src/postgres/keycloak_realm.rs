// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use tracing::instrument;

#[instrument(skip(keycloak_transaction), err)]
pub async fn find_realm_id(
    keycloak_transaction: &Transaction<'_>,
    realm_name: &str,
) -> Result<Option<String>> {
    let row = keycloak_transaction
        .query_opt(
            "SELECT id::VARCHAR AS id FROM realm WHERE name = $1",
            &[&realm_name],
        )
        .await
        .context("Error querying realm ID")?;
    row.map(|row| row.try_get("id"))
        .transpose()
        .context("Error reading realm ID")
}

#[instrument(skip(keycloak_transaction), err)]
pub async fn get_realm_id(
    keycloak_transaction: &Transaction<'_>,
    realm_name: String,
) -> Result<String> {
    find_realm_id(keycloak_transaction, &realm_name)
        .await?
        .ok_or_else(|| anyhow!("realm not found: {realm_name}"))
}

#[instrument(skip(keycloak_transaction), err)]
pub async fn get_duplicate_emails_allowed(
    keycloak_transaction: &Transaction<'_>,
    realm_id: &str,
) -> Result<bool> {
    let row = keycloak_transaction
        .query_one(
            "SELECT duplicate_emails_allowed FROM realm WHERE id = $1",
            &[&realm_id],
        )
        .await
        .context("Error querying duplicate_emails_allowed")?;
    row.try_get::<_, bool>("duplicate_emails_allowed")
        .context("Error reading duplicate_emails_allowed")
}
