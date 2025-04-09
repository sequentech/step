// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::authorization::authorize_voter;
use anyhow::Result;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::services::s3;
use sequent_core::types::hasura::core::BallotPublication;
use sequent_core::types::hasura::core::Election;
use sequent_core::types::permissions::VoterPermissions;
use serde::{Deserialize, Serialize};
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
) -> Result<Json<GetBallotFilesOutput>, (Status, String)> {
    let input = body.into_inner();
    authorize_voter(&claims, vec![VoterPermissions::USER_ROLE], None)?;

    let election_event_id = input.election_event_id.clone();
    let tenant_id = claims.hasura_claims.tenant_id.clone();
    let area_id = claims.hasura_claims.area_id.clone().unwrap_or_default();
    let s3_bucket = s3::get_private_bucket()
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;

    // 1. Find out the ballot_publication_ids
    let ballot_publication_file = s3::get_ballot_publication_file_path(
        &tenant_id,
        &election_event_id,
        &area_id,
    );
    let bp_bytes =
        s3::get_file_from_s3(s3_bucket.clone(), ballot_publication_file)
            .await
            .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;
    let ballot_publication_ids: Vec<String> = serde_json::from_slice(&bp_bytes)
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;

    // WIP...
    let mut files: Vec<String> = vec![];
    for ballot_publication_id in ballot_publication_ids {
        // 2. Find out the elections related to this area.
        let elections_file = s3::get_elections_file_path(
            &tenant_id,
            &election_event_id,
            &area_id,
            &ballot_publication_id,
        );
        let elections_bytes =
            s3::get_file_from_s3(s3_bucket.clone(), elections_file.clone())
                .await
                .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;
        let elections: Vec<Election> = serde_json::from_slice(&elections_bytes)
            .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;

        // 3. Start pushing the file paths that wee need to create its signed
        //    URLs
        let election_event_file = s3::get_election_event_file_path(
            &tenant_id,
            &election_event_id,
            &ballot_publication_id,
        );
        files.push(election_event_file);
        files.push(elections_file);
        for election in elections {
            let election_id = &election.id;
            authorize_voter(
                &claims,
                vec![], // Role is already checked
                Some(election_id.to_string()),
            )?;
            let ballot_style_file = s3::get_ballot_style_file_path(
                &tenant_id,
                &election_event_id,
                &area_id,
                &ballot_publication_id,
                election_id,
            );
            files.push(ballot_style_file);
        }
    }

    // 4. Get the signed URLs
    let mut urls: Vec<String> = vec![];
    for document_s3_key in files {
        let url = s3::get_document_url(document_s3_key, s3_bucket.clone())
            .await
            .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;
        info!("url: {url:?}");
        urls.push(url);
    }

    Ok(Json(GetBallotFilesOutput { urls }))
}
