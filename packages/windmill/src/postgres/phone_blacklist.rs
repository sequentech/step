// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Result};
use chrono::{DateTime, Local, Utc};
use deadpool_postgres::Transaction;
use sequent_core::services::uuid_validation::parse_uuid_v4;
use sequent_core::types::hasura::core::PhoneBlacklistEntry;
use tokio_postgres::row::Row;
use tokio_stream::StreamExt;
use tracing::instrument;
use uuid::Uuid;

struct PhoneBlacklistEntryRow(PhoneBlacklistEntry);

impl TryFrom<Row> for PhoneBlacklistEntryRow {
    type Error = anyhow::Error;

    fn try_from(item: Row) -> Result<Self> {
        Ok(PhoneBlacklistEntryRow(PhoneBlacklistEntry {
            id: item.try_get::<_, Uuid>("id")?.to_string(),
            tenant_id: item.try_get::<_, Uuid>("tenant_id")?.to_string(),
            election_event_id: item.try_get::<_, Uuid>("election_event_id")?.to_string(),
            phone_e164: item.try_get("phone_e164")?,
            reason: item.try_get("reason")?,
            created_at: item
                .try_get::<_, DateTime<Utc>>("created_at")?
                .with_timezone(&Local),
            created_by: item.try_get::<_, Uuid>("created_by")?.to_string(),
            updated_at: item
                .try_get::<_, DateTime<Utc>>("updated_at")?
                .with_timezone(&Local),
        }))
    }
}

#[instrument(err, skip_all)]
pub async fn insert_phone_blacklist_entry(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    phone_e164: &str,
    reason: Option<&String>,
    user_id: &str,
) -> Result<PhoneBlacklistEntry> {
    let statement = hasura_transaction
        .prepare(
            r#"
                INSERT INTO sequent_backend.phone_blacklist
                (tenant_id, election_event_id, phone_e164, reason, created_by)
                VALUES
                ($1, $2, $3, $4, $5)
                RETURNING *;
                "#,
        )
        .await?;
    let rows = hasura_transaction
        .query(
            &statement,
            &[
                &parse_uuid_v4(tenant_id)?,
                &parse_uuid_v4(election_event_id)?,
                &phone_e164,
                &reason,
                &parse_uuid_v4(user_id)?,
            ],
        )
        .await?
        .into_iter()
        .map(|row| PhoneBlacklistEntryRow::try_from(row).map(|entry_row| entry_row.0))
        .collect::<Result<Vec<PhoneBlacklistEntry>>>()?;

    Ok(rows
        .into_iter()
        .next()
        .ok_or(anyhow!("Row insert returned no rows"))?)
}

#[instrument(err, skip_all)]
pub async fn delete_phone_blacklist_entry(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    entry_id: &str,
) -> Result<PhoneBlacklistEntry> {
    let statement = hasura_transaction
        .prepare(
            r#"
                DELETE FROM sequent_backend.phone_blacklist
                WHERE
                  tenant_id = $1
                  AND election_event_id = $2
                  AND id = $3
                  RETURNING *;
                "#,
        )
        .await?;
    let deleted_row = hasura_transaction
        .query_one(
            &statement,
            &[
                &parse_uuid_v4(tenant_id)?,
                &parse_uuid_v4(election_event_id)?,
                &parse_uuid_v4(entry_id)?,
            ],
        )
        .await?;
    let deleted = PhoneBlacklistEntryRow::try_from(deleted_row).map(|entry_row| entry_row.0)?;

    Ok(deleted)
}
