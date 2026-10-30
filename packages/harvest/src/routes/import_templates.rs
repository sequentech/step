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

fn authorize_import_templates(
    claims: &jwt::JwtClaims,
    input: &ImportTemplatesInput,
) -> Result<(), (Status, String)> {
    authorize(
        claims,
        true,
        Some(input.tenant_id.clone()),
        vec![Permissions::TEMPLATE_WRITE],
    )
}

#[instrument(skip(claims))]
#[post("/import-templates", format = "json", data = "<input>")]
pub async fn import_templates_route(
    claims: jwt::JwtClaims,
    input: Json<ImportTemplatesInput>,
) -> Result<Json<ImportTemplatesOutput>, (Status, String)> {
    let body = input.into_inner();
    authorize_import_templates(&claims, &body)?;

    match provide_hasura_transaction(|hasura_transaction| {
        let tenant_id = body.tenant_id.clone();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::authorization::test_claims::{
        admin_claims, CALLER_TENANT_ID, OTHER_TENANT_ID,
    };

    fn input(tenant_id: &str) -> ImportTemplatesInput {
        ImportTemplatesInput {
            tenant_id: tenant_id.to_string(),
            document_id: "document".to_string(),
            sha256: None,
        }
    }

    #[test]
    fn import_templates_rejects_tenant_other_than_callers() {
        let claims =
            admin_claims(CALLER_TENANT_ID, &["communication-template-write"]);
        let result =
            authorize_import_templates(&claims, &input(OTHER_TENANT_ID));
        assert_eq!(result.unwrap_err().0, Status::Unauthorized);
    }

    #[test]
    fn import_templates_accepts_callers_tenant() {
        let claims =
            admin_claims(CALLER_TENANT_ID, &["communication-template-write"]);
        assert!(
            authorize_import_templates(&claims, &input(CALLER_TENANT_ID))
                .is_ok()
        );
    }
}
