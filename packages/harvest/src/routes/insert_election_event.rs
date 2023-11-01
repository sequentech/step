// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use rocket::response::Debug;
use rocket::serde::json::Json;
use sequent_core::services::connection;
use tracing::{event, instrument, Level};
use windmill::hasura::election_event::insert_election_event::sequent_backend_election_event_insert_input as InsertElectionEventInput;
use windmill::services::celery_app::get_celery_app;
use windmill::tasks::insert_election_event;

#[instrument(skip(auth_headers))]
#[post("/insert-election-event", format = "json", data = "<body>")]
pub async fn insert_election_event_f(
    body: Json<InsertElectionEventInput>,
    auth_headers: connection::AuthHeaders,
) -> Result<Json<()>, Debug<anyhow::Error>> {
    let celery_app = get_celery_app().await;
    let task = celery_app
        .send_task(insert_election_event::insert_election_event_t::new(
            body.into_inner().clone(),
        ))
        .await
        .map_err(|e| anyhow::Error::from(e))?;
    event!(
        Level::INFO,
        "Sent INSERT_ELECTION_EVENT task {}",
        task.task_id
    );

    Ok(Json(()))
}
