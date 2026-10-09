// SPDX-FileCopyrightText: 2025 Enric Badia <enric@xtremis.com>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use anyhow::{Context, Result};
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::hasura::core::Area;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use tracing::instrument;
use uuid::Uuid;
use windmill::postgres::area::{
    delete_area_contests, insert_area, update_area,
};
use windmill::postgres::area_contest::insert_area_to_area_contests;
use windmill::postgres::election_event::get_election_event_by_id_if_exist;
use windmill::services::database::get_hasura_pool;
use windmill::services::import::import_election_event::upsert_b3_and_elog;

#[derive(Serialize, Deserialize, Debug)]
pub struct UpsertAreaInput {
    pub id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
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

fn authorize_upsert_area(
    claims: &JwtClaims,
    input: &UpsertAreaInput,
) -> Result<(), (Status, String)> {
    authorize(
        claims,
        true,
        Some(input.tenant_id.to_string()),
        vec![Permissions::AREA_CREATE],
    )
}

#[instrument(skip(claims))]
#[post("/upsert-area", format = "json", data = "<body>")]
pub async fn upsert_area(
    body: Json<UpsertAreaInput>,
    claims: JwtClaims,
) -> Result<Json<UpsertAreaOutput>, (Status, String)> {
    authorize_upsert_area(&claims, &body)?;

    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;

    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;

    let tenant_id = body.tenant_id.to_string();
    let election_event_id_str = body.election_event_id.to_string();

    get_election_event_by_id_if_exist(
        &hasura_transaction,
        &tenant_id,
        &election_event_id_str,
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?
    .ok_or_else(|| {
        (
            Status::NotFound,
            format!("Election event {election_event_id_str} not found"),
        )
    })?;

    let area = Area {
        id: body
            .id
            .map(|uuid| uuid.to_string())
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
        tenant_id: tenant_id.clone(),
        election_event_id: election_event_id_str.clone(),
        labels: body.labels.clone(),
        annotations: body.annotations.clone(),
        name: Some(body.name.clone()),
        description: body.description.clone(),
        r#type: body.r#type.clone(),
        parent_id: body.parent_id.map(|uuid| uuid.to_string()),
        created_at: None,
        last_updated_at: None,
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
    delete_area_contests(
        &hasura_transaction,
        &tenant_id,
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
        &tenant_id,
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
        &tenant_id,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::authorization::test_claims::{
        admin_claims, CALLER_TENANT_ID, OTHER_TENANT_ID,
    };

    fn input(tenant_id: &str) -> UpsertAreaInput {
        UpsertAreaInput {
            id: None,
            name: "area".to_string(),
            description: None,
            election_event_id: Uuid::new_v4(),
            tenant_id: Uuid::parse_str(tenant_id)
                .expect("test tenant id must be a UUID"),
            parent_id: None,
            area_contest_ids: vec![],
            annotations: None,
            labels: None,
            r#type: None,
            allow_early_voting: None,
        }
    }

    #[test]
    fn upsert_area_rejects_tenant_other_than_callers() {
        let claims = admin_claims(CALLER_TENANT_ID, &["area-create"]);
        let result = authorize_upsert_area(&claims, &input(OTHER_TENANT_ID));
        assert_eq!(result.unwrap_err().0, Status::Unauthorized);
    }

    #[test]
    fn upsert_area_accepts_callers_tenant() {
        let claims = admin_claims(CALLER_TENANT_ID, &["area-create"]);
        assert!(
            authorize_upsert_area(&claims, &input(CALLER_TENANT_ID)).is_ok()
        );
    }
}
