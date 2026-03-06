// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use anyhow::{anyhow, Result};
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::ballot::{
    AllowTallyStatus, ElectionStatus, InitReport, VotingStatus,
};
use sequent_core::serialization::deserialize_with_path;
use sequent_core::services::jwt::decode_permission_labels;
use sequent_core::types::ceremonies::TallyExecutionStatus;
use sequent_core::types::ceremonies::TallyType;
use sequent_core::types::permissions::Permissions;
use sequent_core::{
    services::jwt::JwtClaims, types::hasura::core::TallySessionConfiguration,
};
use serde::{Deserialize, Serialize};
use tracing::{event, instrument, Level};
use windmill::postgres::election::get_elections_by_ids;
use windmill::postgres::tally_session::{
    get_tally_session_by_id, update_tally_session_status,
};
use windmill::postgres::tally_session_resolution::{
    get_resolution_by_tally_session, submit_resolution, update_resolution,
    ResolutionStatus, ResolutionType,
};
use windmill::services::celery_app::get_celery_app;
use windmill::services::ceremonies::tally_ceremony::{
    self, extract_tied_candidate_ids, is_resubmission,
    validate_resolution_allowed,
};
use windmill::services::database::get_hasura_pool;
use windmill::services::providers::transactions_provider::provide_hasura_transaction;
use windmill::tasks::electoral_log::{
    enqueue_electoral_log_event, LogEventBody, LogEventInput, LogMessageType,
};

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateTallyCeremonyInput {
    election_event_id: String,
    election_ids: Vec<String>,
    configuration: Option<TallySessionConfiguration>,
    tally_type: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateTallyCeremonyOutput {
    tally_session_id: String,
}

// The main function to start a key ceremony
#[instrument(skip(claims))]
#[post("/create-tally-ceremony", format = "json", data = "<body>")]
pub async fn create_tally_ceremony(
    body: Json<CreateTallyCeremonyInput>,
    claims: JwtClaims,
) -> Result<Json<CreateTallyCeremonyOutput>, (Status, String)> {
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::ADMIN_CEREMONY],
    )?;
    let input = body.into_inner();
    let tenant_id: String = claims.hasura_claims.tenant_id.clone();
    let user_id = claims.clone().hasura_claims.user_id;
    let username = claims
        .clone()
        .preferred_username
        .unwrap_or(claims.name.clone().unwrap_or_else(|| user_id.clone()));
    let permission_labels = decode_permission_labels(&claims);

    let mut hasura_db_client: DbClient =
        get_hasura_pool().await.get().await.map_err(|err| {
            (
                Status::InternalServerError,
                format!("Error getting hasura db pool: {err}"),
            )
        })?;

    let hasura_transaction =
        hasura_db_client.transaction().await.map_err(|err| {
            (
                Status::InternalServerError,
                format!("Error starting hasura transaction: {err}"),
            )
        })?;

    let tally_session_id = tally_ceremony::create_tally_ceremony(
        &hasura_transaction,
        tenant_id,
        &user_id,
        input.election_event_id.clone(),
        input.election_ids,
        input.configuration,
        input.tally_type.clone(),
        &permission_labels,
        username,
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    let _commit = hasura_transaction.commit().await.map_err(|err| {
        (Status::InternalServerError, format!("Commit failed: {err}"))
    })?;
    event!(
        Level::INFO,
        "Created Tally Ceremony, type={}, electionEventId={}, tallySessionId={}",
        input.tally_type,
        input.election_event_id,
        tally_session_id,
    );

    Ok(Json(CreateTallyCeremonyOutput { tally_session_id }))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateTallyCeremonyInput {
    election_event_id: String,
    tally_session_id: String,
    status: TallyExecutionStatus,
}

#[instrument(skip(claims))]
#[post("/update-tally-ceremony", format = "json", data = "<body>")]
pub async fn update_tally_ceremony(
    body: Json<UpdateTallyCeremonyInput>,
    claims: JwtClaims,
) -> Result<Json<CreateTallyCeremonyOutput>, (Status, String)> {
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::ADMIN_CEREMONY],
    )?;
    let input = body.into_inner();
    let tenant_id = claims.hasura_claims.tenant_id.clone();

    let user_id = claims.clone().hasura_claims.user_id;
    let username = claims
        .clone()
        .preferred_username
        .unwrap_or(claims.name.clone().unwrap_or_else(|| user_id.clone()));

    let mut hasura_db_client: DbClient =
        get_hasura_pool().await.get().await.map_err(|err| {
            (
                Status::InternalServerError,
                format!("Error getting hasura db pool: {err}"),
            )
        })?;

    let hasura_transaction =
        hasura_db_client.transaction().await.map_err(|err| {
            (
                Status::InternalServerError,
                format!("Error starting hasura transaction: {err}"),
            )
        })?;

    let tally_session = get_tally_session_by_id(
        &hasura_transaction,
        &tenant_id,
        &input.election_event_id,
        &input.tally_session_id,
    )
    .await
    .map_err(|_| {
        (
            Status::InternalServerError,
            format!(
                "Could not find tally session by id {}",
                input.election_event_id
            ),
        )
    })?;
    let tally_type = tally_session
        .clone()
        .tally_type
        .map(|val: String| {
            TallyType::try_from(val.as_str()).unwrap_or_default()
        })
        .unwrap_or_default();

    let is_tally_allowed = get_elections_by_ids(
        &hasura_transaction,
        &tenant_id,
        &input.election_event_id,
        &tally_session.election_ids.clone().unwrap_or(vec![]),
    )
    .await
    .map_err(|_| {
        (
            Status::InternalServerError,
            format!(
                "Could not find elections for election event {}",
                input.election_event_id
            ),
        )
    })?
    .iter()
    .all(|election| {
        if let Some(election_status) = &election.status {
            deserialize_with_path::deserialize_value::<ElectionStatus>(
                election_status.clone(),
            )
            .map(|election_status| match tally_type {
                TallyType::ELECTORAL_RESULTS => {
                    election_status.allow_tally == AllowTallyStatus::ALLOWED
                        || (election_status.allow_tally
                            == AllowTallyStatus::REQUIRES_VOTING_PERIOD_END
                            && (election_status.voting_status.is_closed()
                                && election_status
                                    .kiosk_voting_status
                                    .is_closed_or_never_started()
                                && election_status
                                    .early_voting_status
                                    .is_closed_or_never_started()))
                }
                TallyType::INITIALIZATION_REPORT => {
                    election_status.init_report == InitReport::ALLOWED
                }
            })
            .unwrap_or(true)
        } else {
            true
        }
    });

    if !is_tally_allowed {
        return Err((
            Status::InternalServerError,
            format!(
                "Tally is not allowed for election event {}.",
                input.election_event_id
            ),
        ));
    }

    tally_ceremony::update_tally_ceremony(
        &hasura_transaction,
        tenant_id,
        input.election_event_id.clone(),
        tally_session.clone(),
        input.status.clone(),
        user_id.clone(),
        username.clone(),
    )
    .await
    .map_err(|e| {
        (
            Status::InternalServerError,
            format!("Error with update_tally_ceremony: {:?}", e),
        )
    })?;

    hasura_transaction.commit().await.map_err(|err| {
        (Status::InternalServerError, format!("Commit failed: {err}"))
    })?;

    Ok(Json(CreateTallyCeremonyOutput {
        tally_session_id: input.tally_session_id.clone(),
    }))
}

