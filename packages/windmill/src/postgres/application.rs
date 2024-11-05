// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::types::hasura::core::Application;
use serde_json::Value;
use tokio_postgres::row::Row;
use tracing::{event, instrument, Level};
use uuid::Uuid;

use crate::types::application::{ApplicationStatus, ApplicationType};

pub struct ApplicationWrapper(pub Application);

impl TryFrom<Row> for ApplicationWrapper {
    type Error = anyhow::Error;
    fn try_from(item: Row) -> Result<Self> {
        Ok(ApplicationWrapper(Application {
            id: item.try_get::<_, Uuid>("id")?.to_string(),
            created_at: item.get("created_at"),
            updated_at: item.get("updated_at"),
            tenant_id: item.try_get::<_, Uuid>("tenant_id")?.to_string(),
            election_event_id: item.try_get::<_, Uuid>("election_event_id")?.to_string(),
            area_id: item.try_get::<_, Uuid>("area_id")?.to_string(),
            applicant_id: item.try_get("applicant_id")?,
            applicant_data: item.try_get("applicant_data")?,
            labels: item.try_get("labels")?,
            annotations: item.try_get("annotations")?,
            verification_type: item.try_get("verification_type")?,
            status: item.try_get("status")?,
        }))
    }
}

#[instrument(err, skip_all)]
pub async fn insert_application(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    area_id: &str,
    applicant_id: &str,
    applicant_data: &Value,
    labels: &Option<Value>,
    annotations: &Option<Value>,
    verification_type: ApplicationType,
    status: ApplicationStatus,
) -> Result<()> {
    let statement = hasura_transaction
        .prepare(
            r#"
            INSERT INTO sequent_backend.applications
            (
                tenant_id,
                election_event_id,
                area_id,
                applicant_id,
                applicant_data,
                labels,
                annotations,
                verification_type,
                status
            )
            VALUES (
                $1,
                $2,
                $3,
                $4,
                $5,
                $6,
                $7,
                $8,
                $9
            );
            "#,
        )
        .await
        .map_err(|err| anyhow!("Error preparing the insert application query: {err}"))?;

    hasura_transaction
        .execute(
            &statement,
            &[
                &Uuid::parse_str(tenant_id)?,
                &Uuid::parse_str(election_event_id)?,
                &Uuid::parse_str(area_id)?,
                &applicant_id,
                &applicant_data,
                &labels,
                &annotations,
                &verification_type.to_string(),
                &status.to_string(),
            ],
        )
        .await
        .map_err(|err| anyhow!("Error inserting application: {err}"))?;

    Ok(())
}

#[instrument(err, skip_all)]
pub async fn update_confirm_application(
    hasura_transaction: &Transaction<'_>,
    id: &str,
    tenant_id: &str,
    election_event_id: &str,
    area_id: &str,
    status: ApplicationStatus,
) -> Result<Option<Application>> {
    let statement = hasura_transaction
        .prepare(
            r#"
                UPDATE
                    sequent_backend.applications
                SET
                    status = $1
                WHERE
                    id = $2 AND
                    tenant_id = $3 AND
                    election_event_id = $4 AND
                    area_id = $5
                RETURNING *;
            "#,
        )
        .await
        .map_err(|err| anyhow!("Error preparing the confirm application query: {err}"))?;

    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                &status.to_string(),
                &Uuid::parse_str(id)?,
                &Uuid::parse_str(tenant_id)?,
                &Uuid::parse_str(election_event_id)?,
                &Uuid::parse_str(area_id)?,
            ],
        )
        .await
        .map_err(|err| anyhow!("Error confirm application: {err}"))?;

    let results: Vec<Application> = rows
        .into_iter()
        .map(|row| -> Result<Application> {
            row.try_into()
                .map(|res: ApplicationWrapper| -> Application { res.0 })
        })
        .collect::<Result<Vec<Application>>>()?;

    Ok(results.get(0).map(|element: &Application| element.clone()))
}
