// SPDX-FileCopyrightText: 2024 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::{
    services::reports::voters::EnrollmentFilters,
    types::application::{ApplicationStatus, ApplicationType},
};
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::types::hasura::core::Application;
use serde_json::Value;
use tokio_postgres::row::Row;
use tokio_postgres::types::ToSql;
use tracing::{event, instrument, Level};
use uuid::Uuid;

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
            area_id: item
                .try_get::<_, Uuid>("area_id")
                .map(|value| value.to_string())
                .ok(),
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
    area_id: &Option<String>,
    applicant_id: &str,
    applicant_data: &Value,
    labels: &Option<Value>,
    annotations: &Option<Value>,
    verification_type: &ApplicationType,
    status: &ApplicationStatus,
) -> Result<()> {
    let area_id = if let Some(area_id) = area_id {
        Some(Uuid::parse_str(area_id)?)
    } else {
        None
    };

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
                &area_id,
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
    applicant_id: &str,
    status: ApplicationStatus,
) -> Result<Application> {
    let statement = hasura_transaction
        .prepare(
            r#"
                UPDATE
                    sequent_backend.applications
                SET
                    status = $1,
                    applicant_id = $2,
                    updated_at = NOW()
                WHERE
                    id = $3 AND
                    tenant_id = $4 AND
                    election_event_id = $5
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
                &applicant_id,
                &Uuid::parse_str(id)?,
                &Uuid::parse_str(tenant_id)?,
                &Uuid::parse_str(election_event_id)?,
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

    let application = results
        .get(0)
        .map(|element: &Application| element.clone())
        .ok_or(anyhow!(
            "Error updating application: No applications with id {id} found."
        ))?;

    Ok(application)
}

#[instrument(err, skip_all)]
pub async fn get_applications(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    area_id: &str,
    filters: Option<&EnrollmentFilters>,
) -> Result<Vec<Application>> {
    let mut query = r#"
        SELECT *
        FROM sequent_backend.applications
        WHERE area_id = $1
          AND tenant_id = $2
          AND election_event_id = $3
    "#
    .to_string();

    let parsed_area_id = Uuid::parse_str(area_id)?;
    let parsed_tenant_id = Uuid::parse_str(tenant_id)?;
    let parsed_election_event_id = Uuid::parse_str(election_event_id)?;

    let mut params: Vec<&(dyn ToSql + Sync)> = vec![
        &parsed_area_id,
        &parsed_tenant_id,
        &parsed_election_event_id,
    ];

    // Apply filters if provided
    let status;
    if let Some(filters) = filters {
        query.push_str(" AND status = $4");
        status = filters.status.to_string();
        params.push(&status);

        if let Some(ref approval_type) = filters.approval_type {
            query.push_str(" AND verification_type = $5");
            params.push(approval_type);
        }
    }

    let statement = hasura_transaction
        .prepare(&query)
        .await
        .map_err(|err| anyhow!("Error preparing the application query: {err}"))?;

    let rows: Vec<Row> = hasura_transaction
        .query(&statement, &params)
        .await
        .map_err(|err| anyhow!("Error querying applications: {err}"))?;

    let results: Vec<Application> = rows
        .into_iter()
        .map(|row| -> Result<Application> {
            row.try_into()
                .map(|res: ApplicationWrapper| -> Application { res.0 })
        })
        .collect::<Result<Vec<Application>>>()?;

    Ok(results)
}
