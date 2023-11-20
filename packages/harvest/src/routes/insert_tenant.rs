// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use rocket::response::Debug;
use rocket::serde::json::Json;
use sequent_core::services::connection;
use serde::{Deserialize, Serialize};
use tracing::{event, instrument, Level};
use uuid::Uuid;
use windmill::services::celery_app::get_celery_app;
use windmill::tasks;

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateTenantInput {
    slug: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateTenantOutput {
    id: String,
    slug: String,
}

#[instrument(skip(auth_headers))]
#[post("/insert-tenant", format = "json", data = "<body>")]
pub async fn insert_tenant(
    body: Json<CreateTenantInput>,
    auth_headers: connection::AuthHeaders,
) -> Result<Json<CreateTenantOutput>, Debug<anyhow::Error>> {
    let celery_app = get_celery_app().await;
    // always set an id;
    let id = Uuid::new_v4().to_string();
    let task = celery_app
        .send_task(tasks::insert_tenant::insert_tenant::new(
            id.clone(),
            body.slug.clone(),
        ))
        .await
        .map_err(|e| anyhow::Error::from(e))?;
    event!(Level::INFO, "Sent INSERT_TENANT task {}", task.task_id);

    Ok(Json(CreateTenantOutput {
        id,
        slug: body.slug.clone(),
    }))
}
