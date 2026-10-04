// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The enrollment approval matrix of an election event: reading the version
//! in force, trying an enrollment against unsaved rules, and saving a new
//! version.

use crate::ports::electoral_log::ApprovalMatrixLog;
use crate::services::authorization::authorize;
use crate::services::dependencies::HarvestServices;
use crate::types::error_response::{ErrorCode, ErrorResponse, JsonError};
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tracing::instrument;
use windmill::postgres::election_event::get_election_event_by_id;
use windmill::services::application::id_card_types;
use windmill::services::approval_matrix::evaluate::{
    decide, Invariant, MatrixSource, MatrixVersion, RuleInputs,
};
use windmill::services::approval_matrix::store::{
    get_latest_approval_matrix, insert_approval_matrix,
};
use windmill::services::approval_matrix::{
    ApprovalMatrix, FieldMatch, IdentityMethod, MatrixError, BUILT_IN_VERSION,
};
use windmill::types::application::{
    ApplicationRejectReason, ApplicationStatus,
};

/// The fields the built-in matrix shows as compared. An election event
/// without a saved matrix compares the fields its enrollment flow sends.
const BUILT_IN_COMPARED_FIELDS: [&str; 5] = [
    "firstName",
    "middleName",
    "lastName",
    "dateOfBirth",
    "embassy",
];

fn internal(context: &str, error: impl std::fmt::Debug) -> JsonError {
    tracing::error!("{context}: {error:?}");
    ErrorResponse::new(
        Status::InternalServerError,
        &format!("{context}: {error:?}"),
        ErrorCode::InternalServerError,
    )
}

fn authorized(
    claims: &JwtClaims,
    permissions: Vec<Permissions>,
) -> Result<(), JsonError> {
    authorize(
        claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        permissions,
    )
    .map_err(|(status, message)| {
        ErrorResponse::new(status, &message, ErrorCode::Unauthorized)
    })
}

fn invalid(errors: &[MatrixError]) -> JsonError {
    let listed: Vec<String> =
        errors.iter().map(MatrixError::to_string).collect();
    ErrorResponse::new(
        Status::BadRequest,
        &format!("Invalid approval matrix: {}", listed.join("; ")),
        ErrorCode::InvalidApprovalMatrix,
    )
}

