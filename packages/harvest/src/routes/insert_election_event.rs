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
use tracing::{event, instrument, Level};
use uuid::Uuid;
use windmill::hasura::election_event::insert_election_event::sequent_backend_election_event_insert_input as InsertElectionEventInput;
use windmill::services::celery_app::get_celery_app;
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
        None,
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
