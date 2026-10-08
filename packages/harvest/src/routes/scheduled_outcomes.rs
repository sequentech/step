// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! What scheduled openings and closings of voting will do (VOTE-LIFECYCLE
//! §5c): the outcome of each, what a pending change would do to them, and
//! saving the lifecycle policies.

use crate::services::authorization::authorize;
use crate::types::error_response::{ErrorCode, ErrorResponse, JsonError};
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::ballot::LifecyclePolicies;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use uuid::Uuid;
use windmill::services::database::get_hasura_pool;
use windmill::services::scheduled_outcome::{
    lifecycle_snapshots, preview_change, save_lifecycle_policies,
    scheduled_outcomes, ChangeApplies, LockedDown, OutcomeChange,
    PendingChange, ScheduledOutcome, SnapshotView,
};
use windmill::services::signing::actions::voting::{
    retained_signed_closes, RetainedSignedClose,
};
use windmill::services::signing::log::Actor;
use windmill::tasks::signing_log_outbox::kick_signing_log_outbox;

fn internal(error: impl std::fmt::Debug) -> JsonError {
    ErrorResponse::new(
        Status::InternalServerError,
        &format!("{error:?}"),
        ErrorCode::InternalServerError,
    )
}

fn allow(claims: &JwtClaims, permission: Permissions) -> Result<(), JsonError> {
    authorize(
        claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![permission],
    )
    .map_err(|error| {
        ErrorResponse::new(
            Status::Unauthorized,
            &format!("{error:?}"),
            ErrorCode::Unauthorized,
        )
    })
}

/// The caller's tenant and the event, as UUIDs.
fn ids(
    claims: &JwtClaims,
    election_event_id: &str,
) -> Result<(Uuid, Uuid), JsonError> {
    let parse = |text: &str| {
        Uuid::parse_str(text).map_err(|_| {
            ErrorResponse::new(
                Status::BadRequest,
                &format!("{text} is not a UUID"),
                ErrorCode::UuidParseFailed,
            )
        })
    };
    Ok((
        parse(&claims.hasura_claims.tenant_id)?,
        parse(election_event_id)?,
    ))
}

async fn client() -> Result<DbClient, JsonError> {
    get_hasura_pool().await.get().await.map_err(internal)
}

#[derive(Deserialize, Debug)]
pub struct ScheduledOutcomesInput {
    election_event_id: String,
}

#[derive(Serialize, Debug)]
pub struct ScheduledOutcomesOutput {
    /// One per scheduled opening or closing and Post (an event-wide row
    /// has one per Post).
    outcomes: Vec<ScheduledOutcome>,
    retained_closes: Vec<RetainedSignedClose>,
}

#[instrument(skip(claims))]
#[post("/get-scheduled-outcomes", format = "json", data = "<body>")]
pub async fn get_scheduled_outcomes(
    body: Json<ScheduledOutcomesInput>,
    claims: JwtClaims,
) -> Result<Json<ScheduledOutcomesOutput>, JsonError> {
    allow(&claims, Permissions::ELECTION_EVENT_READ)?;
    let (tenant_id, election_event_id) = ids(&claims, &body.election_event_id)?;
    let mut client = client().await?;
    let transaction = client.transaction().await.map_err(internal)?;
    let outcomes =
        scheduled_outcomes(&transaction, tenant_id, election_event_id)
            .await
            .map_err(internal)?;
    let retained_closes =
        retained_signed_closes(&transaction, tenant_id, election_event_id)
            .await
            .map_err(internal)?;
    transaction.rollback().await.map_err(internal)?;
    Ok(Json(ScheduledOutcomesOutput {
        outcomes,
        retained_closes,
    }))
}

#[derive(Deserialize, Debug)]
pub struct PreviewScheduledOutcomeChangeInput {
    election_event_id: String,
    /// `{"scheduled_event": {id?, event_processor, cron_config,
    /// event_payload}}`, `{"policies": {...}}` or `{"rule": {action,
    /// required, signatures?}}`.
    change: PendingChange,
}

#[derive(Serialize, Debug)]
pub struct PreviewScheduledOutcomeChangeOutput {
    /// For a rule or policy change: how it applies (`tightens`, `loosens`,
    /// `tightens-and-loosens`), and its i18n key.
    applies: Option<ChangeApplies>,
    applies_message_key: Option<String>,
    changes: Vec<OutcomeChange>,
}

