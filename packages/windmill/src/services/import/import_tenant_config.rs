// SPDX-FileCopyrightText: 2024 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::tasks::import_tenant_config::ImportOptions;
use crate::types::error::Result;
use deadpool_postgres::Transaction;

pub async fn import_tenant_config_file(
    hasura_transaction: &Transaction<'_>,
    object: ImportOptions,
    tenant_id: &str,
) -> Result<()> {
    Ok(())
}
pub async fn import_keycloak_config_file(
    hasura_transaction: &Transaction<'_>,
    object: ImportOptions,
    tenant_id: &str,
) -> Result<()> {
    Ok(())
}
pub async fn import_roles_config_file(
    hasura_transaction: &Transaction<'_>,
    object: ImportOptions,
    tenant_id: &str,
) -> Result<()> {
    Ok(())
}

pub async fn import_tenant_config_zip(
    hasura_transaction: &Transaction<'_>,
    object: ImportOptions,
    tenant_id: &str,
    document_id: &str,
) -> Result<()> {
    Ok(())
}
