// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
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

    let tenant = tenants
        .get(0)
        .map(|tenant| tenant.clone())
        .context("Error obtaining Tenant")?;
    Ok(tenant)
}

#[instrument(skip(hasura_transaction), err)]
pub async fn update_tenant(
    hasura_transaction: &Transaction<'_>,
    new_tenant: Tenant,
    old_tenant_id: &str,
) -> Result<()> {
    let old_tenant_uuid =
        Uuid::parse_str(old_tenant_id).context("Failed to parse old_tenant_id as UUID")?;

    let statement = hasura_transaction
        .prepare(
            r#"
                UPDATE
                    "sequent_backend".tenant
                SET
                    created_at = $1,
                    updated_at = $2,
                    labels = $3,
                    annotations = $4,
                    is_active = $5,
                    voting_channels = $6,
                    settings = $7,
                    test = $8
                WHERE id = $9
                RETURNING
                    *;
            "#,
        )
        .await
        .map_err(|err| anyhow!("Error preparing update_tenant statement: {}", err))?;

    let rows = hasura_transaction
        .execute(
            &statement,
            &[
                &new_tenant.created_at,
                &new_tenant.updated_at,
                &new_tenant.labels,
                &new_tenant.annotations,
                &new_tenant.is_active,
                &new_tenant.voting_channels,
                &new_tenant.settings,
                &new_tenant.test,
                &old_tenant_uuid,
            ],
        )
        .await
        .context("Failed to execute update tenant")?;

    if rows == 0 {
        return Err(anyhow!("No tenant found with the given tenant_id and id"));
    } else if rows > 2 {
        return Err(anyhow!("Too many affected rows in table tenant: {}", rows));
    }

    Ok(())
}

#[instrument(skip(hasura_transaction), err)]
pub async fn insert_tenant(
    hasura_transaction: &Transaction<'_>,
    id: &str,
    slug: &str,
) -> Result<()> {
    let statement = hasura_transaction
        .prepare(
            r#"
                INSERT INTO sequent_backend.tenant
                (id, slug, is_active)
                VALUES ($1, $2, true)
                RETURNING
                id,
                slug,
                created_at,
                updated_at,
                labels,
                annotations,
                is_active;
            "#,
        )
        .await
        .map_err(|err| anyhow!("Error preparing update_tenant statement: {}", err))?;

    let _rows = hasura_transaction
        .execute(&statement, &[&Uuid::parse_str(&id)?, &slug])
        .await
        .context("Failed to execute update tenant")?;

    Ok(())
}

#[instrument(skip(hasura_transaction), err)]
pub async fn get_tenant_by_id_if_exist(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
) -> Result<Option<Tenant>> {
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

    if (rows.is_empty()) {
        return Ok(None);
    }

    let tenants: Vec<Tenant> = rows
        .into_iter()
        .map(|row| row.try_into().map(|res: TenantWrapper| res.0))
        .collect::<Result<Vec<_>, _>>()
        .context("Error converting database rows to Tenant")?;

    let tenant = tenants
        .get(0)
        .map(|tenant| tenant.clone())
        .context("Error obtaining Tenant")?;
    Ok(Some(tenant))
}

#[instrument(skip(hasura_transaction), err)]
pub async fn get_tenant_by_slug_if_exist(
    hasura_transaction: &Transaction<'_>,
    slug: &str,
) -> Result<Option<Tenant>> {
    let statement = hasura_transaction
        .prepare(
            r#"
            SELECT
                *
            FROM
                sequent_backend.tenant
            WHERE
                slug = $1;
            "#,
        )
        .await?;

    let rows = hasura_transaction
        .query(&statement, &[&slug])
        .await
        .map_err(|err| anyhow!("Error fetching Tenants: {}", err))?;

    if (rows.is_empty()) {
        return Ok(None);
    }

    let tenants: Vec<Tenant> = rows
        .into_iter()
        .map(|row| row.try_into().map(|res: TenantWrapper| res.0))
        .collect::<Result<Vec<_>, _>>()
        .context("Error converting database rows to Tenant")?;

    let tenant = tenants
        .get(0)
        .map(|tenant| tenant.clone())
        .context("Error obtaining Tenant")?;
    Ok(Some(tenant))
}