#[instrument(skip(claims))]
#[post("/preview-scheduled-outcome-change", format = "json", data = "<body>")]
pub async fn preview_scheduled_outcome_change(
    body: Json<PreviewScheduledOutcomeChangeInput>,
    claims: JwtClaims,
) -> Result<Json<PreviewScheduledOutcomeChangeOutput>, JsonError> {
    allow(&claims, Permissions::ELECTION_EVENT_READ)?;
    let body = body.into_inner();
    let (tenant_id, election_event_id) = ids(&claims, &body.election_event_id)?;
    let mut client = client().await?;
    let transaction = client.transaction().await.map_err(internal)?;
    let preview = preview_change(
        &transaction,
        tenant_id,
        election_event_id,
        &body.change,
    )
    .await
    .map_err(internal)?;
    transaction.rollback().await.map_err(internal)?;
    Ok(Json(PreviewScheduledOutcomeChangeOutput {
        applies: preview.applies,
        applies_message_key: preview
            .applies
            .map(|applies| applies.message_key().to_owned()),
        changes: preview.changes,
    }))
}

#[derive(Deserialize, Debug)]
pub struct SaveLifecyclePoliciesInput {
    election_event_id: String,
    policies: LifecyclePolicies,
}

#[derive(Serialize, Debug)]
pub struct SaveLifecyclePoliciesOutput {
    policies: LifecyclePolicies,
    applies: Option<ChangeApplies>,
    applies_message_key: Option<String>,
    changes: Vec<OutcomeChange>,
}

#[instrument(skip(claims))]
#[post("/save-lifecycle-policies", format = "json", data = "<body>")]
pub async fn save_lifecycle_policies_route(
    body: Json<SaveLifecyclePoliciesInput>,
    claims: JwtClaims,
) -> Result<Json<SaveLifecyclePoliciesOutput>, JsonError> {
    allow(&claims, Permissions::ELECTION_EVENT_WRITE)?;
    let body = body.into_inner();
    let (tenant_id, election_event_id) = ids(&claims, &body.election_event_id)?;
    let actor = Actor {
        user_id: claims.hasura_claims.user_id.clone(),
        username: claims
            .preferred_username
            .clone()
            .unwrap_or_else(|| claims.hasura_claims.user_id.clone()),
    };
    let mut client = client().await?;
    let transaction = client.transaction().await.map_err(internal)?;
    let saved = save_lifecycle_policies(
        &transaction,
        tenant_id,
        election_event_id,
        &body.policies,
        &actor,
    )
    .await
    .map_err(|error| match error.downcast_ref::<LockedDown>() {
        Some(locked) => ErrorResponse::new(
            Status::Conflict,
            &locked.to_string(),
            ErrorCode::LockedDown,
        ),
        None => internal(error),
    })?;
    transaction.commit().await.map_err(internal)?;
    kick_signing_log_outbox();
    Ok(Json(SaveLifecyclePoliciesOutput {
        policies: saved.policies,
        applies: saved.applies,
        applies_message_key: saved
            .applies
            .map(|applies| applies.message_key().to_owned()),
        changes: saved.changes,
    }))
}

#[derive(Serialize, Debug)]
pub struct LifecycleSnapshotsOutput {
    /// Per target (a Post, or the event: `election_id` null): the newest
    /// kept snapshot, and the newest signed one when that is another.
    snapshots: Vec<SnapshotView>,
}

#[instrument(skip(claims))]
#[post("/get-lifecycle-snapshots", format = "json", data = "<body>")]
pub async fn get_lifecycle_snapshots(
    body: Json<ScheduledOutcomesInput>,
    claims: JwtClaims,
) -> Result<Json<LifecycleSnapshotsOutput>, JsonError> {
    allow(&claims, Permissions::ELECTION_EVENT_READ)?;
    let (tenant_id, election_event_id) = ids(&claims, &body.election_event_id)?;
    let mut client = client().await?;
    let transaction = client.transaction().await.map_err(internal)?;
    let snapshots =
        lifecycle_snapshots(&transaction, tenant_id, election_event_id)
            .await
            .map_err(internal)?;
    transaction.rollback().await.map_err(internal)?;
    Ok(Json(LifecycleSnapshotsOutput { snapshots }))
}
