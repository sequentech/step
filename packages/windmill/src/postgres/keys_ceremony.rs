// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::{Client as DbClient, Transaction};
use sequent_core::types::hasura::core::KeysCeremony;
use tokio_postgres::row::Row;
use tracing::{event, instrument, Level};
use uuid::Uuid;

pub struct KeysCeremonyWrapper(pub KeysCeremony);

impl TryFrom<Row> for KeysCeremonyWrapper {
    type Error = anyhow::Error;

    fn try_from(item: Row) -> Result<Self> {
        Ok(KeysCeremonyWrapper(KeysCeremony {
            id: item.try_get::<_, Uuid>("id")?.to_string(),
            created_at: item.get("created_at"),
            last_updated_at: item.get("last_updated_at"),
            tenant_id: item.try_get::<_, Uuid>("tenant_id")?.to_string(),
            election_event_id: item.try_get::<_, Uuid>("election_event_id")?.to_string(),
            trustee_ids: item
                .try_get::<_, Vec<Uuid>>("trustee_ids")?
                .iter()
                .map(|uuid| uuid.to_string())
                .collect(),
            status: item.try_get("status")?,
            execution_status: item.try_get("description")?,
            labels: item.try_get("labels")?,
            annotations: item.try_get("annotations")?,
            threshold: item.try_get("threshold")?,
        }))
    }
}

#[instrument(err, skip_all)]
pub async fn get_keys_ceremonies(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<Vec<KeysCeremony>> {
    let statement = hasura_transaction
        .prepare(
            r#"
                SELECT
                    *
                FROM
                    sequent_backend.keys_ceremony
                WHERE
                    tenant_id = $1 AND
                    election_event_id = $2;
            "#,
        )
        .await?;

    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                &Uuid::parse_str(tenant_id)?,
                &Uuid::parse_str(election_event_id)?,
            ],
        )
        .await?;

    let keys_ceremonies: Vec<KeysCeremony> = rows
        .into_iter()
        .map(|row| -> Result<KeysCeremony> {
            row.try_into()
                .map(|res: KeysCeremonyWrapper| -> KeysCeremony { res.0 })
        })
        .collect::<Result<Vec<KeysCeremony>>>()?;

    Ok(keys_ceremonies)
}
