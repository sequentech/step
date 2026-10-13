// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::{
    authorize, authorize_tally_session_permission_labels,
};
use crate::services::dependencies::HarvestServices;
use crate::types::error_response::{ErrorCode, ErrorResponse, JsonError};
use protocol_board::{TrusteeActionResult, TrusteeEventUpdates};
use rocket::{http::Status, serde::json::Json, State};
use sequent_core::{services::jwt::JwtClaims, types::permissions::Permissions};
use serde::Serialize;
use windmill::services::trustee_updates::{
    self, ServerCustodyInput, ServerCustodyReply,
};

#[derive(Serialize)]
pub struct ActionAcknowledgement {
    accepted: bool,
}

fn authorization(claims: &JwtClaims) -> Result<(), JsonError> {
    authorize(
        claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::TRUSTEE_CEREMONY],
    )
    .map_err(refusal)
}

#[get("/trustee/event-updates")]
pub async fn trustee_event_updates(
    claims: JwtClaims,
    services: &State<HarvestServices>,
) -> Result<Json<TrusteeEventUpdates>, JsonError> {
    authorization(&claims)?;
    let mut db = services
        .databases
        .hasura()
        .await
        .get()
        .await
        .map_err(internal)?;
    let tx = db.transaction().await.map_err(internal)?;
    let updates = trustee_updates::event_updates(&tx, &claims)
        .await
        .map_err(internal)?;
    Ok(Json(updates))
}

/// An authenticated file upload requests work, never a ceremony completion.
#[post("/trustee/request-server-custody", format = "json", data = "<body>")]
pub async fn request_server_custody(
    body: Json<ServerCustodyInput>,
    claims: JwtClaims,
    services: &State<HarvestServices>,
) -> Result<Json<ServerCustodyReply>, JsonError> {
    authorization(&claims)?;
    let mut db = services
        .databases
        .hasura()
        .await
        .get()
        .await
        .map_err(internal)?;
    let tx = db.transaction().await.map_err(internal)?;
    if let Some(id) = &body.tally_session_id {
        authorize_tally_session_permission_labels(
            &tx,
            &claims,
            &body.election_event_id,
            id,
        )
        .await
        .map_err(refusal)?;
    }
    let reply = trustee_updates::request_server_custody(&tx, &claims, &body)
        .await
        .map_err(refused)?;
    tx.commit().await.map_err(internal)?;
    Ok(Json(reply))
}

/// Store a verified server proof for the operator's existing approval flow.
#[post("/trustee/action-result", format = "json", data = "<body>")]
pub async fn submit_trustee_action_result(
    body: Json<TrusteeActionResult>,
    claims: JwtClaims,
    services: &State<HarvestServices>,
) -> Result<Json<ActionAcknowledgement>, JsonError> {
    authorization(&claims)?;
    let mut db = services
        .databases
        .hasura()
        .await
        .get()
        .await
        .map_err(internal)?;
    let tx = db.transaction().await.map_err(internal)?;
    trustee_updates::submit_action_result(&tx, &claims, &body)
        .await
        .map_err(refused)?;
    tx.commit().await.map_err(internal)?;
    Ok(Json(ActionAcknowledgement { accepted: true }))
}

fn refusal((status, reason): (Status, String)) -> JsonError {
    let code = if status == Status::Unauthorized || status == Status::Forbidden
    {
        ErrorCode::Unauthorized
    } else {
        ErrorCode::InvalidTrusteeAction
    };
    ErrorResponse::new(status, &reason, code)
}

fn internal(error: impl std::fmt::Display) -> JsonError {
    tracing::error!("Trustee updates failed: {error}");
    ErrorResponse::new(
        Status::InternalServerError,
        "The trustee update service is unavailable",
        ErrorCode::InternalServerError,
    )
}
fn refused(error: impl std::fmt::Display) -> JsonError {
    ErrorResponse::new(
        Status::BadRequest,
        &error.to_string(),
        ErrorCode::InvalidTrusteeAction,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[get("/refusal")]
    fn refusal_route() -> JsonError {
        refused(
            "The ceremony file does not match this trustee and key ceremony",
        )
    }

    #[rocket::async_test]
    async fn refusals_use_the_json_contract_hasura_forwards_to_the_portal() {
        let client = rocket::local::asynchronous::Client::tracked(
            rocket::build().mount("/", routes![refusal_route]),
        )
        .await
        .unwrap();
        let response = client.get("/refusal").dispatch().await;
        assert_eq!(response.status(), Status::BadRequest);
        let body = response
            .into_json::<serde_json::Value>()
            .await
            .expect("Hasura JSON webhook error");
        assert_eq!(
            body["message"],
            "The ceremony file does not match this trustee and key ceremony"
        );
    }
}
