// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::routes::ballot_checks::ensure_checks_open;
use crate::routes::ballot_privacy::PrivacyPayload;
use crate::services::authorization::authorize_voter_event;
use crate::services::dependencies::HarvestServices;
use rocket::{http::Status, serde::json::Json, State};
use sequent_core::{
    services::jwt::JwtClaims, types::permissions::VoterPermissions,
};
use serde::Deserialize;
use serde_json::json;

type Error = (Status, String);
fn unavailable(_: impl std::fmt::Display) -> Error {
    (Status::InternalServerError, "Receipt unavailable".into())
}
fn denied() -> Error {
    (Status::Forbidden, "Receipt unavailable".into())
}

#[derive(Deserialize)]
pub struct ReceiptDocumentInput {
    election_event_id: String,
    document_id: String,
}

/// Polling is independent of general document enumeration. Only the requesting
/// voter's private identity can name this receipt; URLs are never shared-cacheable.
#[post("/receipt-document", format = "json", data = "<body>")]
pub async fn receipt_document(
    claims: JwtClaims,
    body: Json<ReceiptDocumentInput>,
    services: &State<HarvestServices>,
) -> Result<Json<PrivacyPayload>, Error> {
    authorize_voter_event(
        &claims,
        vec![VoterPermissions::CAST_VOTE],
        &body.election_event_id,
    )?;
    uuid::Uuid::parse_str(&body.document_id).map_err(|_| denied())?;
    let tenant = &claims.hasura_claims.tenant_id;
    let mut client = services
        .databases
        .hasura()
        .await
        .get()
        .await
        .map_err(unavailable)?;
    let tx = client.transaction().await.map_err(unavailable)?;
    let event = windmill::postgres::election_event::get_election_event_by_id(
        &tx,
        tenant,
        &body.election_event_id,
    )
    .await
    .map_err(|_| denied())?;
    let identity = windmill::services::vote_privacy::ballot_identity(
        &event,
        &claims.hasura_claims.user_id,
    )
    .await
    .map_err(|_| denied())?;
    let store =
        windmill::services::electoral_log_board::get_event_store(&event.id)
            .await
            .map_err(unavailable)?;
    if !store
        .owns_receipt_document(&event.id, &body.document_id, &identity)
        .await
        .map_err(unavailable)?
    {
        return Err(denied());
    }
    ensure_checks_open(tenant, &event.id).await?;
    let document = windmill::postgres::document::get_document(
        &tx,
        tenant,
        Some(event.id.clone()),
        &body.document_id,
    )
    .await
    .map_err(unavailable)?;
    let value = match document {
        None => serde_json::Value::Null,
        Some(document) => {
            let url = services
                .documents
                .url(&tx, tenant, Some(&event.id), &body.document_id)
                .await
                .map_err(unavailable)?;
            match url {
                Some(url) => {
                    json!({"id":document.id,"name":document.name,"url":url})
                }
                None => serde_json::Value::Null,
            }
        }
    };
    Ok(Json(PrivacyPayload {
        payload: json!({"document":value}),
    }))
}
