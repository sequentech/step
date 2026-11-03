// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::authorization::{
    authorize, authorize_election_permission_labels,
};
use rocket::{http::Status, serde::json::Json};
use sequent_core::election_lifecycle::ElectoralPhase;
use sequent_core::{services::jwt::JwtClaims, types::permissions::Permissions};
use serde::{Deserialize, Serialize};
use tracing::instrument;
use windmill::services::{database::get_hasura_pool, election_lifecycle};

#[derive(Serialize, Deserialize, Debug)]
pub struct ArchiveElectionEventInput {
    election_event_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ArchiveElectionEvent {
    id: String,
}

fn internal(error: impl std::fmt::Debug) -> (Status, String) {
    tracing::error!(?error, "Archive election event request failed");
    (
        Status::InternalServerError,
        "Archive election event request failed".into(),
    )
}

/// Archives a wrongly imported election event in Preparation. Nothing sets
/// `is_archived` back; the electoral log records the archiving.
#[instrument(skip(claims))]
#[post("/archive-election-event", format = "json", data = "<body>")]
pub async fn archive_election_event_f(
    body: Json<ArchiveElectionEventInput>,
    claims: JwtClaims,
) -> Result<Json<ArchiveElectionEvent>, (Status, String)> {
    let input = body.into_inner();
    let tenant = &claims.hasura_claims.tenant_id;
    let event = &input.election_event_id;
    authorize(
        &claims,
        false,
        Some(tenant.clone()),
        vec![Permissions::ELECTION_EVENT_ARCHIVE],
    )?;
    let mut client = get_hasura_pool().await.get().await.map_err(internal)?;
    let tx = client.transaction().await.map_err(internal)?;
    authorize_election_permission_labels(&tx, &claims, event, None).await?;
    election_lifecycle::require_phase(
        &tx,
        tenant,
        event,
        ElectoralPhase::Preparation,
    )
    .await
    .map_err(|error| (Status::BadRequest, error.to_string()))?;
    let archived = tx
        .execute(
            "UPDATE sequent_backend.election_event SET is_archived=true WHERE tenant_id=$1::text::uuid AND id=$2::text::uuid AND is_archived IS DISTINCT FROM true",
            &[tenant, event],
        )
        .await
        .map_err(internal)?;
    if archived != 1 {
        return Err((
            Status::BadRequest,
            "The election event is already archived".into(),
        ));
    }
    election_lifecycle::notice(
        &tx,
        tenant,
        event,
        "ElectionEventArchived",
        serde_json::json!({"archived_by": claims.hasura_claims.user_id}),
    )
    .await
    .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    Ok(Json(ArchiveElectionEvent { id: event.clone() }))
}
