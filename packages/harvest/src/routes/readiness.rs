// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! A permission-scoped readiness snapshot; failed checks never mutate events.

use crate::services::audit;
use crate::services::authorization::{
    authorize, authorize_election_permission_labels,
};
use crate::services::dependencies::HarvestServices;
use crate::types::error_response::{ErrorCode, ErrorResponse, JsonError};
use rocket::{http::Status, serde::json::Json, State};
use sequent_core::services::jwt::JwtClaims;
use sequent_core::services::uuid_validation::parse_uuid_v4;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use windmill::domain::readiness::{ComparisonMode, ReadinessReport};
use windmill::domain::readiness_rehearsal::RehearsalReport;
use windmill::services::readiness::ReadinessStage;

/// In rehearsal modes the first position holds the first role the mode
/// names (ERT, or the official event) and the second the other one.
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReadinessInput {
    official_event_id: String,
    tested_event_id: String,
    /// Absent or null is the strict production check.
    #[serde(default)]
    comparison: Option<ComparisonMode>,
}

/// The production report keeps its signed shape; a rehearsal report always
/// names its read-only comparison.
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum ReadinessResponse {
    Production(ReadinessReport),
    Rehearsal(RehearsalReport),
}

fn invalid(message: &str) -> JsonError {
    ErrorResponse::new(
        Status::BadRequest,
        message,
        ErrorCode::InvalidReadinessCheck,
    )
}

fn internal(error: impl std::fmt::Debug) -> JsonError {
    tracing::error!("Readiness check could not be completed: {error:?}");
    ErrorResponse::new(
        Status::InternalServerError,
        "Readiness check could not be completed",
        ErrorCode::InternalServerError,
    )
}

/// Names the step that stopped the check, never the underlying error.
fn incomplete(error: anyhow::Error) -> JsonError {
    match error.downcast_ref::<ReadinessStage>() {
        Some(stage) => {
            tracing::error!(
                "Readiness check could not be completed: {error:?}"
            );
            ErrorResponse::new(
                Status::InternalServerError,
                &format!("Readiness check could not be completed: {stage}"),
                ErrorCode::InternalServerError,
            )
        }
        None => internal(error),
    }
}

fn validate(input: &ReadinessInput) -> Result<(), JsonError> {
    for id in [&input.official_event_id, &input.tested_event_id] {
        parse_uuid_v4(id)
            .map_err(|_| invalid("Each event ID must be a UUID v4"))?;
    }
    if parse_uuid_v4(&input.official_event_id)
        .map_err(|_| invalid("Invalid official event ID"))?
        == parse_uuid_v4(&input.tested_event_id)
            .map_err(|_| invalid("Invalid tested event ID"))?
    {
        return Err(invalid("The official and tested events must differ"));
    }
    Ok(())
}

/// Check both events' tenant and permission labels before any lock or read.
async fn authorize_events(
    tx: &deadpool_postgres::Transaction<'_>,
    claims: &JwtClaims,
    input: &ReadinessInput,
) -> Result<(), JsonError> {
    let tenant = &claims.hasura_claims.tenant_id;
    for event in [&input.official_event_id, &input.tested_event_id] {
        let row = windmill::postgres::election_event::get_election_event_by_id_if_exist(tx, tenant, event)
            .await.map_err(internal)?;
        if row.is_none() {
            return Err(ErrorResponse::new(
                Status::NotFound,
                "Election event not found",
                ErrorCode::ElectionEventNotFound,
            ));
        }
        authorize_election_permission_labels(tx, claims, event, None)
            .await
            .map_err(|(status, message)| {
                ErrorResponse::new(status, &message, ErrorCode::Unauthorized)
            })?;
    }
    Ok(())
}

