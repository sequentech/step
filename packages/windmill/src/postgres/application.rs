// SPDX-FileCopyrightText: 2024 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::collections::HashMap;

use crate::{
    services::{application::ApplicationAnnotations, reports::voters::EnrollmentFilters},
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
pub async fn get_permission_label_from_post(
    hasura_transaction: &Transaction<'_>,
    post: &str,
) -> Result<Option<String>> {
    let query = r#"
        SELECT el.permission_label
        FROM sequent_backend.area a
            LEFT JOIN sequent_backend.area_contest ac ON a.id = ac.area_id
            LEFT JOIN sequent_backend.contest con ON ac.contest_id = con.id
            LEFT JOIN sequent_backend.election el ON con.election_id = el.id
        WHERE
            a.description ILIKE $1
        LIMIT 1
        "#;

    let statement = hasura_transaction
        .prepare(query)
        .await
        .map_err(|err| anyhow!("Error preparing the application query: {err}"))?;

    let row = hasura_transaction
        .query_opt(&statement, &[&post])
        .await
        .map_err(|err| anyhow!("Error querying applications: {err}"))?;

    let result = row.and_then(|row| row.get("permission_label"));

    Ok(result)
}

#[instrument(err, skip_all)]
pub async fn insert_application(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    area_id: &Option<String>,
    applicant_id: &str,
    applicant_data: &HashMap<String, String>,
    labels: &Option<Value>,
    annotations: &ApplicationAnnotations,
    verification_type: &ApplicationType,
    status: &ApplicationStatus,
    permission_label: &Option<String>,
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
                status,
                permission_label
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
                $9,
                $10
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
                &serde_json::to_value(applicant_data)?,
                &labels,
                &serde_json::to_value(annotations)?,
                &verification_type.to_string(),
                &status.to_string(),
                &permission_label,
            ],
        )
        .await
        .map_err(|err| anyhow!("Error inserting application: {err}"))?;

    Ok(())
}

