// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::ballot::VoterCertificatePolicy;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use uuid::Uuid;
use windmill::postgres::election_event::get_election_event_by_id;
use windmill::services::celery_app::get_celery_app;
use windmill::services::database::get_hasura_pool;
use windmill::services::tasks_execution::post;
use windmill::types::tasks::ETasksExecution;

#[derive(Serialize, Deserialize, Debug)]
pub struct ExportCertificateAuthorityInput {
    ids: Vec<uuid::Uuid>,
    election_event_id: uuid::Uuid,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ExportCertificateAuthorityOutput {
    document_id: String,
    task_execution: TasksExecution,
}

#[instrument(skip(claims, input))]
#[post("/export-certificate-authority", format = "json", data = "<input>")]
pub async fn export_certificate_authority_route(
    claims: JwtClaims,
    input: Json<ExportCertificateAuthorityInput>,
) -> Result<Json<ExportCertificateAuthorityOutput>, (Status, String)> {
    let tenant_id_str = claims.hasura_claims.tenant_id.clone();

    authorize(
        &claims,
        true,
        Some(tenant_id_str.clone()),
        vec![Permissions::CA_READ],
    )?;

    let body = input.into_inner();

    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;

    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;

    let election_event = get_election_event_by_id(
        &hasura_transaction,
        &tenant_id_str,
        &body.election_event_id.to_string(),
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;

    let voter_certificate_policy = election_event
        .get_presentation()
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?
        .unwrap_or_default()
        .voter_certificate_policy
        .unwrap_or_default();

    if voter_certificate_policy != VoterCertificatePolicy::ENABLED {
        return Err((
            Status::Forbidden,
            "Digital certificate authentication is not allowed for this election event".to_string(),
        ));
    }

    hasura_transaction
        .commit()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;

    let executer_name = claims
        .name
        .clone()
        .unwrap_or_else(|| claims.hasura_claims.user_id.clone());

    let task_execution = post(
        &tenant_id_str,
        Some(&body.election_event_id.to_string()),
        ETasksExecution::EXPORT_CERTIFICATE_AUTHORITIES,
        &executer_name,
    )
    .await
    .map_err(|e| {
        (
            Status::InternalServerError,
            format!("Failed to insert task execution record: {e:?}"),
        )
    })?;

    let document_id = Uuid::new_v4().to_string();
    let celery_app = get_celery_app().await;

    celery_app
        .send_task(
            windmill::tasks::export_certificate_authority::export_certificate_authority::new(
                tenant_id_str,
                body.election_event_id,
                body.ids,
                document_id.clone(),
                task_execution.clone(),
            ),
        )
        .await
        .map_err(|e| {
            (
                Status::InternalServerError,
                format!("Error sending export_certificate_authority task: {e:?}"),
            )
        })?;

    Ok(Json(ExportCertificateAuthorityOutput {
        document_id,
        task_execution,
    }))
}
