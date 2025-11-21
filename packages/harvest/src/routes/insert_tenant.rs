// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::Permissions;
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

#[instrument(skip(claims))]
#[post("/insert-tenant", format = "json", data = "<body>")]
pub async fn insert_tenant(
    body: Json<CreateTenantInput>,
    claims: JwtClaims,
) -> Result<Json<CreateTenantOutput>, (Status, String)> {
    authorize(&claims, true, None, vec![Permissions::TENANT_CREATE])?;

    let celery_app = get_celery_app().await;
    // always set an id;
    let id = Uuid::new_v4().to_string();
    let task = celery_app
        .send_task(tasks::insert_tenant::insert_tenant::new(
            id.clone(),
            body.slug.clone(),
        ))
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    event!(Level::INFO, "Sent INSERT_TENANT task {}", task.task_id);

    Ok(Json(CreateTenantOutput {
        id,
        slug: body.slug.clone(),
    }))
}
