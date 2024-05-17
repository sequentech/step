// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use anyhow::Result;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{event, instrument, Level};
use uuid::Uuid;
use windmill::hasura::election_event::insert_election_event::sequent_backend_election_event_insert_input as InsertElectionEventInput;
use windmill::services::celery_app::get_celery_app;
use windmill::tasks::import_election_event;
use windmill::tasks::insert_election_event;

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateElectionEventOutput {
    id: String,
}

#[instrument(skip(claims))]
#[post("/insert-election-event", format = "json", data = "<body>")]
pub async fn insert_election_event_f(
    body: Json<InsertElectionEventInput>,
    claims: JwtClaims,
) -> Result<Json<CreateElectionEventOutput>, (Status, String)> {
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::ELECTION_EVENT_CREATE],
    )?;
    let celery_app = get_celery_app().await;
    // always set an id;
    let object = body.into_inner().clone();
    let id = object.id.clone().unwrap_or(Uuid::new_v4().to_string());
    let task = celery_app
        .send_task(insert_election_event::insert_election_event_t::new(
            object,
            id.clone(),
        ))
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    event!(
        Level::INFO,
        "Sent INSERT_ELECTION_EVENT task {}",
        task.task_id
    );

    Ok(Json(CreateElectionEventOutput { id }))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ImportElectionEventOutput {
    id: Option<String>,
    message: Option<String>,
    error: Option<String>,
}

#[instrument(skip(claims))]
#[post("/import-election-event", format = "json", data = "<body>")]
pub async fn import_election_event_f(
    body: Json<import_election_event::ImportElectionEventBody>,
    claims: JwtClaims,
) -> Result<Json<ImportElectionEventOutput>, (Status, String)> {
    let input = body.into_inner();

    authorize(&claims, true, Some(input.tenant_id.clone()), vec![])?;

    let document_result = import_election_event::get_document(
        input.clone(),
        None,
        input.tenant_id.clone(),
    )
    .await;

    if let Err(err) = document_result {
        return Ok(Json(ImportElectionEventOutput {
            id: None,
            message: None,
            error: Some(format!("Error checking import: {:?}", err)),
        }));
    }

    let document = document_result.unwrap();
    let id = document.election_event_data.id.clone();

    let check_only = input.check_only.unwrap_or(false);

    if check_only {
        return Ok(Json(ImportElectionEventOutput {
            id: Some(id),
            message: Some(format!("Import document checked")),
            error: None,
        }));
    }

    let celery_app = get_celery_app().await;
    let task = celery_app
        .send_task(import_election_event::import_election_event::new(
            input.clone(),
            id.clone(),
            input.tenant_id.clone(),
        ))
        .await
        .map_err(|err| {
            (
                Status::InternalServerError,
                format!("Error sending import_election_event task: {:?}", err),
            )
        })?;

    info!("Sent IMPORT_USERS task {}", task.task_id);

    Ok(Json(ImportElectionEventOutput {
        id: Some(id),
        message: Some(format!("Task created: import_election_event")),
        error: None,
    }))
}
