// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::authorization::authorize_voter;
use crate::types::error_response::{ErrorCode, ErrorResponse, JsonError};
use anyhow::Result;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::services::s3;
use sequent_core::types::permissions::VoterPermissions;
use serde::{Deserialize, Serialize};
use tracing::info;
use tracing::instrument;

#[derive(Serialize, Deserialize, Debug)]
pub struct GetBallotFilesUrlsInput {
    election_event_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GetBallotFilesOutput {
    urls: Vec<String>,
}

#[instrument(skip(claims))]
#[post("/get-ballot-files-urls", format = "json", data = "<body>")]
pub async fn get_ballot_files_urls(
    body: Json<GetBallotFilesUrlsInput>,
    claims: JwtClaims,
) -> Result<Json<GetBallotFilesOutput>, JsonError> {
    if claims.hasura_claims.authorized_election_ids.is_none() {
        return Err(ErrorResponse::new(
            Status::NotAcceptable,
            "Voter not authorized",
            ErrorCode::Unauthorized,
        ));
    }
    authorize_voter(&claims, vec![VoterPermissions::USER_ROLE], None).map_err(
        |_e| {
            ErrorResponse::new(
                Status::Unauthorized,
                "Voter not authorized",
                ErrorCode::Unauthorized,
            )
        },
    )?;
    let input = body.into_inner();
    let election_event_id = input.election_event_id.clone();
    let tenant_id = claims.hasura_claims.tenant_id.clone();
    let area_id = claims.hasura_claims.area_id.clone().unwrap_or_default();
    let s3_bucket = s3::get_private_bucket().map_err(|_e| {
        ErrorResponse::new(
            Status::InternalServerError,
            "No private bucket",
            ErrorCode::InternalServerError,
        )
    })?;
    // Find out the last ballot_publication_id and ballot style paths.
    let ballot_publication_file = s3::get_ballot_publication_file_path(
        &tenant_id,
        &election_event_id,
        &area_id,
    );
    let bp_bytes =
        s3::get_file_from_s3(s3_bucket.clone(), ballot_publication_file)
            .await
            .map_err(|_e| {
                ErrorResponse::new(
                    Status::NotFound,
                    ErrorCode::PublicationNotFound.to_string().as_str(),
                    ErrorCode::PublicationNotFound,
                )
            })?;
    // Note: Could be ErrorCode::NoAreaContests, if there is no contest assigned
    // to the area ballot_publication_file is not created at publication time.

    let mut ballot_publications: s3::BallotPublications =
        serde_json::from_slice(&bp_bytes).map_err(|_e| {
            ErrorResponse::new(
                Status::InternalServerError,
                "Error deserializing Ballot Publication",
                ErrorCode::InternalServerError,
            )
        })?;

    // Start pushing the file paths that wee need to create later its signed
    // URLs
    let mut files: Vec<String> = vec![];
    let election_event_file = s3::get_election_event_file_path(
        &tenant_id,
        &election_event_id,
        &ballot_publications.ballot_publication_id,
    );
    files.push(election_event_file);
    let elections_file = s3::get_elections_file_path(
        &tenant_id,
        &election_event_id,
        &area_id,
        &ballot_publications.ballot_publication_id,
    );
    files.push(elections_file);
    files.append(&mut ballot_publications.ballot_style_paths);

    // Get the signed URLs
    let mut urls = vec![];
    for document_s3_key in files {
        let url = s3::get_document_url(document_s3_key, s3_bucket.clone())
            .await
            .map_err(|_e| {
                ErrorResponse::new(
                    Status::InternalServerError,
                    "Error signing url",
                    ErrorCode::InternalServerError,
                )
            })?;
        info!("url: {url:?}");
        urls.push(url);
    }

    Ok(Json(GetBallotFilesOutput { urls }))
}
