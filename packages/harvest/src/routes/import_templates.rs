// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::{event, instrument, Level};
use windmill::{
    services::providers::transactions_provider::provide_hasura_transaction,
    tasks::{
        import_templates::import_templates_task,
        upsert_areas::upsert_areas_task,
    },
};

#[derive(Serialize, Deserialize, Debug)]
pub struct ImportTemplatesInput {
    tenant_id: String,
    document_id: String,
    sha256: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ImportTemplatesOutput {
    error_msg: Option<String>,
    document_id: String,
}

#[instrument(skip(claims))]
#[post("/import-templates", format = "json", data = "<input>")]
pub async fn import_templates_route(
    claims: jwt::JwtClaims,
    input: Json<ImportTemplatesInput>,
) -> Result<Json<ImportTemplatesOutput>, (Status, String)> {
    let body = input.into_inner();
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::TEMPLATE_WRITE],
    )?;

    match provide_hasura_transaction(|hasura_transaction| {
        let tenant_id = claims.hasura_claims.tenant_id.clone();
        let document_id = body.document_id.clone();
        Box::pin(async move {
            // Your async code here
            import_templates_task(
                hasura_transaction,
                tenant_id,
                document_id,
                body.sha256.clone(),
            )
            .await?;
            Ok(())
        })
    })
    .await
    {
        Ok(_) => Ok(Json(ImportTemplatesOutput {
            error_msg: None,
            document_id: body.document_id,
        })),
        Err(err) => Ok(Json(ImportTemplatesOutput {
            error_msg: Some(err.to_string()),
            document_id: body.document_id,
        })),
    }
}
