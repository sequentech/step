// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::types::hasura::core::Tenant;
use tokio_postgres::row::Row;
use tracing::{event, instrument, Level};
use uuid::Uuid;

pub struct TenantWrapper(pub Tenant);

impl TryFrom<Row> for TenantWrapper {
    type Error = anyhow::Error;
    fn try_from(item: Row) -> Result<Self> {
        Ok(TenantWrapper(Tenant {
            id: item.try_get::<_, Uuid>("id")?.to_string(),
            slug: item.try_get("slug")?,
            created_at: item.try_get("created_at")?,
            updated_at: item.try_get("updated_at")?,
            labels: item.try_get("labels")?,
            annotations: item.try_get("annotations")?,
            is_active: item.try_get("is_active")?,
            voting_channels: item.try_get("voting_channels")?,
            settings: item.try_get("settings")?,
            test: item.try_get::<_, Option<i32>>("test")?,
        }))
    }
}

#[instrument(skip(hasura_transaction), err)]
pub async fn get_tenant_by_id(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
) -> Result<Tenant> {
    // Check if tenant_id is a valid UUID string
    if tenant_id.is_empty() {
        return Err(anyhow!("Tenant ID is empty"));
    }

    let tenant_uuid =
        Uuid::parse_str(tenant_id).map_err(|err| anyhow!("Error parsing tenant UUID: {}", err))?;

    let statement = hasura_transaction
        .prepare(
            r#"
            SELECT
                *
            FROM
                sequent_backend.tenant
            WHERE
                id = $1;
            "#,
        )
        .await?;

    let rows = hasura_transaction
        .query(&statement, &[&tenant_uuid])
        .await
        .map_err(|err| anyhow!("Error fetching Tenants: {}", err))?;

    let tenants: Vec<Tenant> = rows
        .into_iter()
        .map(|row| row.try_into().map(|res: TenantWrapper| res.0))
        .collect::<Result<Vec<_>, _>>()
        .context("Error converting database rows to Tenant")?;

    let tenant = tenants.get(0).map(|tenant| tenant.clone()).context("Error obtaining Tenant")?;
    Ok(tenant)
}
