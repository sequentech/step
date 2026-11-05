// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The schedule as a CSV file (VOTE-LIFECYCLE): preview an uploaded file,
//! import it, and export the event's schedule.

use crate::services::authorization::authorize;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use windmill::services::schedule_csv::{
    export_schedule_document, import_schedule_document,
    preview_schedule_document, ImportOutcome, RowsWithErrors, ScheduleAuthor,
    ScheduleFileError, SchedulePreview,
};

#[derive(Deserialize, Debug)]
pub struct ScheduleDocumentInput {
    election_event_id: String,
    document_id: String,
}

#[derive(Deserialize, Debug)]
pub struct ExportScheduleInput {
    election_event_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ExportScheduleOutput {
    document_id: String,
}

fn authorize_schedule(claims: &JwtClaims) -> Result<(), (Status, String)> {
    authorize(
        claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::SCHEDULED_EVENT_WRITE],
    )
}

/// A file that isn't a schedule, or has row errors, is the caller's to fix
/// (400); anything else is ours (500).
fn schedule_error(error: anyhow::Error) -> (Status, String) {
    if error.downcast_ref::<RowsWithErrors>().is_some()
        || error.downcast_ref::<ScheduleFileError>().is_some()
    {
        (Status::BadRequest, error.to_string())
    } else {
        (Status::InternalServerError, format!("{error:?}"))
    }
}

#[instrument(skip(claims))]
#[post("/preview-schedule-import", format = "json", data = "<input>")]
pub async fn preview_schedule_import(
    claims: JwtClaims,
    input: Json<ScheduleDocumentInput>,
) -> Result<Json<SchedulePreview>, (Status, String)> {
    authorize_schedule(&claims)?;
    let input = input.into_inner();
    let preview = preview_schedule_document(
        &claims.hasura_claims.tenant_id,
        &input.election_event_id,
        &input.document_id,
    )
    .await
    .map_err(schedule_error)?;
    Ok(Json(preview))
}

#[instrument(skip(claims))]
#[post("/import-schedule", format = "json", data = "<input>")]
pub async fn import_schedule(
    claims: JwtClaims,
    input: Json<ScheduleDocumentInput>,
) -> Result<Json<ImportOutcome>, (Status, String)> {
    authorize_schedule(&claims)?;
    let input = input.into_inner();
    let author = ScheduleAuthor {
        user_id: claims.hasura_claims.user_id.clone(),
        username: claims.preferred_username.clone(),
    };
    let outcome = import_schedule_document(
        &claims.hasura_claims.tenant_id,
        &input.election_event_id,
        &input.document_id,
        &author,
    )
    .await
    .map_err(schedule_error)?;
    Ok(Json(outcome))
}

#[instrument(skip(claims))]
#[post("/export-schedule", format = "json", data = "<input>")]
pub async fn export_schedule(
    claims: JwtClaims,
    input: Json<ExportScheduleInput>,
) -> Result<Json<ExportScheduleOutput>, (Status, String)> {
    authorize_schedule(&claims)?;
    let input = input.into_inner();
    let document_id = export_schedule_document(
        &claims.hasura_claims.tenant_id,
        &input.election_event_id,
    )
    .await
    .map_err(|error| (Status::InternalServerError, format!("{error:?}")))?;
    Ok(Json(ExportScheduleOutput { document_id }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_with_row_errors_or_not_a_schedule_is_a_bad_request() {
        let preview = SchedulePreview {
            rows: vec![],
            ok: 0,
            posts: 0,
            errors: 1,
            primary_time_zone: "UTC".to_string(),
            outcome_changes: None,
        };
        let (status, _) = schedule_error(RowsWithErrors(preview).into());
        assert_eq!(status, Status::BadRequest);
        let (status, message) = schedule_error(
            ScheduleFileError("Missing column \"event_type\"".to_string())
                .into(),
        );
        assert_eq!(status, Status::BadRequest);
        assert_eq!(message, "Missing column \"event_type\"");
        let (status, _) = schedule_error(anyhow::anyhow!("database down"));
        assert_eq!(status, Status::InternalServerError);
    }
}
