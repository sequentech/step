// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::ballot::VoterDigitalCertPolicy;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use uuid::Uuid;
use windmill::postgres::certificate_authority::delete_certificate_authority;
use windmill::postgres::election_event::get_election_event_by_id;
use windmill::services::database::get_hasura_pool;

#[derive(Serialize, Deserialize, Debug)]
pub struct DeleteCertificateAuthorityInput {
    id: uuid::Uuid,
    election_event_id: uuid::Uuid,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DeleteCertificateAuthorityOutput {
    deleted: bool,
}

#[instrument(skip(claims, input))]
#[post("/delete-certificate-authority", format = "json", data = "<input>")]
pub async fn delete_certificate_authority_route(
    claims: JwtClaims,
    input: Json<DeleteCertificateAuthorityInput>,
) -> Result<Json<DeleteCertificateAuthorityOutput>, (Status, String)> {
    let tenant_id_str = claims.hasura_claims.tenant_id.clone();

    authorize(
        &claims,
        true,
        Some(tenant_id_str.clone()),
        vec![Permissions::CA_WRITE],
    )?;

    let body = input.into_inner();

    let tenant_uuid = Uuid::parse_str(&tenant_id_str)
        .map_err(|e| (Status::BadRequest, format!("Invalid tenant_id: {e}")))?;

    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;

    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;

    let election_event = get_election_event_by_id(
        &hasura_transaction,
        &tenant_id_str,
        &body.election_event_id.to_string(),
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;

    let voter_digital_cert_policy = election_event
        .get_presentation()
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?
        .unwrap_or_default()
        .voter_digital_cert_policy
        .unwrap_or_default();

    if voter_digital_cert_policy != VoterDigitalCertPolicy::ENABLED {
        return Err((
            Status::Forbidden,
            "Digital certificate authentication is not allowed for this election event".to_string(),
        ));
    }

    let deleted =
        delete_certificate_authority(&hasura_transaction, body.id, tenant_uuid)
            .await
            .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;

    hasura_transaction
        .commit()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;

    Ok(Json(DeleteCertificateAuthorityOutput { deleted }))
}