#[instrument(err, skip_all)]
pub async fn update_application_status(
    hasura_transaction: &Transaction<'_>,
    id: &str,
    tenant_id: &str,
    election_event_id: &str,
    applicant_id: &str,
    status: ApplicationStatus,
    rejection_reason: Option<String>,
    rejection_message: Option<String>,
    admin_name: &str,
) -> Result<Application> {
    // Base query structure
    let base_query = r#"
        UPDATE
            sequent_backend.applications
        SET
            status = $1,
            applicant_id = $2,
            updated_at = NOW(),
            annotations = {}
        WHERE
            id = $3 AND
            tenant_id = $4 AND
            election_event_id = $5
        RETURNING *;
    "#;

    // Build annotations update dynamically
    let annotations_update = {
        let mut update = "COALESCE(annotations, '{}'::jsonb)".to_string();
        update = format!(
            "jsonb_set({}, '{{verified_by}}', to_jsonb($6::text), true)",
            update
        );
        if rejection_reason.is_some() {
            update = format!(
                "jsonb_set({}, '{{rejection_reason}}', to_jsonb($7::text), true)",
                update
            );
        }
        if rejection_message.is_some() {
            update = format!(
                "jsonb_set({}, '{{rejection_message}}', to_jsonb($8::text), true)",
                update
            );
        }
        update
    };

    // Finalize the query
    let query = base_query.replace("{}", &annotations_update);

    // Prepare the statement
    let statement = hasura_transaction
        .prepare(&query)
        .await
        .map_err(|err| anyhow!("Error preparing the update query: {err}"))?;

    // Parse UUIDs
    let status_str = status.to_string();
    let parsed_id = Uuid::parse_str(id)?;
    let parsed_tenant_id = Uuid::parse_str(tenant_id)?;
    let parsed_election_event_id = Uuid::parse_str(election_event_id)?;

    // Build parameter list
    let mut params: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = vec![
        &status_str,
        &applicant_id,
        &parsed_id,
        &parsed_tenant_id,
        &parsed_election_event_id,
        &admin_name,
    ];
    if let Some(reason) = &rejection_reason {
        params.push(reason);
    }
    if let Some(message) = &rejection_message {
        params.push(message);
    }

    // Execute the query
    let rows: Vec<Row> = hasura_transaction
        .query(&statement, &params)
        .await
        .map_err(|err| anyhow!("Error updating application: {err}"))?;

    let results: Vec<Application> = rows
        .into_iter()
        .map(|row| -> Result<Application> {
            row.try_into()
                .map(|res: ApplicationWrapper| -> Application { res.0 })
        })
        .collect::<Result<Vec<Application>>>()?;

    // Return the updated application or error if none found
    let application = results
        .get(0)
        .map(|element: &Application| element.clone())
        .ok_or(anyhow!(
            "Error updating application: No application with id {id} found."
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
    let verification_type;
    if let Some(filters) = filters {
        query.push_str(" AND status = $4");
        status = filters.clone().status.to_string();
        params.push(&status);

        if filters.verification_type.is_some() {
            query.push_str(" AND verification_type = $5");
            verification_type =
                <std::option::Option<ApplicationType> as Clone>::clone(&filters.verification_type)
                    .unwrap()
                    .to_string();
            params.push(&verification_type);
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

#[instrument(err, skip_all)]
pub async fn count_applications(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    area_id: Option<&str>,
    filters: Option<&EnrollmentFilters>,
) -> Result<i64> {
    let mut current_param_place = 3;
    let area_clause = match area_id {
        Some(area_id) => {
            current_param_place += 1;
            format!("AND area_id = $3 ")
        }
        None => "".to_string(),
    };
    let mut query = format!(
        r#"
        SELECT COUNT(*)
        FROM sequent_backend.applications
        WHERE 
          tenant_id = $1
          AND election_event_id = $2
          {area_clause}
    "#
    );

    let parsed_tenant_id = Uuid::parse_str(tenant_id)?;
    let parsed_election_event_id = Uuid::parse_str(election_event_id)?;

    let mut params: Vec<&(dyn ToSql + Sync)> = vec![&parsed_tenant_id, &parsed_election_event_id];

    let mut optional_area_id: Option<Uuid> = None; // Declare the variable outside the match

    if let Some(area_id) = area_id {
        let parsed_area_id = Uuid::parse_str(area_id)?;
        optional_area_id = Some(parsed_area_id); // Store the value in the variable
    }

    if let Some(ref area_id) = optional_area_id {
        params.push(area_id); // Push the reference to the vector
    }

    // Apply filters if provided
    let status;
    let verification_type;
    if let Some(filters) = filters {
        let place = current_param_place.to_string();
        query.push_str(&format!("AND status = ${place} "));
        status = filters.clone().status.to_string();
        params.push(&status);
        current_param_place += 1;

        if filters.verification_type.is_some() {
            let place = current_param_place.to_string();
            query.push_str(&format!("AND verification_type = ${place}"));
            verification_type =
                <std::option::Option<ApplicationType> as Clone>::clone(&filters.verification_type)
                    .unwrap()
                    .to_string();
            params.push(&verification_type);
        }
    }

    let statement = hasura_transaction
        .prepare(&query)
        .await
        .map_err(|err| anyhow!("Error preparing the application query: {err}"))?;

    let row: Row = hasura_transaction
        .query_one(&statement, &params)
        .await
        .map_err(|err| anyhow!("Error querying applications: {err}"))?;

    let count: i64 = row.get(0);

    Ok(count)
}

#[instrument(err, skip_all)]
pub async fn get_applications_by_election(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: Option<&str>,
) -> Result<Vec<Application>> {
    let mut query = r#"
        SELECT *
        FROM sequent_backend.applications
        WHERE tenant_id = $1
          AND election_event_id = $2
    "#
    .to_string();

    let parsed_tenant_id = Uuid::parse_str(tenant_id)?;
    let parsed_election_event_id = Uuid::parse_str(election_event_id)?;

    let mut params: Vec<&(dyn ToSql + Sync)> = vec![&parsed_tenant_id, &parsed_election_event_id];

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

#[instrument(err, skip_all)]
pub async fn insert_applications(
    hasura_transaction: &Transaction<'_>,
    applications: &[Application],
) -> Result<()> {
    let statement = hasura_transaction
        .prepare(
            r#"
            INSERT INTO sequent_backend.applications
            (
                id,
                created_at,
                updated_at,
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
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12
            );
            "#,
        )
        .await
        .map_err(|err| anyhow!("Error preparing the insert applications query: {err}"))?;

    for application in applications {
        let area_id = application
            .area_id
            .as_ref()
            .map(|id| Uuid::parse_str(id))
            .transpose()?;
        hasura_transaction
            .execute(
                &statement,
                &[
                    &Uuid::parse_str(&application.id)?,
                    &application.created_at,
                    &application.updated_at,
                    &Uuid::parse_str(&application.tenant_id)?,
                    &Uuid::parse_str(&application.election_event_id)?,
                    &area_id,
                    &application.applicant_id,
                    &application.applicant_data,
                    &application.labels,
                    &application.annotations,
                    &application.verification_type.to_string(),
                    &application.status.to_string(),
                ],
            )
            .await
            .map_err(|err| anyhow!("Error inserting application: {err}"))?;
    }

    Ok(())
}