#[post("/readiness-check", format = "json", data = "<input>")]
pub async fn readiness_check(
    claims: JwtClaims,
    input: Json<ReadinessInput>,
    services: &State<HarvestServices>,
) -> Result<Json<ReadinessResponse>, JsonError> {
    let input = input.into_inner();
    authorize(
        &claims,
        false,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::READINESS_CHECK],
    )
    .map_err(|(status, message)| {
        ErrorResponse::new(status, &message, ErrorCode::Unauthorized)
    })?;
    validate(&input)?;
    let tenant = &claims.hasura_claims.tenant_id;
    let mut client = services
        .databases
        .hasura()
        .await
        .get()
        .await
        .map_err(internal)?;
    if let Some(comparison) = input.comparison.unwrap_or_default().rehearsal() {
        // Read committed observes work that finished while the advisory lock
        // was waiting. A snapshot fixed during authorization would predate
        // that work while the separate evidence stores return current data.
        // The transaction stays read-only and never has a recorder.
        let tx = client
            .build_transaction()
            .isolation_level(tokio_postgres::IsolationLevel::ReadCommitted)
            .read_only(true)
            .start()
            .await
            .map_err(internal)?;
        authorize_events(&tx, &claims, &input).await?;
        windmill::services::readiness::lock_events_read_only(
            &tx,
            tenant,
            &input.official_event_id,
            &input.tested_event_id,
        )
        .await
        .map_err(incomplete)?;
        let report = windmill::services::readiness::compare_rehearsal(
            &tx,
            tenant,
            comparison,
            &input.official_event_id,
            &input.tested_event_id,
        )
        .await
        .map_err(incomplete)?;
        tx.rollback().await.map_err(internal)?;
        return Ok(Json(ReadinessResponse::Rehearsal(report)));
    }
    let tx = client.transaction().await.map_err(internal)?;
    authorize_events(&tx, &claims, &input).await?;
    windmill::services::readiness::lock_events(
        &tx,
        tenant,
        &input.official_event_id,
        &input.tested_event_id,
    )
    .await
    .map_err(incomplete)?;
    let actor = audit::actor(&claims);
    let report = windmill::services::readiness::verify(
        &tx,
        tenant,
        &input.official_event_id,
        &input.tested_event_id,
        &actor,
    )
    .await
    .map_err(incomplete)?;
    // This transaction only holds observation locks. The signed event entry
    // has its own durable append and exists before a passing response returns.
    tx.commit().await.map_err(internal)?;
    Ok(Json(ReadinessResponse::Production(report)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn message(error: JsonError) -> (Status, String) {
        let rocket::response::status::Custom(status, Json(body)) = error;
        (status, body.message)
    }

    #[test]
    fn an_incomplete_check_says_where_it_stopped_without_internal_detail() {
        for (stage, expected) in [
            (
                ReadinessStage::Locks,
                "Readiness check could not be completed: the events could not be locked for a consistent snapshot; a voter import, signing or publication may be running on one of them",
            ),
            (
                ReadinessStage::OfficialEvidence,
                "Readiness check could not be completed: the official event's evidence could not be read",
            ),
            (
                ReadinessStage::TestedEvidence,
                "Readiness check could not be completed: the tested event's evidence could not be read",
            ),
            (
                ReadinessStage::SignedAppend,
                "Readiness check could not be completed: every check passed but the signed entry could not be confirmed on the official event's log; check its Logs before running it again",
            ),
        ] {
            let error = anyhow::anyhow!("password=secret at db:5432")
                .context(stage);
            let (status, body) = message(incomplete(error));
            assert_eq!(status, Status::InternalServerError);
            assert_eq!(body, expected);
        }
        let (status, body) =
            message(incomplete(anyhow::anyhow!("password=secret at db:5432")));
        assert_eq!(status, Status::InternalServerError);
        assert_eq!(body, "Readiness check could not be completed");
    }

    #[test]
    fn the_two_events_must_be_distinct_real_ids() {
        let official = uuid::Uuid::new_v4().to_string();
        let tested = uuid::Uuid::new_v4().to_string();
        assert!(validate(&ReadinessInput {
            official_event_id: official.clone(),
            tested_event_id: tested,
            comparison: None,
        })
        .is_ok());
        assert!(validate(&ReadinessInput {
            official_event_id: official.clone(),
            tested_event_id: official,
            comparison: Some(ComparisonMode::RehearsalErtStl),
        })
        .is_err());
        assert!(validate(&ReadinessInput {
            official_event_id: "not-an-id".into(),
            tested_event_id: uuid::Uuid::new_v4().to_string(),
            comparison: None,
        })
        .is_err());
    }

    #[test]
    fn an_absent_or_null_mode_is_the_strict_production_check() {
        let (official, tested) = (uuid::Uuid::new_v4(), uuid::Uuid::new_v4());
        for body in [
            serde_json::json!({"official_event_id": official, "tested_event_id": tested}),
            serde_json::json!({"official_event_id": official, "tested_event_id": tested, "comparison": null}),
            serde_json::json!({"official_event_id": official, "tested_event_id": tested, "comparison": "production"}),
        ] {
            let input: ReadinessInput = serde_json::from_value(body).unwrap();
            assert_eq!(input.comparison.unwrap_or_default().rehearsal(), None);
        }
        for (wire, rehearsal) in [
            ("rehearsal_ert_stl", true),
            ("rehearsal_official_ert", true),
            ("rehearsal_official_stl", true),
        ] {
            let input: ReadinessInput = serde_json::from_value(serde_json::json!({
                "official_event_id": official, "tested_event_id": tested, "comparison": wire
            }))
            .unwrap();
            assert_eq!(
                input.comparison.unwrap_or_default().rehearsal().is_some(),
                rehearsal
            );
        }
        // An unknown mode is refused, never read as production.
        assert!(serde_json::from_value::<ReadinessInput>(serde_json::json!({
            "official_event_id": official, "tested_event_id": tested, "comparison": "rehearsal"
        }))
        .is_err());
    }
}
