// SPDX-FileCopyrightText: 2025 Enric Badia <enric@xtremis.com>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use anyhow::{Context, Result};
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::ballot::{AreaPresentation, EarlyVotingPolicy};
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::hasura::core::Area;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use tracing::{info, instrument};
use uuid::Uuid;
use windmill::postgres::area::{
    delete_area_contests, insert_area, update_area,
};
use windmill::postgres::area_contest::insert_area_to_area_contests;
use windmill::services::database::get_hasura_pool;
use windmill::services::import::import_election_event::upsert_b3_and_elog;

/// Request body for creating or updating an area.
#[derive(Serialize, Deserialize, Debug)]
pub struct UpsertAreaInput {
    /// Optional area ID; if not provided, a new ID will be generated
    pub id: Option<Uuid>,
    /// Area name
    pub name: String,
    /// Optional area description
    pub description: Option<String>,
    /// The election event this area belongs to
    pub election_event_id: Uuid,
    /// The tenant this area belongs to
    pub tenant_id: Uuid,
    /// Optional parent area ID for hierarchical structure
    pub parent_id: Option<Uuid>,
    /// Associated area contest IDs
    pub area_contest_ids: Vec<Uuid>,
    /// Optional annotations for the area
    pub annotations: Option<JsonValue>,
    /// Optional labels for the area
    pub labels: Option<JsonValue>,
    /// Optional area type
    pub r#type: Option<String>,
    /// Optional early voting policy
    pub allow_early_voting: Option<EarlyVotingPolicy>,
}

/// Response containing the ID of the created or updated area.
#[derive(Serialize, Deserialize, Debug)]
pub struct UpsertAreaOutput {
    /// The area ID
    id: String,
}

/// Creates or updates an area in an election event.
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
        vec![Permissions::AREA_CREATE],
    )?;

    info!("Policy: {:#?}", body.allow_early_voting);
    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;

    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;

    let election_event_id_str = body.election_event_id.to_string();

    let presentation = serde_json::to_value(AreaPresentation {
        allow_early_voting: body.allow_early_voting,
    })
    .map_err(|e| {
        (
            Status::InternalServerError,
            format!("Error serializing AreaPresentation: {e:?}"),
        )
    })?;
    let area = Area {
        id: body.id.map_or_else(
            || uuid::Uuid::new_v4().to_string(),
            |uuid| uuid.to_string(),
        ),
        tenant_id: body.tenant_id.to_string(),
        election_event_id: election_event_id_str.clone(),
        labels: body.labels.clone(),
        annotations: body.annotations.clone(),
        name: Some(body.name.clone()),
        description: body.description.clone(),
        r#type: body.r#type.clone(),
        parent_id: body.parent_id.map(|uuid| uuid.to_string()),
        created_at: None,
        last_updated_at: None,
        presentation: Some(presentation),
    };

    // Perform insert or update based on presence of ID
    if body.id.is_some() {
        update_area(&hasura_transaction, area.clone())
            .await
            .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;
    } else {
        insert_area(&hasura_transaction, area.clone())
            .await
            .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;
    }
    let tenant_id = &claims.hasura_claims.tenant_id;
    delete_area_contests(
        &hasura_transaction,
        tenant_id,
        &body.election_event_id,
        &area.id,
    )
    .await
    .map_err(|e| {
        (
            Status::InternalServerError,
            format!("Failed to insert area_contests: {e:?}"),
        )
    })?;

    insert_area_to_area_contests(
        &hasura_transaction,
        tenant_id,
        &election_event_id_str,
        &area.id,
        &body.area_contest_ids,
    )
    .await
    .map_err(|e| {
        (
            Status::InternalServerError,
            format!("Failed to insert area_contests: {e:?}"),
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
    .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;

    hasura_transaction
        .commit()
        .await
        .with_context(|| "error comitting transaction")
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;

    Ok(Json(UpsertAreaOutput { id: area.id }))
}
