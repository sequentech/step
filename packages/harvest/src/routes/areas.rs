// SPDX-FileCopyrightText: 2025 Enric Badia <enric@xtremis.com>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use anyhow::{Context, Result};
use chrono;
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::hasura::core::Area;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use tracing::{event, instrument, Level};
use uuid::Uuid;
use windmill::postgres::area::{
    delete_area_contests, insert_area_contests, insert_area, update_area,
};
use windmill::services::database::get_hasura_pool;
use windmill::services::import::import_election_event::upsert_b3_and_elog;

#[derive(Serialize, Deserialize, Debug)]
pub struct UpsertAreaInput {
    pub id: Option<Uuid>,
    pub name: String,
    pub description: String,
    pub election_event_id: Uuid,
    pub tenant_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub area_contest_ids: Vec<Uuid>,
    pub annotations: Option<JsonValue>,
    pub labels: Option<JsonValue>,
    pub r#type: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UpsertAreaOutput {
    id: String,
}

#[instrument(skip(claims))]
#[post("/upsert-area", format = "json", data = "<body>")]
pub async fn upsert_area(
    body: Json<UpsertAreaInput>,
    claims: JwtClaims,
) -> Result<Json<UpsertAreaOutput>, (Status, String)> {
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::ELECTION_EVENT_WRITE],
    )?;

    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    let now_local = chrono::Utc::now().with_timezone(&chrono::Local);
    let area = Area {
        id: body
            .id
            .map(|uuid| uuid.to_string())
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
        tenant_id: body.tenant_id.to_string(),
        election_event_id: body.election_event_id.to_string(),
        labels: body.labels.clone(),
        annotations: body.annotations.clone(),
        name: Some(body.name.clone()),
        description: Some(body.description.clone()),
        r#type: body.r#type.clone(),
        parent_id: body.parent_id.map(|uuid| uuid.to_string()),
        created_at: Some(now_local),
        last_updated_at: Some(now_local),
    };

    // Perform insert or update based on presence of ID
    if body.id.is_some() {
        update_area(&hasura_transaction, area.clone())
            .await
            .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    } else {
        insert_area(&hasura_transaction, area.clone())
            .await
            .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    }

    // Parse UUIDs without using the ? operator
    let area_id = match Uuid::parse_str(&area.id) {
        Ok(id) => id,
        Err(e) => {
            return Err((
                Status::InternalServerError,
                format!("Invalid area ID: {:?}", e),
            ))
        }
    };
    let election_event_id = body.election_event_id;
    let tenant_id = match Uuid::parse_str(&claims.hasura_claims.tenant_id) {
        Ok(id) => id,
        Err(e) => {
            return Err((
                Status::InternalServerError,
                format!("Invalid tenant ID: {:?}", e),
            ))
        }
    };

    delete_area_contests(
        &hasura_transaction,
        &area_id,
        &election_event_id,
        &tenant_id,
    )
    .await
    .map_err(|e| {
        (
            Status::InternalServerError,
            format!("Failed to insert area_contests: {:?}", e),
        )
    })?;

    insert_area_contests(
        &hasura_transaction,
        &area_id,
        &body.area_contest_ids,
        &election_event_id,
        &tenant_id,
    )
    .await
    .map_err(|e| {
        (
            Status::InternalServerError,
            format!("Failed to insert area_contests: {:?}", e),
        )
    })?;

    upsert_b3_and_elog(
        &hasura_transaction,
        &claims.hasura_claims.tenant_id,
        &body.election_event_id.to_string(),
        &vec![area.id.clone()],
        false,
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    hasura_transaction
        .commit()
        .await
        .with_context(|| "error comitting transaction")
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    Ok(Json(UpsertAreaOutput { id: area.id }))
}
