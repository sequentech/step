// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::authorization::authorize;
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use windmill::services::{database::get_hasura_pool, documents};
#[derive(Serialize, Deserialize, Debug)]
pub struct UploadDocumentInput {
    name: String,
    media_type: String,
    size: usize,
    is_public: bool,
    is_local: Option<bool>,
    election_event_id: Option<String>,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct UploadDocumentOutput {
    document_id: String,
    url: String,
}

#[instrument(skip(claims))]
#[post("/get-upload-url", format = "json", data = "<body>")]
pub async fn get_upload_url(
    claims: JwtClaims,
    body: Json<UploadDocumentInput>,
) -> Result<Json<UploadDocumentOutput>, (Status, String)> {
    let inner = body.into_inner();
    authorize_upload(&claims, &inner)?;

    let mut hasura_db_client: DbClient =
        get_hasura_pool().await.get().await.map_err(|err| {
            (
                Status::InternalServerError,
                format!("Error loading hasura db client: {err}"),
            )
        })?;
    let hasura_transaction =
        hasura_db_client.transaction().await.map_err(|err| {
            (
                Status::InternalServerError,
                format!("Error creating a transaction: {err}"),
            )
        })?;

    let (document, url) = documents::get_upload_url(
        &hasura_transaction,
        &inner.name,
        &inner.media_type,
        inner.size,
        &claims.hasura_claims.tenant_id,
        inner.is_public,
        inner.is_local,
        inner.election_event_id,
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    hasura_transaction.commit().await.map_err(|err| {
        (
            Status::InternalServerError,
            format!("Failed to commit Hasura transactionn: {err}"),
        )
    })?;

    Ok(Json(UploadDocumentOutput {
        document_id: document.id,
        url,
    }))
}

/// Existing generic upload access is unchanged. Tenant restore and signed
/// election import may create only a fresh private archive outside an event.
/// documents::get_upload_url allocates the document UUID server-side; callers
/// cannot choose an existing object key through this alternative.
fn authorize_upload(
    claims: &JwtClaims,
    input: &UploadDocumentInput,
) -> Result<(), (Status, String)> {
    let check = |permission| {
        authorize(
            claims,
            true,
            Some(claims.hasura_claims.tenant_id.clone()),
            vec![permission],
        )
    };
    match check(Permissions::DOCUMENT_UPLOAD) {
        Ok(()) => Ok(()),
        Err(denied) => {
            if is_private_archive(input)
                && (check(Permissions::TENANT_WRITE).is_ok()
                    || check(Permissions::ELECTION_EVENT_CREATE).is_ok())
            {
                Ok(())
            } else {
                Err(denied)
            }
        }
    }
}

fn is_private_archive(input: &UploadDocumentInput) -> bool {
    let name = &input.name;
    if input.is_public
        || input.is_local == Some(true)
        || input.election_event_id.is_some()
        || name.is_empty()
        || name.len() > 255
        || name.starts_with('.')
        || name.trim() != name
        || name.chars().any(|ch| {
            ch.is_control() || matches!(ch, '/' | '\\' | '%' | '?' | '#')
        })
    {
        return false;
    }
    let lower = name.to_ascii_lowercase();
    if lower.ends_with(".zip") {
        matches!(
            input.media_type.as_str(),
            "application/zip"
                | "application/x-zip-compressed"
                | "application/octet-stream"
                | ""
        )
    } else {
        lower.ends_with(".ezip") && input.media_type == "application/ezip"
    }
}
