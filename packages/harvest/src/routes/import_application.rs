// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use anyhow::Result;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::{event, info, instrument, Level};
use windmill::{
    services::providers::transactions_provider::provide_hasura_transaction,
    tasks::import_application::import_applications_task,
};

#[derive(Serialize, Deserialize, Debug)]
pub struct ImportApplicationsInput {
    tenant_id: String,
    election_event_id: String,
    election_id: Option<String>,
    document_id: String,
    sha256: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ImportApplicationsOutput {
    error_msg: String,
    document_id: String,
}

#[instrument(skip(claims))]
#[post("/import-application", format = "json", data = "<input>")]
pub async fn import_application_route(
    claims: jwt::JwtClaims,
    input: Json<ImportApplicationsInput>,
) -> Result<Json<ImportApplicationsOutput>, (Status, String)> {
    let body = input.into_inner();
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::APPLICATION_IMPORT],
    )?;

    provide_hasura_transaction(|hasura_transaction| {
        let tenant_id = claims.hasura_claims.tenant_id.clone();
        let document_id = body.document_id.clone();
        let election_event_id = body.election_event_id.clone();
        Box::pin(async move {
            import_applications_task(
                hasura_transaction,
                tenant_id,
                election_event_id,
                document_id,
            )
            .await
        })
    })
    .await
    .map_err(|error| {
        (
            Status::InternalServerError,
            format!("Error importing applications: {error:?}"),
        )
    })?;

    Ok(Json(ImportApplicationsOutput {
        error_msg: String::new(),
        document_id: body.document_id,
    }))
}
