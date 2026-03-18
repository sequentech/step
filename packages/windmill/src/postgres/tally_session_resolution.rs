// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Result};
use deadpool_postgres::Transaction;
use sequent_core::types::ceremonies::{
    TallySessionResolution, TallySessionResolutionData, TallySessionResolutionStatus,
    TallySessionResolutionType,
};
use tokio_postgres::Row;
use tracing::{info, instrument};
use uuid::Uuid;

fn map_row_to_resolution(row: &Row) -> Result<TallySessionResolution> {
    let resolution_type_str: String = row.get(7);
    let status_str: String = row.get(8);
    Ok(TallySessionResolution {
        id: row.get::<_, Uuid>(0).to_string(),
        tenant_id: row.get::<_, Uuid>(1).to_string(),
        election_event_id: row.get::<_, Uuid>(2).to_string(),
        tally_session_id: row.get::<_, Uuid>(3).to_string(),
        contest_id: row.get::<_, Option<Uuid>>(4).map(|u| u.to_string()),
        created_at: row.get(5),
        last_updated_at: row.get(6),
        resolution_type: serde_json::from_value(serde_json::Value::String(resolution_type_str))?,
        status: serde_json::from_value(serde_json::Value::String(status_str))?,
        resolution_data: row
            .get::<_, Option<serde_json::Value>>(9)
            .map(serde_json::from_value)
            .transpose()?,
        resolved_by_user: row.get::<_, Option<Uuid>>(10).map(|u| u.to_string()),
        resolved_at: row.get(11),
        labels: row.get(12),
        annotations: row.get(13),
    })
}

/// Create a new pending resolution
#[instrument(skip(hasura_transaction))]
pub async fn create_tally_session_resolution(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    tally_session_id: &str,
    contest_id: &str,
    resolution_type: TallySessionResolutionType,
    resolution_data: TallySessionResolutionData,
) -> Result<String> {
    let query = r#"
        INSERT INTO sequent_backend.tally_session_resolution
        (tenant_id, election_event_id, tally_session_id, contest_id, resolution_type, status, resolution_data)
        VALUES ($1, $2, $3, $4, $5, 'pending', $6)
        RETURNING id
    "#;

    let resolution_data_value = serde_json::to_value(&resolution_data)?;
    let row = hasura_transaction
        .query_one(
            query,
            &[
                &Uuid::parse_str(tenant_id)?,
                &Uuid::parse_str(election_event_id)?,
                &Uuid::parse_str(tally_session_id)?,
                &Uuid::parse_str(contest_id)?,
                &resolution_type.to_string(),
                &resolution_data_value,
            ],
        )
        .await?;

    let id: Uuid = row.get(0);
    info!(
        "Created resolution {} for tally session {} contest {}",
        id, tally_session_id, contest_id
    );
    Ok(id.to_string())
}

/// Get pending resolutions for a tally session
#[instrument(skip(hasura_transaction))]
pub async fn get_pending_resolutions(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    tally_session_id: &str,
) -> Result<Vec<TallySessionResolution>> {
    let query = r#"
        SELECT
            id, tenant_id, election_event_id, tally_session_id,
            contest_id,
            created_at, last_updated_at, resolution_type, status,
            resolution_data, resolved_by_user, resolved_at,
            labels, annotations
        FROM sequent_backend.tally_session_resolution
        WHERE tenant_id = $1
          AND election_event_id = $2
          AND tally_session_id = $3
          AND status = 'pending'
        ORDER BY created_at ASC
    "#;

    let rows = hasura_transaction
        .query(
            query,
            &[
                &Uuid::parse_str(tenant_id)?,
                &Uuid::parse_str(election_event_id)?,
                &Uuid::parse_str(tally_session_id)?,
            ],
        )
        .await?;

    let mut resolutions = Vec::new();
    for row in rows {
        resolutions.push(map_row_to_resolution(&row)?);
    }

    Ok(resolutions)
}

/// Get all resolutions for a tally session (both pending and resolved)
#[instrument(skip(hasura_transaction))]
pub async fn get_resolution_by_tally_session(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    tally_session_id: &str,
) -> Result<Vec<TallySessionResolution>> {
    let query = r#"
        SELECT
            id, tenant_id, election_event_id, tally_session_id,
            contest_id,
            created_at, last_updated_at, resolution_type, status,
            resolution_data, resolved_by_user, resolved_at,
            labels, annotations
        FROM sequent_backend.tally_session_resolution
        WHERE tenant_id = $1
          AND election_event_id = $2
          AND tally_session_id = $3
        ORDER BY created_at ASC
    "#;

    let rows = hasura_transaction
        .query(
            query,
            &[
                &Uuid::parse_str(tenant_id)?,
                &Uuid::parse_str(election_event_id)?,
                &Uuid::parse_str(tally_session_id)?,
            ],
        )
        .await?;

    let mut resolutions = Vec::new();
    for row in rows {
        resolutions.push(map_row_to_resolution(&row)?);
    }

    Ok(resolutions)
}

/// Submit a resolution for a pending item
#[instrument(skip(hasura_transaction))]
pub async fn submit_resolution(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    resolution_id: &str,
    resolution: TallySessionResolutionData,
    resolved_by_user: &str,
) -> Result<()> {
    let query = r#"
        UPDATE sequent_backend.tally_session_resolution
        SET resolution_data = $1,
            status = 'resolved',
            resolved_by_user = $2,
            resolved_at = NOW()
        WHERE id = $3
          AND tenant_id = $4
          AND election_event_id = $5
          AND status = 'pending'
    "#;

    let resolution_value = serde_json::to_value(&resolution)?;
    let affected = hasura_transaction
        .execute(
            query,
            &[
                &resolution_value,
                &Uuid::parse_str(resolved_by_user)?,
                &Uuid::parse_str(resolution_id)?,
                &Uuid::parse_str(tenant_id)?,
                &Uuid::parse_str(election_event_id)?,
            ],
        )
        .await?;

    if affected == 0 {
        return Err(anyhow!(
            "Resolution not found or already resolved: {}",
            resolution_id
        ));
    }

    info!("Submitted resolution {}", resolution_id);
    Ok(())
}

/// Update the resolution decision for an already-resolved record
#[instrument(skip(hasura_transaction))]
pub async fn update_resolution(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    resolution_id: &str,
    resolution: TallySessionResolutionData,
    resolved_by_user: &str,
) -> Result<()> {
    let query = r#"
        UPDATE sequent_backend.tally_session_resolution
        SET resolution_data = $1,
            resolved_by_user = $2,
            resolved_at = NOW()
        WHERE id = $3
          AND tenant_id = $4
          AND election_event_id = $5
    "#;

    let resolution_value = serde_json::to_value(&resolution)?;
    let affected = hasura_transaction
        .execute(
            query,
            &[
                &resolution_value,
                &Uuid::parse_str(resolved_by_user)?,
                &Uuid::parse_str(resolution_id)?,
                &Uuid::parse_str(tenant_id)?,
                &Uuid::parse_str(election_event_id)?,
            ],
        )
        .await?;

    if affected == 0 {
        return Err(anyhow!("Resolution not found: {}", resolution_id));
    }

    info!("Updated resolution {}", resolution_id);
    Ok(())
}
