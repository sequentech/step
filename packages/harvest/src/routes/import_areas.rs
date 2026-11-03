// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::audit;
use crate::services::authorization::authorize;
use crate::services::dependencies::HarvestServices;
use anyhow::Result;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use sequent_core::services::jwt;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::{event, instrument, Level};
use windmill::{
    services::audit_log::stage,
    services::providers::transactions_provider::provide_hasura_transaction,
    services::task_audit,
    tasks::audit_log_outbox::kick_audit_log_outbox,
    tasks::{import_areas::import_areas_task, upsert_areas::upsert_areas_task},
};

#[derive(Serialize, Deserialize, Debug)]
pub struct ImportAreasInput {
    election_event_id: String,
    document_id: String,
    sha256: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ImportAreasOutput {}

const AREA: &str = "area";

#[instrument(skip(services, claims))]
#[post("/upsert-areas", format = "json", data = "<input>")]
pub async fn upsert_areas_route(
    claims: jwt::JwtClaims,
    input: Json<ImportAreasInput>,
    services: &State<HarvestServices>,
) -> Result<Json<ImportAreasOutput>, (Status, String)> {
    let body = input.into_inner();
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::AREA_WRITE],
    )?;
    upsert_areas_task(
        claims.hasura_claims.tenant_id.clone(),
        body.election_event_id.clone(),
        body.document_id.clone(),
    )
    .await
    .map_err(|error| {
        (
            Status::InternalServerError,
            format!("Error importing areas: {error:?}"),
        )
    })?;
    let entry = audit::imported(
        &claims,
        audit::target(
            &claims.hasura_claims.tenant_id,
            Some(&body.election_event_id),
        )?,
        AREA,
        &body.document_id,
        body.sha256.as_deref(),
        "Updated the parents of areas from a file",
    );
    // The task commits on its own; its entry follows the change.
    audit::record(services, &entry).await?;

    Ok(Json(ImportAreasOutput {}))
}

#[instrument(skip(claims))]
#[post("/import-areas", format = "json", data = "<input>")]
pub async fn import_areas_route(
    claims: jwt::JwtClaims,
    input: Json<ImportAreasInput>,
) -> Result<Json<ImportAreasOutput>, (Status, String)> {
    let body = input.into_inner();
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::AREA_WRITE],
    )?;

    provide_hasura_transaction(|hasura_transaction| {
        let tenant_id = claims.hasura_claims.tenant_id.clone();
        let election_event_id = body.election_event_id.clone();
        let document_id = body.document_id.clone();
        Box::pin(async move {
            // An invalid event id fails the import before its entry.
            import_areas_task(
                hasura_transaction,
                tenant_id.clone(),
                election_event_id.clone(),
                document_id.clone(),
                body.sha256.clone(),
            )
            .await?;
            let entry = audit::imported(
                &claims,
                task_audit::target(&tenant_id, Some(&election_event_id))?,
                AREA,
                &document_id,
                body.sha256.as_deref(),
                "Imported areas",
            );
            stage(hasura_transaction, &entry).await?;
            Ok(())
        })
    })
    .await
    .map_err(|error| {
        (
            Status::InternalServerError,
            format!("Error importing areas: {error:?}"),
        )
    })?;
    kick_audit_log_outbox();

    Ok(Json(ImportAreasOutput {}))
}
