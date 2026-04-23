// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use deadpool_postgres::Client as DbClient;
use electoral_log::messages::newtypes::CertificateAuthEventAction;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::ballot::VoterCertificatePolicy;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::{error, instrument};
use uuid::Uuid;
use windmill::postgres::certificate_authority::delete_certificate_authorities;
use windmill::postgres::election_event::get_election_event_by_id;
use windmill::services::database::get_hasura_pool;
use windmill::services::election_event_board::get_election_event_board;
use windmill::services::electoral_log::ElectoralLog;

/// Request body for deleting a certificate authority.
#[derive(Serialize, Deserialize, Debug)]
pub struct DeleteCertificateAuthorityInput {
    /// The certificate IDs
    ids: Vec<uuid::Uuid>,
    election_event_id: uuid::Uuid,
}

/// Response for certificate authority deletion.
#[derive(Serialize, Deserialize, Debug)]
pub struct DeleteCertificateAuthorityOutput {
    /// The number of deleted certificate authorities
    deleted_count: i32,
}

/// Deletes a certificate authority.
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

    let voter_certificate_policy = election_event
        .get_presentation()
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?
        .unwrap_or_default()
        .voter_certificate_policy
        .unwrap_or_default();

    if voter_certificate_policy != VoterCertificatePolicy::ENABLED {
        return Err((
            Status::Forbidden,
            "Digital certificate authentication is not allowed for this election event".to_string(),
        ));
    }

    let deleted_subjects = delete_certificate_authorities(
        &hasura_transaction,
        &body.ids,
        body.election_event_id,
        tenant_uuid,
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;

    let electoral_log = if !deleted_subjects.is_empty() {
        let board_name =
            get_election_event_board(election_event.bulletin_board_reference)
                .ok_or_else(|| {
                (
                    Status::InternalServerError,
                    "Missing bulletin board".to_string(),
                )
            })?;
        match ElectoralLog::for_admin_user(
            &hasura_transaction,
            &board_name,
            &tenant_id_str,
            &body.election_event_id.to_string(),
            &claims.hasura_claims.user_id,
            claims.preferred_username.clone(),
            None,
            None,
        )
        .await
        {
            Ok(log) => Some(log),
            Err(e) => {
                error!("Error initializing electoral log for CA delete: {e:?}");
                None
            }
        }
    } else {
        None
    };

    let deleted_count = deleted_subjects.len();

    hasura_transaction
        .commit()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;

    if let Some(log) = electoral_log {
        if let Err(e) = log
            .post_certificate_auth_event(
                body.election_event_id.to_string(),
                CertificateAuthEventAction::Delete,
                deleted_subjects,
                Some(claims.hasura_claims.user_id.clone()),
                claims.preferred_username.clone(),
            )
            .await
        {
            error!("Error posting CA delete event to electoral log: {e:?}");
        }
    }

    Ok(Json(DeleteCertificateAuthorityOutput {
        deleted_count: deleted_count as i32,
    }))
}
