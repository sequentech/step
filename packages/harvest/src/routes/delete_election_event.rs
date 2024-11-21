// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use anyhow::Result;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::{event, instrument, Level};
use windmill::services::celery_app::get_celery_app;
use windmill::tasks::delete_election_event;

#[derive(Serialize, Deserialize, Debug)]
pub struct DeleteElectionEventOutput {
    id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DeleteElectionEventInput {
    election_event_id: String,
}

#[instrument(skip(claims))]
#[post("/delete-election-event", format = "json", data = "<body>")]
pub async fn delete_election_event_f(
    body: Json<DeleteElectionEventInput>,
    claims: JwtClaims,
) -> Result<Json<DeleteElectionEventOutput>, (Status, String)> {
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::ELECTION_EVENT_DELETE],
    )?;
    let celery_app = get_celery_app().await;
    let input = body.into_inner();

    let realm = get_event_realm(
        &claims.hasura_claims.tenant_id,
        &input.election_event_id,
    );

    let task = celery_app
        .send_task(delete_election_event::delete_election_event_t::new(
            claims.hasura_claims.tenant_id.clone(),
            input.election_event_id.clone(),
            realm.clone(),
        ))
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    event!(
        Level::INFO,
        "Sent DELETE_ELECTION_EVENT task {}",
        task.task_id
    );

    Ok(Json(DeleteElectionEventOutput {
        id: input.election_event_id,
    }))
}