////////////////////////////////////////////////////////////////////////////////
/// Endpoint: /restore-private-key
////////////////////////////////////////////////////////////////////////////////

#[derive(Serialize, Deserialize, Debug)]
pub struct SetPrivateKeyInput {
    election_event_id: String,
    private_key_base64: String,
    tally_session_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SetPrivateKeyOutput {
    is_valid: bool,
}

// The main function to restore the private key
#[instrument(skip(claims))]
#[post("/restore-private-key", format = "json", data = "<body>")]
pub async fn restore_private_key(
    body: Json<SetPrivateKeyInput>,
    claims: JwtClaims,
) -> Result<Json<SetPrivateKeyOutput>, (Status, String)> {
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::TRUSTEE_CEREMONY],
    )?;
    let input = body.into_inner();
    let tenant_id = claims.hasura_claims.tenant_id.clone();

    let mut hasura_db_client: DbClient =
        get_hasura_pool().await.get().await.map_err(|err| {
            (
                Status::InternalServerError,
                format!("Error getting hasura db pool: {err}"),
            )
        })?;

    let hasura_transaction =
        hasura_db_client.transaction().await.map_err(|err| {
            (
                Status::InternalServerError,
                format!("Error starting hasura transaction: {err}"),
            )
        })?;

    let is_valid = tally_ceremony::set_private_key(
        &hasura_transaction,
        &claims,
        &tenant_id,
        &input.election_event_id,
        &input.tally_session_id,
        &input.private_key_base64,
    )
    .await
    .map_err(|e| (Status::BadRequest, format!("{:?}", e)))?;

    event!(
        Level::INFO,
        "Restoring given private key, election_event_id={}, tally_session_id={}, is_valid={}",
        input.election_event_id,
        input.tally_session_id,
        is_valid,
    );

    hasura_transaction.commit().await.map_err(|err| {
        (Status::InternalServerError, format!("Commit failed: {err}"))
    })?;
    Ok(Json(SetPrivateKeyOutput { is_valid }))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TallyResolution {
    contest_id: String,
    selected_candidate_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SubmitTallyResolutionInput {
    election_event_id: String,
    tally_session_id: String,
    resolutions: Vec<TallyResolution>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SubmitTallyResolutionOutput {
    success: bool,
    tally_session_id: String,
    resolved_count: usize,
}

/// Submit multiple tally resolutions for a paused tally (batch operation)
#[instrument(skip(claims))]
#[post("/submit-tally-resolution", format = "json", data = "<body>")]
pub async fn submit_tally_resolution(
    body: Json<SubmitTallyResolutionInput>,
    claims: JwtClaims,
) -> Result<Json<SubmitTallyResolutionOutput>, (Status, String)> {
    // 1. Authorize
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::TALLY_RESOLUTION_SUBMIT],
    )?;

    let input = body.into_inner();
    let tenant_id = claims.hasura_claims.tenant_id.clone();
    let user_id = claims.hasura_claims.user_id.clone();

    // 2. Validate input
    if input.resolutions.is_empty() {
        return Err((
            Status::BadRequest,
            "At least one resolution required".to_string(),
        ));
    }

    // 3. Get DB connection
    let mut hasura_db_client: DbClient =
        get_hasura_pool().await.get().await.map_err(|err| {
            (
                Status::InternalServerError,
                format!("Error getting hasura db pool: {err}"),
            )
        })?;

    let hasura_transaction =
        hasura_db_client.transaction().await.map_err(|err| {
            (
                Status::InternalServerError,
                format!("Error starting hasura transaction: {err}"),
            )
        })?;

    // 4. Get tally session and validate status
    let tally_session = get_tally_session_by_id(
        &hasura_transaction,
        &tenant_id,
        &input.election_event_id,
        &input.tally_session_id,
    )
    .await
    .map_err(|_| {
        (
            Status::NotFound,
            format!("Tally session not found: {}", input.tally_session_id),
        )
    })?;

    let execution_status = tally_session
        .execution_status
        .and_then(|s| s.parse::<TallyExecutionStatus>().ok())
        .ok_or((
            Status::BadRequest,
            "Invalid or missing execution status".to_string(),
        ))?;

    // 5. Get all resolutions for this tally session (needed before status validation)
    let all_resolutions = get_resolution_by_tally_session(
        &hasura_transaction,
        &tenant_id,
        &input.election_event_id,
        &input.tally_session_id,
    )
    .await
    .map_err(|e| {
        (
            Status::InternalServerError,
            format!("Error fetching resolutions: {}", e),
        )
    })?;

    // Allow re-submission of already-resolved resolutions even when the tally is not
    // in AWAITING_INPUT (e.g., SUCCESS). Only reject if the tally is not awaiting
    // input AND at least one submitted resolution targets a pending (not yet resolved) record.
    let input_contest_ids: Vec<&str> = input
        .resolutions
        .iter()
        .map(|r| r.contest_id.as_str())
        .collect();
    validate_resolution_allowed(
        &execution_status,
        &input_contest_ids,
        &all_resolutions,
    )?;

    // 6. Validate and submit each resolution
    let mut resolved_count = 0;
    let mut has_previous_resolutions = false;

    for tie_resolution in &input.resolutions {
        // Find the latest resolution for this contest (by created_at)
        let latest_resolution = all_resolutions
            .iter()
            .filter(|r| {
                r.resolution_type == ResolutionType::IrvTieBreak
                    && r.contest_id.as_ref() == Some(&tie_resolution.contest_id)
            })
            .max_by_key(|r| r.created_at)
            .ok_or((
                Status::BadRequest,
                format!(
                    "No resolution found for contest {}",
                    tie_resolution.contest_id
                ),
            ))?;

        // Validate selected candidate is in tied candidates
        let tied_candidate_ids = extract_tied_candidate_ids(
            latest_resolution,
            &tie_resolution.contest_id,
        )?;

        if !tied_candidate_ids.contains(&tie_resolution.selected_candidate_id) {
            return Err((
                Status::BadRequest,
                format!(
                    "Selected candidate {} is not in tied candidates for contest {}: {:?}",
                    tie_resolution.selected_candidate_id,
                    tie_resolution.contest_id,
                    tied_candidate_ids
                ),
            ));
        }

        let resolution_value = serde_json::json!({
            "resolved_by_candidate_id": tie_resolution.selected_candidate_id,
            "resolved_at": chrono::Utc::now().to_rfc3339(),
        });

        let resolution_id = latest_resolution.id.clone();

        if !is_resubmission(latest_resolution) {
            // First submission: resolve the existing pending record
            submit_resolution(
                &hasura_transaction,
                &tenant_id,
                &input.election_event_id,
                &resolution_id,
                resolution_value,
                &user_id,
            )
            .await
            .map_err(|e| {
                (
                    Status::InternalServerError,
                    format!("Error submitting resolution: {}", e),
                )
            })?;
        } else {
            // Re-submission: admin changed their mind — update the existing record
            has_previous_resolutions = true;
            event!(
                Level::INFO,
                "Re-submission detected for contest {} - updating existing resolution",
                tie_resolution.contest_id
            );
            update_resolution(
                &hasura_transaction,
                &tenant_id,
                &input.election_event_id,
                &resolution_id,
                resolution_value,
                &user_id,
            )
            .await
            .map_err(|e| {
                (
                    Status::InternalServerError,
                    format!("Error updating resolution: {}", e),
                )
            })?;
        }

        // Log to electoral log
        let event_type = if is_resubmission(latest_resolution) {
            "tally_tie_resolution_updated"
        } else {
            "tally_tie_resolved"
        };
        let electoral_log_body = serde_json::json!({
            "event_type": event_type,
            "tally_session_id": input.tally_session_id,
            "contest_id": tie_resolution.contest_id,
            "resolution_id": resolution_id,
            "round_number": latest_resolution.resolution_data.get("round_number"),
            "tied_candidate_ids": latest_resolution.resolution_data.get("tied_candidate_ids"),
            "resolved_by_candidate_id": tie_resolution.selected_candidate_id.clone(),
            "resolved_by_user": user_id.clone(),
            "message": format!("Tie resolved for contest {} - tally resuming", tie_resolution.contest_id)
        });

        let log_input = LogEventInput {
            election_event_id: input.election_event_id.clone(),
            message_type: LogMessageType::Internal,
            user_id: Some(user_id.clone()),
            username: claims.preferred_username.clone(),
            tenant_id: tenant_id.clone(),
            body: LogEventBody::Plain(
                serde_json::to_string(&electoral_log_body)
                    .unwrap_or_else(|_| "{}".to_string()),
            ),
        };

        let celery_app = get_celery_app().await;
        if let Err(e) = celery_app
            .send_task(enqueue_electoral_log_event::new(log_input))
            .await
        {
            event!(
                Level::WARN,
                "Failed to enqueue tie resolution electoral log event: {}",
                e
            );
        }

        resolved_count += 1;
    }

    let next_status = TallyExecutionStatus::IN_PROGRESS;

    // 7. Update tally session status — always resume so the tally re-runs and
    // produces intermediate results. Windmill will pause again in AWAITING_INPUT
    // if any new tie-breaks are detected during the re-run.
    update_tally_session_status(
        &hasura_transaction,
        &tenant_id,
        &input.election_event_id,
        &input.tally_session_id,
        next_status.clone(),
        false,
    )
    .await
    .map_err(|e| {
        (
            Status::InternalServerError,
            format!("Error updating status: {}", e),
        )
    })?;

    event!(
        Level::INFO,
        "Tally session status set to {:?} after resolution submission",
        next_status
    );

    // 8. Commit transaction
    hasura_transaction.commit().await.map_err(|err| {
        (Status::InternalServerError, format!("Commit failed: {err}"))
    })?;

    event!(
        Level::INFO,
        "Batch tally resolution submission completed for tally session {}, resolved {} contest(s)",
        input.tally_session_id,
        resolved_count
    );

    Ok(Json(SubmitTallyResolutionOutput {
        success: true,
        tally_session_id: input.tally_session_id,
        resolved_count,
    }))
}
