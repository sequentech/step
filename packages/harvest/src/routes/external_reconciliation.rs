// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Admin-portal-facing Datafix reconciliation routes. Unlike
//! `routes::api_datafix` (the *inbound* Datafix->Sequent API, gated by
//! `DatafixClaims`/`DATAFIX_ACCOUNT`), these are triggered by an admin
//! operator through the wizard and authorized like every other admin action
//! (`JwtClaims` + `authorize`).

use crate::services::authorization::authorize;
use anyhow::Result;
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use windmill::postgres::document::get_document;
use windmill::postgres::election_event::{
    get_election_event_by_id, ElectionEventDatafix,
};
use windmill::services::celery_app::get_celery_app;
use windmill::services::consolidation::eml_generator::ValidateAnnotations;
use windmill::services::database::get_hasura_pool;
use windmill::services::documents::get_document_as_temp_file;
use windmill::services::external::reconciliation::diff::ReconciliationDiff;
use windmill::services::external::types::ReconciliationPatchSource;
use windmill::services::tasks_execution::{
    post as post_task_execution, update_fail,
};
use windmill::tasks::apply_reconciliation_patch::{
    apply_reconciliation_patch, ApplyReconciliationPatchBody,
};
use windmill::tasks::generate_reconciliation_patches::{
    generate_reconciliation_patches, GenerateReconciliationPatchesBody,
};
use windmill::types::tasks::ETasksExecution;

#[derive(Serialize, Debug)]
pub struct DatafixReconciliationTaskOutput {
    pub task_execution: TasksExecution,
}

#[derive(Deserialize, Debug)]
pub struct CreateDatafixReconciliationImportInput {
    pub election_event_id: String,
    pub document_id: String,
}

/// Kicks off `generate_reconciliation_patches` for an uploaded reconciliation
/// file. Mirrors `import_users_f`'s shape (insert a `task_execution` row,
/// enqueue the Celery task, return the task_execution for the admin portal's
/// widget) rather than `create_tally_sheet_import`'s synchronous shape, since
/// reconciliation files can be 100k+ rows and always need the async task
/// path. There is no row to insert here — the generate task is the only
/// record of this round until it produces the diff-envelope document. The
/// uploaded file itself isn't read here at all: `generate_reconciliation_patches`
/// downloads it once (to parse it) and hashes those same bytes for the
/// electoral log; re-downloading it here just to check a client-computed
/// hash would be a second, redundant download that doesn't protect anything
/// the electoral log's own hash doesn't already cover — that hash exists so
/// Datafix's own generated hash can be compared against it manually, not as
/// an upload integrity gate.
#[instrument(skip(claims))]
#[post("/create-reconciliation-import", format = "json", data = "<body>")]
pub async fn create_reconciliation_import(
    claims: JwtClaims,
    body: Json<CreateDatafixReconciliationImportInput>,
) -> Result<Json<DatafixReconciliationTaskOutput>, (Status, String)> {
    let input = body.into_inner();
    let tenant_id = claims.hasura_claims.tenant_id.clone();
    authorize(
        &claims,
        true,
        Some(tenant_id.clone()),
        vec![Permissions::ELECTION_EVENT_VOTER_LIST_SYNC],
    )
    .map_err(|err| (Status::Forbidden, format!("{err:?}")))?;

    let executer_name = claims
        .name
        .clone()
        .unwrap_or_else(|| claims.hasura_claims.user_id.clone());

    let task_execution = post_task_execution(
        &tenant_id,
        Some(&input.election_event_id),
        ETasksExecution::GENERATE_RECONCILIATION_PATCHES,
        &executer_name,
    )
    .await
    .map_err(|err| {
        (
            Status::InternalServerError,
            format!("Failed to insert task execution record: {err:?}"),
        )
    })?;

    let task_body = GenerateReconciliationPatchesBody {
        tenant_id: tenant_id.clone(),
        election_event_id: input.election_event_id.clone(),
        source_document_id: input.document_id.clone(),
        requested_by_user_id: claims.hasura_claims.user_id.clone(),
        requested_by_username: claims.name.clone(),
    };

    let celery_app = get_celery_app().await;
    if let Err(err) = celery_app
        .send_task(generate_reconciliation_patches::new(
            task_body,
            task_execution.clone(),
        ))
        .await
    {
        let message =
            format!("Failed to enqueue reconciliation generation: {err}");
        update_fail(&task_execution, &message).await.ok();
        return Err((Status::InternalServerError, message));
    }

    Ok(Json(DatafixReconciliationTaskOutput { task_execution }))
}

#[derive(Deserialize, Debug)]
pub struct ApplyDatafixReconciliationChangesInput {
    pub election_event_id: String,
    /// The `ReconciliationDiff` envelope document id from the generate round
    /// being applied (the frontend already fetched and parsed this document
    /// to render the review tables, so it has this id in hand).
    pub diff_document_id: String,
}

