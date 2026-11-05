// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Per-tenant keys for the digests that correlate recipients and Messenger
//! links without storing them. Generated on first use and kept in the
//! secret store.

use crate::services::vault::{read_secret, save_secret};
use anyhow::{Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::types::messaging::CredentialName;
use uuid::Uuid;

#[derive(Debug, Clone, Copy)]
pub enum TenantKey {
    Destination,
    MessengerLink,
}

impl TenantKey {
    fn secret_key(&self, tenant_id: &Uuid) -> String {
        match self {
            TenantKey::Destination => format!("messaging-destination-key-{tenant_id}"),
            TenantKey::MessengerLink => format!("messaging-link-key-{tenant_id}"),
        }
    }
}

/// The key, created if this tenant has none yet.
pub async fn tenant_key(tx: &Transaction<'_>, tenant_id: &Uuid, key: TenantKey) -> Result<Vec<u8>> {
    let name = key.secret_key(tenant_id);
    let tenant = tenant_id.to_string();
    if let Some(value) = read_secret(tx, &tenant, None, &name).await? {
        return hex::decode(value).context("invalid stored messaging key");
    }
    let value = messaging::link::new_reference();
    save_secret(tx, &tenant, None, &name, &value).await?;
    hex::decode(value).context("invalid generated messaging key")
}

/// Name of an account credential in the secret store. Secret keys are
/// unique across tenants, so the account ID is part of it.
pub fn credential_secret_key(account_id: &Uuid, name: CredentialName) -> String {
    format!("messaging-account-{account_id}-{name}")
}
