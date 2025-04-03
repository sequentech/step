// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::authorization::authorize;
use anyhow::Result;
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::services::s3;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::instrument;
use windmill::{
    postgres::election_event::get_election_event_by_id,
    services::database::get_hasura_pool,
};

#[derive(Serialize, Deserialize, Debug)]
pub struct GetSignedUrlsInput {
    election_event_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GetSignedUrlsOutput {
    urls: Vec<String>,
}

#[instrument(skip(claims))]
#[post("/get-signed-urls", format = "json", data = "<body>")]
pub async fn get_signed_urls(
    body: Json<GetSignedUrlsInput>,
    claims: JwtClaims,
) -> Result<Json<GetSignedUrlsOutput>, (Status, String)> {
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::DOCUMENT_DOWNLOAD], // TODO
    )?;
    let input = body.into_inner();
    info!("input: {input:?}");

    let election_event_id = input.election_event_id.clone();
    let tenant_id = claims.hasura_claims.tenant_id.clone();
    let area_id = claims.hasura_claims.area_id.clone().unwrap_or_default();
    let user_id = claims.hasura_claims.user_id.clone();
    let auth_election_ids =
        claims.hasura_claims.authorized_election_ids.clone();

    let s3_bucket = s3::get_private_bucket()
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;
    let mut urls: Vec<String> = vec![];

    // WIP... get needed urls
    let document_s3_key = s3::get_public_ballot_publication_file_path(
        &tenant_id,
        &election_event_id,
        &area_id,
    );
    let url = s3::get_document_url(document_s3_key, s3_bucket.clone())
        .await
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;
    urls.push(url);

    Ok(Json(GetSignedUrlsOutput { urls }))
}
