// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Context, Result};
use deadpool_postgres::{Client as DbClient, Transaction};
use sequent_core::types::ceremonies::TrusteeModePolicy;
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

// Per-ceremony key lookup. `public_key` comes from the `trustee_ceremony_key`
// row scoped to the requested `(election_event_id, keys_ceremony_id)` ($3/$4)
// via a LEFT JOIN. When no scope is requested (both None), or there is no
// matching per-ceremony row, the join yields NULL — there is no fallback to the
// trustee's stable/global key. Callers that only need identity fields (id/name)
// pass None for both and ignore `public_key`.
const TRUSTEE_CEREMONY_KEY_JOIN: &str = r#"
    LEFT JOIN sequent_backend.trustee_ceremony_key AS tck
        ON tck.trustee_id = t.id
        AND tck.tenant_id = t.tenant_id
        AND tck.election_event_id = $3::uuid
        AND tck.keys_ceremony_id = $4::uuid
"#;

const TRUSTEE_CEREMONY_KEY_COLUMNS: &str = r#"
    t.id, t.name, t.tenant_id, t.created_at, t.last_updated_at, t.labels, t.annotations,
    tck.public_key
"#;

#[instrument(err, skip(hasura_transaction))]
pub async fn get_trustees_by_id(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    trustee_ids: &Vec<String>,
    election_event_id: Option<&str>,
    keys_ceremony_id: Option<&str>,
) -> Result<Vec<Trustee>> {
    let trustee_uuids = trustee_ids
        .clone()
        .into_iter()
        .map(|id| Uuid::parse_str(&id).map_err(|err| anyhow!("{:?}", err)))
        .collect::<Result<Vec<Uuid>>>()?;
    let event_uuid: Option<Uuid> = election_event_id
        .map(|s| Uuid::parse_str(s).map_err(|err| anyhow!("{:?}", err)))
        .transpose()?;
    let ceremony_uuid: Option<Uuid> = keys_ceremony_id
        .map(|s| Uuid::parse_str(s).map_err(|err| anyhow!("{:?}", err)))
        .transpose()?;
    let statement = hasura_transaction
        .prepare(&format!(
            r#"
                SELECT
                    {TRUSTEE_CEREMONY_KEY_COLUMNS}
                FROM
                    sequent_backend.trustee AS t
                    {TRUSTEE_CEREMONY_KEY_JOIN}
                WHERE
                    t.tenant_id = $1 AND
                    t.id = ANY($2);
            "#
        ))
        .await?;

    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                &Uuid::parse_str(tenant_id)?,
                &trustee_uuids,
                &event_uuid,
                &ceremony_uuid,
            ],
        )
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
    election_event_id: Option<&str>,
    keys_ceremony_id: Option<&str>,
) -> Result<Vec<Trustee>> {
    let event_uuid: Option<Uuid> = election_event_id
        .map(|s| Uuid::parse_str(s).map_err(|err| anyhow!("{:?}", err)))
        .transpose()?;
    let ceremony_uuid: Option<Uuid> = keys_ceremony_id
        .map(|s| Uuid::parse_str(s).map_err(|err| anyhow!("{:?}", err)))
        .transpose()?;
    let statement = hasura_transaction
        .prepare(&format!(
            r#"
                SELECT
                    {TRUSTEE_CEREMONY_KEY_COLUMNS}
                FROM
                    sequent_backend.trustee AS t
                    {TRUSTEE_CEREMONY_KEY_JOIN}
                WHERE
                    t.tenant_id = $1 AND
                    t.name = ANY($2);
            "#
        ))
        .await?;

    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                &Uuid::parse_str(tenant_id)?,
                &names,
                &event_uuid,
                &ceremony_uuid,
            ],
        )
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
    let trustees = get_trustees_by_name(
        hasura_transaction,
        tenant_id,
        &vec![name.to_string()],
        None,
        None,
    )
    .await?;

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
    // `public_key` here is the trustee's own stable/global key.
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

/// Register (upsert) a trustee's public key for one specific ceremony.
///
/// Writes a row in `trustee_ceremony_key` keyed on the
/// `(tenant_id, trustee_id, election_event_id, keys_ceremony_id)` tuple, so a
/// trustee participating in several ceremonies has one independent key row per
/// ceremony — registrations for different ceremonies never overwrite each other.
/// Re-registering the same ceremony updates that ceremony's key only (the BBT
/// key-loss/regenerate-before-Configuration path relies on this).
#[instrument(err, skip(hasura_transaction))]
pub async fn update_trustee_key_for_event(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    trustee_id: &str,
    election_event_id: &str,
    keys_ceremony_id: &str,
    public_key: &str,
) -> Result<()> {
    let statement = hasura_transaction
        .prepare(
            r#"
                INSERT INTO sequent_backend.trustee_ceremony_key
                    (tenant_id, trustee_id, election_event_id, keys_ceremony_id, public_key)
                VALUES ($1, $2, $3, $4, $5)
                ON CONFLICT (tenant_id, trustee_id, election_event_id, keys_ceremony_id)
                DO UPDATE SET
                    public_key = EXCLUDED.public_key,
                    last_updated_at = NOW();
            "#,
        )
        .await?;

    hasura_transaction
        .execute(
            &statement,
            &[
                &Uuid::parse_str(tenant_id)?,
                &Uuid::parse_str(trustee_id)?,
                &Uuid::parse_str(election_event_id)?,
                &Uuid::parse_str(keys_ceremony_id)?,
                &public_key,
            ],
        )
        .await
        .map_err(|err| anyhow!("Error registering trustee key for ceremony: {err}"))?;

    Ok(())
}

pub fn get_trustee_mode_policy(trustee: &Trustee) -> TrusteeModePolicy {
    trustee
        .annotations
        .as_ref()
        .and_then(|a| a.get("trustee_mode_policy"))
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default()
}
