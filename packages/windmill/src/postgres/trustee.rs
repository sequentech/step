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
            election_event_id: item
                .try_get::<_, Option<Uuid>>("election_event_id")?
                .map(|u| u.to_string()),
            keys_ceremony_id: item
                .try_get::<_, Option<Uuid>>("keys_ceremony_id")?
                .map(|u| u.to_string()),
            name: item.try_get::<_, Option<String>>("name")?,
            tenant_id: item.try_get::<_, Uuid>("tenant_id")?.to_string(),
            created_at: item.get("created_at"),
            last_updated_at: item.get("last_updated_at"),
            labels: item.try_get("labels")?,
            annotations: item.try_get("annotations")?,
        }))
    }
}

// `public_key` is returned only when BOTH `election_event_id` and
// `keys_ceremony_id` are provided AND match the row exactly. No scope
// requested, or scope requested but not matching, both yield NULL — there is
// no fallback to "whatever key the row currently holds". Callers that only
// need identity fields (id/name) pass None for both and ignore `public_key`.
const TRUSTEE_PUBLIC_KEY_SCOPED_CASE: &str = r#"
    CASE
        WHEN ($3::uuid IS NOT NULL AND $4::uuid IS NOT NULL
              AND election_event_id = $3::uuid AND keys_ceremony_id = $4::uuid)
        THEN public_key
        ELSE NULL
    END AS public_key
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
                    id, name, tenant_id, created_at, last_updated_at, labels, annotations,
                    election_event_id, keys_ceremony_id,
                    {TRUSTEE_PUBLIC_KEY_SCOPED_CASE}
                FROM
                    sequent_backend.trustee
                WHERE
                    tenant_id = $1 AND
                    id = ANY($2);
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
                    id, name, tenant_id, created_at, last_updated_at, labels, annotations,
                    election_event_id, keys_ceremony_id,
                    {TRUSTEE_PUBLIC_KEY_SCOPED_CASE}
                FROM
                    sequent_backend.trustee
                WHERE
                    tenant_id = $1 AND
                    name = ANY($2);
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
    let trustees =
        get_trustees_by_name(hasura_transaction, tenant_id, &vec![name.to_string()], None, None)
            .await?;

    trustees
        .get(0)
        .map(|tally_session: &Trustee| tally_session.clone())
        .ok_or(anyhow!("Trustee {name} not found"))
}

/// Idempotently re-stamps a server-based trustee's existing row with this
/// ceremony's scope, without touching `public_key` — server-based trustees
/// keep one stable, long-lived key provisioned out-of-band (CLI /
/// `gen_trustee_config`), so there is nothing to copy, only the scope
/// columns need to point at the current ceremony. Safe to re-run every beat
/// cycle: re-stamping the same values is a no-op. Only stamps rows that
/// already have a public_key — a server-based trustee with no key yet is
/// left untouched (the ceremony's gate will simply keep waiting).
#[instrument(err, skip(hasura_transaction))]
pub async fn stamp_trustee_ceremony_scope(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    trustee_id: &str,
    election_event_id: &str,
    keys_ceremony_id: &str,
) -> Result<()> {
    let statement = hasura_transaction
        .prepare(
            r#"
                UPDATE sequent_backend.trustee
                SET election_event_id = $1,
                    keys_ceremony_id = $2,
                    last_updated_at = NOW()
                WHERE id = $3 AND tenant_id = $4 AND public_key IS NOT NULL;
            "#,
        )
        .await?;

    hasura_transaction
        .execute(
            &statement,
            &[
                &Uuid::parse_str(election_event_id)?,
                &Uuid::parse_str(keys_ceremony_id)?,
                &Uuid::parse_str(trustee_id)?,
                &Uuid::parse_str(tenant_id)?,
            ],
        )
        .await
        .map_err(|err| anyhow!("Error stamping trustee ceremony scope: {err}"))?;

    Ok(())
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
                UPDATE sequent_backend.trustee
                SET public_key = $1,
                    election_event_id = $2,
                    keys_ceremony_id = $3,
                    last_updated_at = NOW()
                WHERE id = $4 AND tenant_id = $5;
            "#,
        )
        .await?;

    hasura_transaction
        .execute(
            &statement,
            &[
                &public_key,
                &Uuid::parse_str(election_event_id)?,
                &Uuid::parse_str(keys_ceremony_id)?,
                &Uuid::parse_str(trustee_id)?,
                &Uuid::parse_str(tenant_id)?,
            ],
        )
        .await
        .map_err(|err| anyhow!("Error updating trustee key for event: {err}"))?;

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
