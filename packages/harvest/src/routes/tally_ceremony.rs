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
    create_tally_session_resolution, get_pending_resolutions,
    get_resolution_by_tally_session, submit_resolution, ResolutionStatus,
    ResolutionType, TallySessionResolution,
};
use windmill::services::celery_app::get_celery_app;
use windmill::services::providers::transactions_provider::provide_hasura_transaction;
use windmill::services::{
    ceremonies::tally_ceremony, database::get_hasura_pool,
};
use windmill::tasks::electoral_log::{
    enqueue_electoral_log_event, LogEventInput, INTERNAL_MESSAGE_TYPE,
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
        vec![Permissions::ADMIN_CEREMONY],
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

    if execution_status != TallyExecutionStatus::AWAITING_INPUT {
        return Err((
            Status::BadRequest,
            format!(
                "Tally session is not awaiting input. Current status: {}",
                execution_status
            ),
        ));
    }

    // 5. Get all resolutions for this tally session (for re-submission support)
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

        // Check if this is a re-submission
        if latest_resolution.status == ResolutionStatus::Resolved {
            has_previous_resolutions = true;
            event!(
                Level::INFO,
                "Re-submission detected for contest {} - creating new resolution",
                tie_resolution.contest_id
            );
        }

        // Validate selected candidate is in tied candidates
        let tied_candidates = latest_resolution
            .resolution_data
            .get("tied_candidate_ids")
            .and_then(|v| v.as_array())
            .ok_or((
                Status::BadRequest,
                format!(
                    "Invalid resolution data for contest {}: missing tied_candidate_ids",
                    tie_resolution.contest_id
                ),
            ))?;

        let tied_candidate_ids: Vec<String> = tied_candidates
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect();

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

        // Create new resolution row (supports re-submission)
        let new_resolution_id = create_tally_session_resolution(
            &hasura_transaction,
            &tenant_id,
            &input.election_event_id,
            &input.tally_session_id,
            &tie_resolution.contest_id,
            latest_resolution.results_event_id.as_ref().ok_or((
                Status::InternalServerError,
                "Missing results_event_id in resolution".to_string(),
            ))?,
            ResolutionType::IrvTieBreak,
            serde_json::json!({
                "round_number": latest_resolution.resolution_data.get("round_number"),
                "tied_candidate_ids": latest_resolution.resolution_data.get("tied_candidate_ids"),
                "vote_counts": latest_resolution.resolution_data.get("vote_counts"),
            }),
        )
        .await
        .map_err(|e| {
            (
                Status::InternalServerError,
                format!("Error creating resolution: {}", e),
            )
        })?;

        // Immediately mark the new resolution as resolved
        let resolution = serde_json::json!({
            "resolved_by_candidate_id": tie_resolution.selected_candidate_id,
            "resolved_at": chrono::Utc::now().to_rfc3339(),
        });

        submit_resolution(
            &hasura_transaction,
            &tenant_id,
            &input.election_event_id,
            &new_resolution_id,
            resolution,
            &user_id,
        )
        .await
        .map_err(|e| {
            (
                Status::InternalServerError,
                format!("Error submitting resolution: {}", e),
            )
        })?;

        // Log to electoral log
        let electoral_log_body = serde_json::json!({
            "event_type": "tally_tie_resolved",
            "tally_session_id": input.tally_session_id,
            "contest_id": tie_resolution.contest_id,
            "resolution_id": new_resolution_id,
            "round_number": latest_resolution.resolution_data.get("round_number"),
            "tied_candidate_ids": latest_resolution.resolution_data.get("tied_candidate_ids"),
            "resolved_by_candidate_id": tie_resolution.selected_candidate_id.clone(),
            "resolved_by_user": user_id.clone(),
            "message": format!("Tie resolved for contest {} - tally resuming", tie_resolution.contest_id)
        });

        let log_input = LogEventInput {
            election_event_id: input.election_event_id.clone(),
            message_type: INTERNAL_MESSAGE_TYPE.to_string(),
            user_id: Some(user_id.clone()),
            username: claims.preferred_username.clone(),
            tenant_id: tenant_id.clone(),
            body: serde_json::to_string(&electoral_log_body)
                .unwrap_or_else(|_| "{}".to_string()),
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

    // 7. Update tally session status
    if has_previous_resolutions {
        // Re-submission: reset status to trigger re-execution
        update_tally_session_status(
            &hasura_transaction,
            &tenant_id,
            &input.election_event_id,
            &input.tally_session_id,
            TallyExecutionStatus::STARTED,
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
            "Re-submission detected - tally status set to STARTED for re-execution"
        );
    } else {
        // First submission: set to IN_PROGRESS as before
        update_tally_session_status(
            &hasura_transaction,
            &tenant_id,
            &input.election_event_id,
            &input.tally_session_id,
            TallyExecutionStatus::IN_PROGRESS,
            false,
        )
        .await
        .map_err(|e| {
            (
                Status::InternalServerError,
                format!("Error updating status: {}", e),
            )
        })?;
    }

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

/// Get all pending tie resolutions for a tally session
#[instrument(skip(claims))]
#[get("/pending-tie-resolutions?<election_event_id>&<tally_session_id>")]
pub async fn get_pending_tie_resolutions_endpoint(
    election_event_id: String,
    tally_session_id: String,
    claims: JwtClaims,
) -> Result<Json<Vec<TallySessionResolution>>, (Status, String)> {
    // Authorize
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::ADMIN_CEREMONY],
    )?;

    let tenant_id = claims.hasura_claims.tenant_id.clone();

    // Get DB connection
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

    // Get pending resolutions
    let resolutions = get_pending_resolutions(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        &tally_session_id,
    )
    .await
    .map_err(|e| {
        (
            Status::InternalServerError,
            format!("Error fetching pending resolutions: {}", e),
        )
    })?;

    hasura_transaction.commit().await.map_err(|err| {
        (Status::InternalServerError, format!("Commit failed: {err}"))
    })?;

    Ok(Json(resolutions))
}
