// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Context, Result};
use deadpool_postgres::{Client as DbClient, Transaction};
use sequent_core::types::hasura::core::Trustee;
use serde_json::value::Value;
use tokio_postgres::row::Row;
use tracing::{event, instrument, Level};
use uuid::Uuid;

pub struct TrusteeWrapper(pub Trustee);

impl TryFrom<Row> for TrusteeWrapper {
    type Error = anyhow::Error;

    fn try_from(item: Row) -> Result<Self> {
        Ok(TrusteeWrapper(Trustee {
            id: item.try_get::<_, Uuid>("id")?.to_string(),
            public_key: item.try_get::<_, Option<String>>("public_key")?,
            name: item.try_get::<_, Option<String>>("name")?,
            tenant_id: item.try_get::<_, Uuid>("tenant_id")?.to_string(),
            created_at: item.get("created_at"),
            last_updated_at: item.get("last_updated_at"),
            labels: item.try_get("labels")?,
            annotations: item.try_get("annotations")?,
        }))
    }
}

#[instrument(err, skip(hasura_transaction))]
pub async fn get_trustees_by_id(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    trustee_ids: &Vec<String>,
) -> Result<Vec<Trustee>> {
    let trustee_uuids = trustee_ids
        .clone()
        .into_iter()
        .map(|id| Uuid::parse_str(&id).map_err(|err| anyhow!("{:?}", err)))
        .collect::<Result<Vec<Uuid>>>()?;
    let statement = hasura_transaction
        .prepare(
            r#"
                SELECT
                    *
                FROM
                    sequent_backend.trustee
                WHERE
                    tenant_id = $1 AND
                    id = ANY($2);
            "#,
        )
        .await?;

    let rows: Vec<Row> = hasura_transaction
        .query(&statement, &[&Uuid::parse_str(tenant_id)?, &trustee_uuids])
        .await?;

    rows.into_iter()
        .map(|row| -> Result<Trustee> {
            row.try_into()
                .map(|res: TrusteeWrapper| -> Trustee { res.0 })
        })
        .collect::<Result<Vec<Trustee>>>()
}

#[instrument(err, skip(hasura_transaction))]
pub async fn get_trustees_by_name(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    names: &Vec<String>,
) -> Result<Vec<Trustee>> {
    let statement = hasura_transaction
        .prepare(
            r#"
                SELECT
                    *
                FROM
                    sequent_backend.trustee
                WHERE
                    tenant_id = $1 AND
                    name = ANY($2);
            "#,
        )
        .await?;

    let rows: Vec<Row> = hasura_transaction
        .query(&statement, &[&Uuid::parse_str(tenant_id)?, &names])
        .await?;

    rows.into_iter()
        .map(|row| -> Result<Trustee> {
            row.try_into()
                .map(|res: TrusteeWrapper| -> Trustee { res.0 })
        })
        .collect::<Result<Vec<Trustee>>>()
}

#[instrument(err, skip(hasura_transaction))]
pub async fn get_trustee_by_name(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    name: &str,
) -> Result<Trustee> {
    let trustees =
        get_trustees_by_name(hasura_transaction, tenant_id, &vec![name.to_string()]).await?;

    trustees
        .get(0)
        .map(|tally_session: &Trustee| tally_session.clone())
        .ok_or(anyhow!("Trustee {name} not found"))
}

#[instrument(err, skip(hasura_transaction))]
pub async fn get_all_trustees(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
) -> Result<Vec<Trustee>> {
    let statement = hasura_transaction
        .prepare(
            r#"
                SELECT
                    *
                FROM
                    sequent_backend.trustee
                WHERE
                    tenant_id = $1;
            "#,
        )
        .await?;

    let rows: Vec<Row> = hasura_transaction
        .query(&statement, &[&Uuid::parse_str(tenant_id)?])
        .await?;

    let elements: Vec<Trustee> = rows
        .into_iter()
        .map(|row| -> Result<Trustee> {
            row.try_into()
                .map(|res: TrusteeWrapper| -> Trustee { res.0 })
        })
        .collect::<Result<Vec<Trustee>>>()?;

    Ok(elements)
}