/// Kicks off `apply_reconciliation_patch`. Re-validates server-side that the
/// referenced round's Datafix-side diff is empty — the same check the
/// frontend uses to enable the "Apply" button, re-run here (by independently
/// re-fetching and re-parsing the diff-envelope document) because the client
/// is never trusted to enforce it.
#[instrument(skip(claims))]
#[post("/apply-reconciliation-changes", format = "json", data = "<body>")]
pub async fn apply_reconciliation_changes(
    claims: JwtClaims,
    body: Json<ApplyDatafixReconciliationChangesInput>,
) -> Result<Json<DatafixReconciliationTaskOutput>, (Status, String)> {
    let input = body.into_inner();
    let tenant_id = claims.hasura_claims.tenant_id.clone();
    authorize(
        &claims,
        true,
        Some(tenant_id.clone()),
        vec![Permissions::ELECTION_EVENT_VOTER_LIST_SYNC],
    )
    .map_err(|err| (Status::Forbidden, format!("{err:?}")))?;

    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|err| (Status::InternalServerError, format!("{err:?}")))?;
    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|err| (Status::InternalServerError, format!("{err:?}")))?;

    let document = get_document(
        &hasura_transaction,
        &tenant_id,
        Some(input.election_event_id.clone()),
        &input.diff_document_id,
    )
    .await
    .map_err(|err| (Status::InternalServerError, format!("{err:?}")))?
    .ok_or_else(|| {
        (
            Status::NotFound,
            "Reconciliation diff not found".to_string(),
        )
    })?;
    let temp_file = get_document_as_temp_file(&tenant_id, &document)
        .await
        .map_err(|err| (Status::InternalServerError, format!("{err:?}")))?;
    let bytes = std::fs::read(temp_file.path()).map_err(|err| {
        (
            Status::InternalServerError,
            format!("Error reading reconciliation diff: {err}"),
        )
    })?;
    let envelope: ReconciliationDiff =
        serde_json::from_slice(&bytes).map_err(|err| {
            (
                Status::InternalServerError,
                format!("Error parsing reconciliation diff: {err}"),
            )
        })?;

    if envelope.external_patch_document_id.is_some() {
        return Err((
            Status::Conflict,
            "The external-side diff is not empty — apply the external patch and re-import first".to_string(),
        ));
    }
    if !envelope.apply_allowed {
        return Err((
            Status::Conflict,
            "This reconciliation envelope is a diff-only convergence check and cannot be applied"
                .to_string(),
        ));
    }

    // Every reconciliation round today comes from Datafix — resolve its
    // `CountyMun` so the apply task can record the round's source (see
    // `ReconciliationPatchSource`), gating its own Datafix-specific
    // bookkeeping without the generic apply logic needing to know about it.
    let election_event = get_election_event_by_id(
        &hasura_transaction,
        &tenant_id,
        &input.election_event_id,
    )
    .await
    .map_err(|err| (Status::InternalServerError, format!("{err:?}")))?;
    let datafix_annotations = ElectionEventDatafix(election_event)
        .get_annotations()
        .map_err(|err| (Status::InternalServerError, format!("{err:?}")))?;
    let source = ReconciliationPatchSource::Datafix {
        county_mun: datafix_annotations.voterview_request.county_mun,
    };

    hasura_transaction
        .commit()
        .await
        .map_err(|err| (Status::InternalServerError, format!("{err:?}")))?;

    let executer_name = claims
        .name
        .clone()
        .unwrap_or_else(|| claims.hasura_claims.user_id.clone());

    let task_execution = post_task_execution(
        &tenant_id,
        Some(&input.election_event_id),
        ETasksExecution::APPLY_RECONCILIATION_PATCH,
        &executer_name,
    )
    .await
    .map_err(|err| {
        (
            Status::InternalServerError,
            format!("Failed to insert task execution record: {err:?}"),
        )
    })?;

    let task_body = ApplyReconciliationPatchBody {
        tenant_id: tenant_id.clone(),
        election_event_id: input.election_event_id.clone(),
        source,
        diff_document_id: input.diff_document_id.clone(),
        applied_by_user_id: claims.hasura_claims.user_id.clone(),
        applied_by_username: claims.name.clone(),
    };

    let celery_app = get_celery_app().await;
    if let Err(err) = celery_app
        .send_task(apply_reconciliation_patch::new(
            task_body,
            task_execution.clone(),
        ))
        .await
    {
        let message = format!("Failed to enqueue reconciliation apply: {err}");
        update_fail(&task_execution, &message).await.ok();
        return Err((Status::InternalServerError, message));
    }

    Ok(Json(DatafixReconciliationTaskOutput { task_execution }))
}
