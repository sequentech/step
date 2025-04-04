// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::authorization::authorize_voter;
use anyhow::Result;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::services::s3;
use sequent_core::types::permissions::VoterPermissions;
use serde::{Deserialize, Serialize};
use tracing::instrument;

#[derive(Serialize, Deserialize, Debug)]
pub struct GetBallotFilesUrlsInput {
    election_event_id: String,
    election_id: String,
    ballot_publication_id: String,
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
) -> Result<Json<GetBallotFilesOutput>, (Status, String)> {
    let input = body.into_inner();
    authorize_voter(
        &claims,
        vec![VoterPermissions::USER_ROLE],
        Some(input.election_id.clone()),
    )?;

    let election_event_id = input.election_event_id.clone();
    let election_id = input.election_id.clone();
    let ballot_publication_id = input.ballot_publication_id.clone();
    let tenant_id = claims.hasura_claims.tenant_id.clone();
    let area_id = claims.hasura_claims.area_id.clone().unwrap_or_default();
    let s3_bucket = s3::get_private_bucket()
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;
    let mut files: Vec<String> = vec![];
    let mut urls: Vec<String> = vec![];

    let election_event_file = s3::get_election_event_file_path(
        &tenant_id,
        &election_event_id,
        &ballot_publication_id,
    );
    files.push(election_event_file);
    let elections_file = s3::get_elections_file_path(
        &tenant_id,
        &election_event_id,
        &area_id,
        &ballot_publication_id,
    );
    files.push(elections_file);
    let ballot_style_file = s3::get_ballot_style_file_path(
        &tenant_id,
        &election_event_id,
        &area_id,
        &ballot_publication_id,
        &election_id,
    );
    files.push(ballot_style_file);

    for document_s3_key in files {
        let url =
            s3::get_document_url(document_s3_key, false, s3_bucket.clone())
                .await
                .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;
        info!("url: {url:?}");
        urls.push(url);
    }

    Ok(Json(GetBallotFilesOutput { urls }))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GetBallotPublicationUrlInput {
    election_event_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GetBallotPublicationUrlOutput {
    url: String,
}

#[instrument(skip(claims))]
#[post("/get-ballot-publication-url", format = "json", data = "<body>")]
pub async fn get_ballot_publication_url(
    body: Json<GetBallotPublicationUrlInput>,
    claims: JwtClaims,
) -> Result<Json<GetBallotPublicationUrlOutput>, (Status, String)> {
    let input = body.into_inner();
    authorize_voter(&claims, vec![VoterPermissions::USER_ROLE], None)?;

    let election_event_id = input.election_event_id.clone();
    let tenant_id = claims.hasura_claims.tenant_id.clone();
    let area_id = claims.hasura_claims.area_id.clone().unwrap_or_default();
    let s3_bucket = s3::get_private_bucket()
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;

    let document_s3_key = s3::get_ballot_publication_file_path(
        &tenant_id,
        &election_event_id,
        &area_id,
    );

    let url = s3::get_document_url(document_s3_key, false, s3_bucket.clone())
        .await
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;
    info!("url: {url:?}");

    Ok(Json(GetBallotPublicationUrlOutput { url }))
}
