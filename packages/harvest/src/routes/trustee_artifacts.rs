// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::services::s3;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::instrument;

/// Input for requesting a presigned upload URL for a trustee artifact (ballots,
/// mixes, decryption factors, plaintexts, etc.) associated with an election
/// event.
#[derive(Serialize, Deserialize, Debug)]
pub struct TrusteeArtifactUploadInput {
    pub election_event_id: String,
    /// Free-form artifact kind (e.g. "BALLOTS", "MIX", "DECRYPTION_FACTORS", "PLAINTEXTS").
    pub artifact_kind: String,
    pub file_name: String,
    pub media_type: String,
    pub size: usize,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TrusteeArtifactUploadOutput {
    pub url: String,
    pub bucket: String,
    pub key: String,
}

/// Input for requesting a presigned download URL for a trustee artifact.
#[derive(Serialize, Deserialize, Debug)]
pub struct TrusteeArtifactDownloadInput {
    pub bucket: String,
    pub key: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TrusteeArtifactDownloadOutput {
    pub url: String,
}

#[instrument(skip(claims))]
#[post("/trustee/get-artifact-upload-url", format = "json", data = "<body>")]
pub async fn get_artifact_upload_url(
    claims: JwtClaims,
    body: Json<TrusteeArtifactUploadInput>,
) -> Result<Json<TrusteeArtifactUploadOutput>, (Status, String)> {
    // Trustees must be allowed to participate in ceremonies to upload artifacts.
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::TRUSTEE_CEREMONY],
    )?;

    let inner = body.into_inner();
    let tenant_id = &claims.hasura_claims.tenant_id;

    // Key layout is specific for trustee protocol artifacts and separate from
    // generic documents.
    // Example: tenant-{tenant}/election-event-{event}/braid/{kind}/{filename}
    let key = format!(
        "tenant-{}/election-event-{}/braid/{}/{}",
        tenant_id, inner.election_event_id, inner.artifact_kind, inner.file_name
    );

    let bucket = s3::get_private_bucket().map_err(|err| {
        (
            Status::InternalServerError,
            format!("Error getting private S3 bucket: {err}"),
        )
    })?;

    // Artifacts are always private and use the regular (non-local) config.
    let url = s3::get_upload_url(key.clone(), false, false)
        .await
        .map_err(|err| {
            (
                Status::InternalServerError,
                format!("Error generating presigned upload URL: {err}"),
            )
        })?;

    Ok(Json(TrusteeArtifactUploadOutput { url, bucket, key }))
}

#[instrument(skip(claims))]
#[post("/trustee/get-artifact-download-url", format = "json", data = "<body>")]
pub async fn get_artifact_download_url(
    claims: JwtClaims,
    body: Json<TrusteeArtifactDownloadInput>,
) -> Result<Json<TrusteeArtifactDownloadOutput>, (Status, String)> {
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::TRUSTEE_CEREMONY],
    )?;

    let inner = body.into_inner();

    let url = s3::get_document_url(inner.key.clone(), inner.bucket.clone())
        .await
        .map_err(|err| {
            (
                Status::InternalServerError,
                format!("Error generating presigned download URL: {err}"),
            )
        })?;

    Ok(Json(TrusteeArtifactDownloadOutput { url }))
}