async fn hasura_client(
    services: &HarvestServices,
) -> Result<DbClient, JsonError> {
    services
        .databases
        .hasura()
        .await
        .get()
        .await
        .map_err(|error| {
            internal("Failed to get client from the db pool", error)
        })
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GetApprovalMatrixInput {
    election_event_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ApprovalMatrixOutput {
    version: i32,
    source: MatrixSource,
    matrix: ApprovalMatrix,
    sha256: Option<String>,
    created_at: Option<String>,
    created_by: Option<String>,
    /// The version the next save creates.
    next_version: i32,
    /// The identity document types a rule can require.
    valid_ids: Vec<String>,
}

fn output(
    matrix: MatrixVersion,
    sha256: Option<String>,
    created_at: Option<String>,
    created_by: Option<String>,
) -> ApprovalMatrixOutput {
    ApprovalMatrixOutput {
        version: matrix.version,
        source: matrix.source,
        next_version: matrix.version.max(BUILT_IN_VERSION) + 1,
        matrix: matrix.matrix,
        sha256,
        created_at,
        created_by,
        valid_ids: id_card_types(),
    }
}

#[instrument(skip(claims, services))]
#[post("/get-approval-matrix", format = "json", data = "<input>")]
pub async fn get_approval_matrix(
    claims: JwtClaims,
    input: Json<GetApprovalMatrixInput>,
    services: &State<HarvestServices>,
) -> Result<Json<ApprovalMatrixOutput>, JsonError> {
    let body = input.into_inner();
    authorized(&claims, vec![Permissions::APPLICATION_READ])?;

    let mut client = hasura_client(services).await?;
    let transaction = client
        .transaction()
        .await
        .map_err(|error| internal("Failed to start transaction", error))?;
    let saved = get_latest_approval_matrix(
        &transaction,
        &claims.hasura_claims.tenant_id,
        &body.election_event_id,
    )
    .await
    .map_err(|error| internal("Failed to read the approval matrix", error))?;

    Ok(Json(match saved {
        Some(saved) => output(
            saved.matrix_version(),
            Some(saved.sha256),
            Some(saved.created_at.to_rfc3339()),
            saved.created_by_username.or(saved.created_by),
        ),
        None => output(
            MatrixVersion::built_in(
                BUILT_IN_COMPARED_FIELDS
                    .iter()
                    .map(|field| field.to_string())
                    .collect(),
            ),
            None,
            None,
            None,
        ),
    }))
}

/// An enrollment as the test panel describes it.
#[derive(Serialize, Deserialize, Debug)]
pub struct TestEnrollment {
    identity: Option<IdentityMethod>,
    voter_found: bool,
    #[serde(default)]
    already_enrolled: bool,
    valid_id: Option<String>,
    #[serde(default)]
    fields: BTreeMap<String, FieldMatch>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EvaluateApprovalMatrixInput {
    election_event_id: String,
    matrix: ApprovalMatrix,
    enrollment: TestEnrollment,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EvaluateApprovalMatrixOutput {
    /// The rule that applies, starting at 1; empty for the last rule.
    rule: Option<usize>,
    decision: Option<ApplicationStatus>,
    reason: Option<ApplicationRejectReason>,
    invariant: Option<Invariant>,
    /// Why the matrix can't be saved. Nothing is decided while it has
    /// errors.
    errors: Vec<MatrixError>,
}

/// Decides the described enrollment with the given rules, saved or not,
/// without storing anything.
#[instrument(skip(claims))]
#[post("/evaluate-approval-matrix", format = "json", data = "<input>")]
pub async fn evaluate_approval_matrix(
    claims: JwtClaims,
    input: Json<EvaluateApprovalMatrixInput>,
) -> Result<Json<EvaluateApprovalMatrixOutput>, JsonError> {
    let body = input.into_inner();
    authorized(&claims, vec![Permissions::APPLICATION_READ])?;

    let errors = body.matrix.validate();
    if !errors.is_empty() {
        return Ok(Json(EvaluateApprovalMatrixOutput {
            rule: None,
            decision: None,
            reason: None,
            invariant: None,
            errors,
        }));
    }

    let enrollment = body.enrollment;
    let fields = if enrollment.voter_found {
        enrollment
            .fields
            .into_iter()
            .filter(|(field, _)| body.matrix.compared_fields.contains(field))
            .collect()
    } else {
        BTreeMap::new()
    };
    let inputs = RuleInputs {
        identity: enrollment.identity,
        voter_found: enrollment.voter_found,
        already_enrolled: enrollment.voter_found && enrollment.already_enrolled,
        valid_id: enrollment.valid_id,
        fields,
        differing: 0,
    }
    .with_counted_differences();
    let decision = decide(&body.matrix, &inputs);

    Ok(Json(EvaluateApprovalMatrixOutput {
        rule: decision.rule,
        decision: Some(decision.outcome.decision),
        reason: decision.outcome.reason,
        invariant: decision.invariant,
        errors,
    }))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SaveApprovalMatrixInput {
    election_event_id: String,
    matrix: ApprovalMatrix,
}

/// Saves the rules as the election event's next version, with its
/// electoral-log entry.
#[instrument(skip(claims, services))]
#[post("/save-approval-matrix", format = "json", data = "<input>")]
pub async fn save_approval_matrix(
    claims: JwtClaims,
    input: Json<SaveApprovalMatrixInput>,
    services: &State<HarvestServices>,
) -> Result<Json<ApprovalMatrixOutput>, JsonError> {
    let body = input.into_inner();
    let tenant_id = &claims.hasura_claims.tenant_id;
    let user_id = &claims.hasura_claims.user_id;
    authorized(&claims, vec![Permissions::APPROVAL_MATRIX_WRITE])?;

    let errors = body.matrix.validate();
    if !errors.is_empty() {
        return Err(invalid(&errors));
    }

    let mut client = hasura_client(services).await?;
    let transaction = client
        .transaction()
        .await
        .map_err(|error| internal("Failed to start transaction", error))?;
    get_election_event_by_id(&transaction, tenant_id, &body.election_event_id)
        .await
        .map_err(|error| {
            ErrorResponse::new(
                Status::NotFound,
                &format!("Election event not found: {error:?}"),
                ErrorCode::ElectionEventNotFound,
            )
        })?;

    let saved = insert_approval_matrix(
        &transaction,
        tenant_id,
        &body.election_event_id,
        &body.matrix,
        Some(user_id),
        claims.preferred_username.as_deref(),
    )
    .await
    .map_err(|error| internal("Failed to save the approval matrix", error))?;

    services
        .electoral_log
        .approval_matrix_updated(
            &transaction,
            ApprovalMatrixLog {
                tenant_id,
                election_event_id: &body.election_event_id,
                user_id,
                username: claims.preferred_username.clone(),
                version: u32::try_from(saved.version).map_err(|error| {
                    internal("Unexpected approval matrix version", error)
                })?,
                sha256: saved.sha256.clone(),
            },
        )
        .await
        .map_err(|error| {
            internal("Failed to post the electoral log message", error)
        })?;

    transaction
        .commit()
        .await
        .map_err(|error| internal("Failed to commit the transaction", error))?;

    Ok(Json(output(
        saved.matrix_version(),
        Some(saved.sha256),
        Some(saved.created_at.to_rfc3339()),
        saved.created_by_username.or(saved.created_by),
    )))
}

#[cfg(test)]
#[path = "../../tests/support/approval_matrix_routes.rs"]
mod route_tests;
