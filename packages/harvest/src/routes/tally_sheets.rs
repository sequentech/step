// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use anyhow::{Context, Result};
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::services::tally_sheet_validation::validate_area_contest_results;
use sequent_core::types::ceremonies::{
    AutomaticRecountPolicy, TallyExecutionStatus,
};
use sequent_core::types::hasura::core::{TallySession, TallySheet};
use sequent_core::types::permissions::Permissions;
use sequent_core::types::tally_sheet_import::{
    TallySheetImportItemStatus, TallySheetImportReviewDecision,
    TallySheetImportSourceFormat, TallySheetImportStatus,
    TallySheetImportValidationError,
};
use sequent_core::types::tally_sheets::{
    AreaContestResults, TallySheetStatus, VotingChannel,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use tracing::{event, instrument, Level};
use windmill::postgres::{
    contest::get_contest_by_id,
    document::get_document,
    election_event::get_election_event_by_id,
    tally_session::get_tally_sessions_by_election_event_id,
    tally_sheet,
    tally_sheet_import::get_tally_sheet_import_items_for_review,
};
use windmill::services::{
    celery_app::get_celery_app,
    ceremonies::tally_ceremony::{
        begin_tally_session_recount,
        reset_tally_session_status_after_failed_recount_task,
    },
    database::get_hasura_pool,
    documents::get_document_as_temp_file,
    ess_xml_converter::convert_ess_enhanced_xml_to_csv,
    tally_sheet_import::{
        application::{
            create_tally_sheet_import as create_tally_sheet_import_service,
            preview_tally_sheet_import as preview_tally_sheet_import_service,
            review_tally_sheet_import as review_tally_sheet_import_service,
        },
        errors::TallySheetImportError,
        hash::hash_bytes,
    },
};
use windmill::tasks::execute_tally_session::execute_tally_session;

/// Maps a tally sheet import service error to its HTTP status, downcasting
/// to the known `TallySheetImportError` domain variants and defaulting to
/// 500 for anything unexpected (infra/db failures).
fn map_tally_sheet_import_error(error: anyhow::Error) -> (Status, String) {
    match error.downcast_ref::<TallySheetImportError>() {
        Some(TallySheetImportError::NotFound(_))
        | Some(TallySheetImportError::DocumentNotFound(_)) => {
            (Status::NotFound, format!("{error:?}"))
        }
        Some(TallySheetImportError::DocumentTooLarge { .. }) => {
            (Status::PayloadTooLarge, format!("{error:?}"))
        }
        Some(TallySheetImportError::InvalidReviewState { .. }) => {
            (Status::Conflict, format!("{error:?}"))
        }
        None => (Status::InternalServerError, format!("{error:?}")),
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateNewTallySheetInput {
    election_event_id: String,
    channel: VotingChannel,
    content: AreaContestResults,
    contest_id: String,
    area_id: String,
}

#[instrument(skip(claims))]
#[post("/create-new-tally-sheet", format = "json", data = "<body>")]
pub async fn create_new_tally_sheet(
    body: Json<CreateNewTallySheetInput>,
    claims: JwtClaims,
) -> Result<Json<TallySheet>, (Status, String)> {
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::TALLY_SHEET_CREATE],
    )?;
    let input = body.into_inner();
    let validation_errors = validate_area_contest_results(&input.content);
    if !validation_errors.is_empty() {
        let messages = validation_errors
            .into_iter()
            .map(|error| format!("{}: {}", error.code, error.message))
            .collect::<Vec<String>>()
            .join("; ");
        return Err((
            Status::BadRequest,
            format!("Invalid tally sheet content: {messages}"),
        ));
    }

    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    let contest_opt = get_contest_by_id(
        &hasura_transaction,
        &claims.hasura_claims.tenant_id,
        &input.election_event_id,
        &input.contest_id,
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    let Some(contest) = contest_opt else {
        return Err((
            Status::NotFound,
            format!("Contest {} not found ", input.contest_id),
        ));
    };

    tally_sheet::lock_ballot_box_version_assignment(
        &hasura_transaction,
        &claims.hasura_claims.tenant_id,
        &input.election_event_id,
        &contest.election_id,
        &input.area_id,
        &input.contest_id,
        &input.channel,
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    let version = tally_sheet::get_latest_ballot_box_version(
        &hasura_transaction,
        &claims.hasura_claims.tenant_id,
        &input.election_event_id,
        &contest.election_id,
        &input.area_id,
        &input.contest_id,
        &input.channel,
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    let new_tally_sheet = tally_sheet::insert_tally_sheet(
        &hasura_transaction,
        &claims.hasura_claims.tenant_id,
        &input.election_event_id,
        &contest.election_id,
        &input.contest_id,
        &input.area_id,
        &input.content,
        &input.channel,
        &claims.hasura_claims.user_id,
        TallySheetStatus::PENDING,
        version + 1,
        None,
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    hasura_transaction
        .commit()
        .await
        .with_context(|| "error committing transaction")
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    Ok(Json(new_tally_sheet.clone()))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ReviewTallySheetInput {
    election_event_id: String,
    tally_sheet_id: String,
    new_status: TallySheetStatus,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PreviewTallySheetImportInput {
    election_event_id: String,
    document_id: String,
    sha256: Option<String>,
    source_format: TallySheetImportSourceFormat,
    selected_channel: VotingChannel,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateTallySheetImportInput {
    election_event_id: String,
    document_id: String,
    sha256: Option<String>,
    source_format: TallySheetImportSourceFormat,
    selected_channel: VotingChannel,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ReviewTallySheetImportInput {
    election_event_id: String,
    import_id: String,
    decision: TallySheetImportReviewDecision,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TallySheetImportPreviewOutput {
    preview: Value,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TallySheetImportOutput {
    import: Value,
}

#[instrument(skip(claims))]
#[post("/review-tally-sheet", format = "json", data = "<body>")]
pub async fn review_tally_sheet(
    body: Json<ReviewTallySheetInput>,
    claims: JwtClaims,
) -> Result<Json<TallySheet>, (Status, String)> {
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::TALLY_SHEET_REVIEW],
    )?;
    let input = body.into_inner();
    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    let review_outcome = tally_sheet::review_tally_sheet_status(
        &hasura_transaction,
        &claims.hasura_claims.tenant_id,
        &input.election_event_id,
        &input.tally_sheet_id,
        &claims.hasura_claims.user_id,
        input.new_status.clone(),
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;

    let tally_sheet = match review_outcome {
        tally_sheet::ReviewTallySheetOutcome::Reviewed(t) => t,
        tally_sheet::ReviewTallySheetOutcome::NotPending(t) => {
            return Err((
                Status::Conflict,
                format!(
                    "Tally sheet {} cannot be reviewed from status {}",
                    t.id, t.status
                ),
            ));
        }
        tally_sheet::ReviewTallySheetOutcome::NotFound => {
            return Err((
                Status::NotFound,
                "Tally sheet not found".to_string(),
            ));
        }
    };

    if input.new_status == TallySheetStatus::APPROVED {
        tally_sheet::soft_delete_tally_sheet_leftover_versions(
            &hasura_transaction,
            &tally_sheet,
        )
        .await
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;
    }

    hasura_transaction
        .commit()
        .await
        .with_context(|| "error committing transaction")
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;

    Ok(Json(tally_sheet.clone()))
}

#[instrument(skip(claims))]
#[post("/preview-tally-sheet-import", format = "json", data = "<body>")]
pub async fn preview_tally_sheet_import(
    body: Json<PreviewTallySheetImportInput>,
    claims: JwtClaims,
) -> Result<Json<TallySheetImportPreviewOutput>, (Status, String)> {
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::TALLY_SHEET_IMPORT_CREATE],
    )?;
    let input = body.into_inner();
    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    let (_document, source_bytes) = read_import_document(
        &hasura_transaction,
        &claims.hasura_claims.tenant_id,
        &input.election_event_id,
        &input.document_id,
    )
    .await
    .map_err(map_tally_sheet_import_error)?;
    verify_source_sha256(input.sha256.as_deref(), &source_bytes)
        .map_err(|e| (Status::BadRequest, format!("{e:?}")))?;
    let (canonical_csv, conversion_validation_errors) = canonical_csv_bytes(
        &source_bytes,
        &input.source_format,
        &input.selected_channel,
    )
    .map_err(|e| (Status::BadRequest, format!("{e:?}")))?;

    let preview = preview_tally_sheet_import_service(
        &hasura_transaction,
        &claims.hasura_claims.tenant_id,
        &input.election_event_id,
        &input.document_id,
        input.source_format,
        input.selected_channel,
        &canonical_csv,
        conversion_validation_errors,
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;

    hasura_transaction
        .commit()
        .await
        .with_context(|| "error committing transaction")
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;

    Ok(Json(TallySheetImportPreviewOutput {
        preview: serde_json::to_value(preview)
            .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?,
    }))
}

#[instrument(skip(claims))]
#[post("/create-tally-sheet-import", format = "json", data = "<body>")]
pub async fn create_tally_sheet_import(
    body: Json<CreateTallySheetImportInput>,
    claims: JwtClaims,
) -> Result<Json<TallySheetImportOutput>, (Status, String)> {
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::TALLY_SHEET_IMPORT_CREATE],
    )?;
    let input = body.into_inner();
    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    let (document, source_bytes) = read_import_document(
        &hasura_transaction,
        &claims.hasura_claims.tenant_id,
        &input.election_event_id,
        &input.document_id,
    )
    .await
    .map_err(map_tally_sheet_import_error)?;
    verify_source_sha256(input.sha256.as_deref(), &source_bytes)
        .map_err(|e| (Status::BadRequest, format!("{e:?}")))?;
    let (canonical_csv, conversion_validation_errors) = canonical_csv_bytes(
        &source_bytes,
        &input.source_format,
        &input.selected_channel,
    )
    .map_err(|e| (Status::BadRequest, format!("{e:?}")))?;

    let import = create_tally_sheet_import_service(
        &hasura_transaction,
        &claims.hasura_claims.tenant_id,
        &input.election_event_id,
        &input.document_id,
        document.name.as_deref(),
        input.source_format,
        input.selected_channel,
        &canonical_csv,
        &source_bytes,
        &claims.hasura_claims.user_id,
        conversion_validation_errors,
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;

    hasura_transaction
        .commit()
        .await
        .with_context(|| "error committing transaction")
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;

    Ok(Json(TallySheetImportOutput {
        import: serde_json::to_value(import)
            .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?,
    }))
}

#[instrument(skip(claims))]
#[post("/review-tally-sheet-import", format = "json", data = "<body>")]
pub async fn review_tally_sheet_import(
    body: Json<ReviewTallySheetImportInput>,
    claims: JwtClaims,
) -> Result<Json<TallySheetImportOutput>, (Status, String)> {
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::TALLY_SHEET_IMPORT_REVIEW],
    )?;
    let ReviewTallySheetImportInput {
        election_event_id,
        import_id,
        decision,
    } = body.into_inner();
    let should_trigger_recount =
        decision == TallySheetImportReviewDecision::APPROVE;
    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    let import = review_tally_sheet_import_service(
        &hasura_transaction,
        &claims.hasura_claims.tenant_id,
        &election_event_id,
        &import_id,
        decision,
        &claims.hasura_claims.user_id,
    )
    .await
    .map_err(map_tally_sheet_import_error)?;

    hasura_transaction
        .commit()
        .await
        .with_context(|| "error committing transaction")
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;

    if should_trigger_recount
        && import.status == TallySheetImportStatus::APPROVED
    {
        // The review itself already committed above; a failure here must not
        // turn into a request failure, or the client would retry a review
        // that already succeeded (and get "cannot be reviewed from status
        // APPROVED"). Log it instead so it can be triaged/retried out of band.
        match maybe_trigger_automatic_recount_for_import(
            &claims.hasura_claims.tenant_id,
            &election_event_id,
            &import_id,
        )
        .await
        {
            Ok(recount_count) => {
                event!(
                    Level::INFO,
                    "Automatic recount policy processed for tally sheet import {}, enqueued {} recount task(s)",
                    import_id,
                    recount_count
                );
            }
            Err(err) => {
                event!(
                    Level::ERROR,
                    "Failed to process automatic recount policy for tally sheet import {}: {err:?}",
                    import_id,
                );
            }
        }
    }

    Ok(Json(TallySheetImportOutput {
        import: serde_json::to_value(import)
            .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?,
    }))
}

async fn maybe_trigger_automatic_recount_for_import(
    tenant_id: &str,
    election_event_id: &str,
    import_id: &str,
) -> Result<usize> {
    let mut hasura_db_client: DbClient =
        get_hasura_pool().await.get().await.with_context(|| {
            "error getting hasura db pool for automatic recount"
        })?;
    let hasura_transaction =
        hasura_db_client.transaction().await.with_context(|| {
            "error starting automatic recount selection transaction"
        })?;

    let election_event = get_election_event_by_id(
        &hasura_transaction,
        tenant_id,
        election_event_id,
    )
    .await?;
    if election_event.automatic_recount_policy()
        != AutomaticRecountPolicy::ENABLED
    {
        hasura_transaction.commit().await.with_context(|| {
            "error committing automatic recount policy read"
        })?;
        return Ok(0);
    }

    let items = get_tally_sheet_import_items_for_review(
        &hasura_transaction,
        tenant_id,
        election_event_id,
        import_id,
    )
    .await?;
    let affected_election_ids: HashSet<String> = items
        .into_iter()
        .filter(|item| {
            item.generated_tally_sheet_id.is_some()
                && item.status == TallySheetImportItemStatus::APPROVED
        })
        .map(|item| item.election_id)
        .collect();
    if affected_election_ids.is_empty() {
        hasura_transaction.commit().await.with_context(|| {
            "error committing automatic recount no-op selection"
        })?;
        return Ok(0);
    }

    let tally_sessions = get_tally_sessions_by_election_event_id(
        &hasura_transaction,
        tenant_id,
        election_event_id,
        false,
    )
    .await?;
    let sessions_to_recount: Vec<TallySession> = tally_sessions
        .into_iter()
        .filter(|session| {
            session.is_execution_completed
                && session.execution_status.as_deref()
                    == Some(TallyExecutionStatus::SUCCESS.to_string().as_str())
        })
        .filter(|session| {
            session
                .election_ids
                .as_ref()
                .map(|election_ids| {
                    election_ids
                        .iter()
                        .any(|id| affected_election_ids.contains(id))
                })
                .unwrap_or(false)
        })
        .collect();

    hasura_transaction
        .commit()
        .await
        .with_context(|| "error committing automatic recount selection")?;

    // Each session's recount is independent: one session failing (e.g. a
    // corrupt/legacy status blob) must not stop the rest of the batch from
    // being attempted, or later sessions would silently miss their
    // automatic recount for this import with no retry.
    let mut enqueued_recount_count = 0usize;
    for tally_session in sessions_to_recount {
        let tally_session_id = tally_session.id.clone();
        match enqueue_automatic_recount_tally_session(
            tenant_id,
            election_event_id,
            &tally_session,
        )
        .await
        {
            Ok(true) => enqueued_recount_count += 1,
            Ok(false) => {}
            Err(err) => {
                event!(
                    Level::ERROR,
                    "Failed to enqueue automatic recount for tally session {}: {err:?}",
                    tally_session_id,
                );
            }
        }
    }

    Ok(enqueued_recount_count)
}

/// Returns `Ok(true)` if a recount task was enqueued, `Ok(false)` if the
/// session has no execution history to recount (skipped, not an error — see
/// `begin_tally_session_recount`), or `Err` on a genuine failure.
async fn enqueue_automatic_recount_tally_session(
    tenant_id: &str,
    election_event_id: &str,
    tally_session: &TallySession,
) -> Result<bool> {
    let tally_session_id = tally_session.id.clone();
    let election_ids = tally_session.election_ids.clone().unwrap_or_default();
    let mut hasura_db_client: DbClient =
        get_hasura_pool().await.get().await.with_context(|| {
            "error getting hasura db pool for automatic recount status update"
        })?;
    let hasura_transaction =
        hasura_db_client.transaction().await.with_context(|| {
            "error starting automatic recount status transaction"
        })?;

    let Some((last_execution, original_status)) = begin_tally_session_recount(
        &hasura_transaction,
        tenant_id,
        election_event_id,
        &tally_session_id,
        &election_ids,
    )
    .await
    .with_context(|| "error starting automatic tally session recount")?
    else {
        // The session is marked SUCCESS/completed but has no execution
        // history, so it was never actually tallied: nothing to recount.
        // Such a session is also inert for a *first* tally, since
        // process_board_impl only enqueues sessions still IN_PROGRESS with
        // is_execution_completed = false. Logged at WARN so the mismatch
        // stays visible rather than being silently skipped.
        event!(
            Level::WARN,
            "Tally session {} is marked completed but has no execution history \
             (never actually tallied); skipping automatic recount",
            tally_session_id,
        );
        return Ok(false);
    };

    hasura_transaction
        .commit()
        .await
        .with_context(|| "error committing automatic recount status update")?;

    let celery_app = get_celery_app().await;
    let task = celery_app
        .send_task(execute_tally_session::new(
            tenant_id.to_string(),
            election_event_id.to_string(),
            tally_session_id.clone(),
            tally_session.tally_type.clone(),
            tally_session.election_ids.clone(),
            true, // force_new_results_id: automatic recount always produces a fresh results event
        ))
        .await;

    if let Err(err) = task {
        reset_tally_session_status_after_failed_recount_task(
            tenant_id,
            election_event_id,
            &tally_session_id,
            &last_execution,
            original_status,
            &format!("{err:?}"),
        )
        .await?;
        return Err(anyhow::anyhow!(
            "Failed to send automatic recount task: {err:?}"
        ));
    }

    event!(
        Level::INFO,
        "Sent automatic recount tally task for election_event_id={}, tally_session_id={}",
        election_event_id,
        tally_session_id,
    );

    Ok(true)
}

const MAX_TALLY_SHEET_IMPORT_BYTES: u64 = 50 * 1024 * 1024;

async fn read_import_document(
    transaction: &deadpool_postgres::Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    document_id: &str,
) -> Result<(sequent_core::types::hasura::core::Document, Vec<u8>)> {
    let document = get_document(
        transaction,
        tenant_id,
        Some(election_event_id.to_string()),
        document_id,
    )
    .await?
    .ok_or_else(|| {
        TallySheetImportError::DocumentNotFound(document_id.to_string())
    })?;

    if let Some(document_size) = document.size {
        let normalized_size = u64::try_from(document_size).map_err(|_| {
            anyhow::anyhow!(
                "Document {document_id} has invalid size {document_size}"
            )
        })?;
        if normalized_size > MAX_TALLY_SHEET_IMPORT_BYTES {
            return Err(TallySheetImportError::DocumentTooLarge {
                document_id: document_id.to_string(),
                size: normalized_size,
                max: MAX_TALLY_SHEET_IMPORT_BYTES,
            }
            .into());
        }
    }

    let file = get_document_as_temp_file(tenant_id, &document).await?;
    let file_size = tokio::fs::metadata(file.path()).await?.len();
    if file_size > MAX_TALLY_SHEET_IMPORT_BYTES {
        return Err(TallySheetImportError::DocumentTooLarge {
            document_id: document_id.to_string(),
            size: file_size,
            max: MAX_TALLY_SHEET_IMPORT_BYTES,
        }
        .into());
    }
    let bytes = tokio::fs::read(file.path()).await?;
    Ok((document, bytes))
}

/// Returns the canonical CSV bytes plus any validation errors already known
/// before parsing (only possible for XML sources, where a problem scoped to
/// one Contest skips just that contest instead of failing the whole file).
/// The `Result` here is reserved for genuinely file-wide problems (invalid
/// UTF-8, unparseable XML, an unreadable CSV byte stream).
fn canonical_csv_bytes(
    source_bytes: &[u8],
    source_format: &TallySheetImportSourceFormat,
    selected_channel: &VotingChannel,
) -> Result<(Vec<u8>, Vec<TallySheetImportValidationError>)> {
    match source_format {
        TallySheetImportSourceFormat::CANONICAL_CSV => {
            Ok((source_bytes.to_vec(), Vec::new()))
        }
        TallySheetImportSourceFormat::ESS_ENHANCED_XML => {
            convert_ess_enhanced_xml_to_csv(
                source_bytes,
                selected_channel.clone(),
            )
        }
    }
}

fn verify_source_sha256(
    expected_sha256: Option<&str>,
    source_bytes: &[u8],
) -> Result<()> {
    let Some(expected_sha256) = expected_sha256
        .map(str::trim)
        .filter(|expected_sha256| !expected_sha256.is_empty())
    else {
        return Ok(());
    };

    let actual_sha256 = hash_bytes(source_bytes);
    if actual_sha256.eq_ignore_ascii_case(expected_sha256) {
        return Ok(());
    }

    Err(anyhow::anyhow!(
        "Uploaded source SHA-256 mismatch: expected {}, got {}",
        expected_sha256,
        actual_sha256
    ))
}
