// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::{Client as DbClient, Transaction};
use sequent_core::types::hasura::core::KeysCeremony;
use serde_json::Value;
use tokio_postgres::row::Row;
use tracing::{event, info, instrument, Level};
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
            execution_status: item.try_get("execution_status")?,
            labels: item.try_get("labels")?,
            annotations: item.try_get("annotations")?,
            threshold: item.try_get::<_, i32>("threshold")? as i64,
            name: item.try_get("name")?,
            settings: item.try_get("settings")?,
            is_default: item.try_get("is_default")?,
            permission_label: item.get::<_, Option<Vec<String>>>("permission_label"),
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

#[instrument(err, skip_all)]
pub async fn get_keys_ceremony_by_id(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    keys_ceremony_id: &str,
) -> Result<KeysCeremony> {
    let statement = hasura_transaction
        .prepare(
            r#"
                SELECT
                    *
                FROM
                    sequent_backend.keys_ceremony
                WHERE
                    tenant_id = $1 AND
                    election_event_id = $2 AND
                    id = $3;
            "#,
        )
        .await?;

    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                &Uuid::parse_str(tenant_id)?,
                &Uuid::parse_str(election_event_id)?,
                &Uuid::parse_str(keys_ceremony_id)?,
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

    keys_ceremonies
        .get(0)
        .map(|keys_ceremony| keys_ceremony.clone())
        .ok_or(anyhow!("Keys ceremony {keys_ceremony_id} not found"))
}

#[instrument(skip(hasura_transaction), err)]
pub async fn insert_keys_ceremony(
    hasura_transaction: &Transaction<'_>,
    id: String,
    tenant_id: String,
    election_event_id: String,
    trustee_ids: Vec<String>,
    threshold: i32,
    status: Option<Value>,
    execution_status: Option<String>,
    name: Option<String>,
    settings: Option<Value>,
    is_default: bool,
    permission_label: Vec<String>,
) -> Result<KeysCeremony> {
    let id_uuid: uuid::Uuid = Uuid::parse_str(&id).with_context(|| "Error parsing id as UUID")?;
    let tenant_uuid: uuid::Uuid =
        Uuid::parse_str(&tenant_id).with_context(|| "Error parsing tenant_id as UUID")?;
    let election_event_uuid: uuid::Uuid = Uuid::parse_str(&election_event_id)
        .with_context(|| "Error parsing election_event_id as UUID")?;
    let trustee_uuids: Vec<uuid::Uuid> = trustee_ids
        .into_iter()
        .map(|trustee_id| Uuid::parse_str(&trustee_id).map_err(|err| anyhow!("{:?}", err)))
        .collect::<Result<Vec<uuid::Uuid>>>()
        .with_context(|| "Error parsing trustee_ids as UUIDs")?;

    let statement = hasura_transaction
        .prepare(
            r#"
                INSERT INTO
                    sequent_backend.keys_ceremony
                (id, tenant_id, election_event_id, trustee_ids, status, execution_status, threshold, name, settings, is_default, permission_label, created_at)
                VALUES(
                    $1,
                    $2,
                    $3,
                    $4,
                    $5,
                    $6,
                    $7,
                    $8,
                    $9,
                    $10,
                    $11,
                    NOW()
                )
                RETURNING
                    *;
            "#,
        )
        .await?;
    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                &id_uuid,
                &tenant_uuid,
                &election_event_uuid,
                &trustee_uuids,
                &status,
                &execution_status,
                &threshold,
                &name,
                &settings,
                &is_default,
                &permission_label,
            ],
        )
        .await
        .map_err(|err| anyhow!("Error inserting keys ceremony: {}", err))?;

    let elements: Vec<KeysCeremony> = rows
        .into_iter()
        .map(|row| -> Result<KeysCeremony> {
            row.try_into()
                .map(|res: KeysCeremonyWrapper| -> KeysCeremony { res.0 })
        })
        .collect::<Result<Vec<KeysCeremony>>>()?;

    elements
        .get(0)
        .map(|val| val.clone())
        .ok_or(anyhow!("Row not inserted"))
}

#[instrument(skip(hasura_transaction, status), err)]
pub async fn update_keys_ceremony_status(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    keys_ceremony_id: &str,
    status: &Value,
    execution_status: &str,
) -> Result<()> {
    let statement = hasura_transaction
        .prepare(
            r#"
                UPDATE
                    sequent_backend.keys_ceremony
                SET
                    status = $1,
                    execution_status = $2
                WHERE
                    id = $3 AND
                    tenant_id = $4 AND
                    election_event_id = $5
                RETURNING
                    id;
            "#,
        )
        .await?;

    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                &status,
                &execution_status.to_string(),
                &Uuid::parse_str(keys_ceremony_id)?,
                &Uuid::parse_str(tenant_id)?,
                &Uuid::parse_str(election_event_id)?,
            ],
        )
        .await
        .map_err(|err| anyhow!("Error running the update_keys_ceremony_status query: {err}"))?;

    if 0 == rows.len() {
        return Err(anyhow!("No keys ceremony found"));
    }

    Ok(())
}

#[instrument(skip(hasura_transaction), err)]
pub async fn list_keys_ceremony(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    permission_labels: &Vec<String>,
) -> Result<Vec<KeysCeremony>> {
    let permission_labels_slice: Vec<&str> = permission_labels.iter().map(AsRef::as_ref).collect();

    info!("permission_labels_slice {:?}", &permission_labels_slice);

    let statement = hasura_transaction
        .prepare(
            r#"
                SELECT
                    keys_ceremony.*
                FROM
                    sequent_backend.keys_ceremony AS keys_ceremony
                WHERE
                    keys_ceremony.tenant_id = $1 AND
                    keys_ceremony.election_event_id = $2 AND
                    (
                        cardinality($3::text[]) = 0
                        OR keys_ceremony.permission_label && $3::text[]
                    );
            "#,
        )
        .await?;

    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                &Uuid::parse_str(tenant_id)?,
                &Uuid::parse_str(election_event_id)?,
                &permission_labels_slice,
            ],
        )
        .await?;

    info!("rows: {:?}", rows);

    let keys_ceremonies: Vec<KeysCeremony> = rows
        .into_iter()
        .map(|row| -> Result<KeysCeremony> {
            let res: KeysCeremonyWrapper = row.try_into()?;
            Ok(res.0)
        })
        .collect::<Result<Vec<KeysCeremony>>>()?;

    Ok(keys_ceremonies)
}
