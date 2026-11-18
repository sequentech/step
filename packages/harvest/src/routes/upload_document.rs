// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::authorization::authorize;
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::services::keycloak;
use sequent_core::types::hasura::core::DocumentAnnotations;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use windmill::services::{
    database::get_hasura_pool, documents,
    providers::transactions_provider::provide_hasura_transaction,
};
#[derive(Serialize, Deserialize, Debug)]
pub struct UploadDocumentInput {
    name: String,
    media_type: String,
    size: usize,
    is_public: bool,
    is_local: Option<bool>,
    election_event_id: Option<String>,
    /// Set for files that can hold voter passwords or secret attributes, such
    /// as voter and election event imports. Downloading the file then needs
    /// the voter secret attribute read permission.
    voter_secret_attributes: Option<bool>,
}

/// Restricting a document only narrows who can download it, so any uploader
/// may ask for it. Public documents are served without permission checks, so
/// they cannot be restricted.
fn upload_annotations(
    is_public: bool,
    voter_secret_attributes: Option<bool>,
) -> Result<Option<DocumentAnnotations>, String> {
    match (voter_secret_attributes.unwrap_or(false), is_public) {
        (false, _) => Ok(None),
        (true, true) => Err(
            "A public document cannot be restricted to voter secret readers"
                .to_string(),
        ),
        (true, false) => Ok(Some(DocumentAnnotations::voter_secret_export())),
    }
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
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::DOCUMENT_UPLOAD],
    )?;

    let inner = body.into_inner();
    let annotations =
        upload_annotations(inner.is_public, inner.voter_secret_attributes)
            .map_err(|err| (Status::BadRequest, err))?;

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
        annotations.as_ref(),
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

#[cfg(test)]
mod tests {
    use super::upload_annotations;

    #[test]
    fn leaves_ordinary_uploads_unrestricted() {
        for voter_secret_attributes in [None, Some(false)] {
            for is_public in [true, false] {
                assert!(upload_annotations(is_public, voter_secret_attributes)
                    .unwrap()
                    .is_none());
            }
        }
    }

    #[test]
    fn restricts_private_voter_secret_uploads() {
        let annotations = upload_annotations(false, Some(true)).unwrap();
        assert!(annotations
            .is_some_and(|annotations| annotations
                .requires_voter_secret_attribute_read()));
    }

    #[test]
    fn refuses_public_voter_secret_uploads() {
        assert!(upload_annotations(true, Some(true)).is_err());
    }
}
