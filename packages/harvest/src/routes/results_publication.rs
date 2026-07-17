// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::Permissions;
use windmill::services::results_publication::{
    configure_results_website_policy_request, fetch_results_artifact_request,
    refresh_results_publication_index_request,
    request_results_website_publication, resolve_results_publication_request,
    revoke_results_publication_request, ResultsPublicationServiceError,
};
use windmill::types::results_publication::{
    ConfigureResultsWebsitePolicyInput, ConfigureResultsWebsitePolicyOutput,
    FetchResultsArtifactInput, FetchResultsArtifactOutput,
    PublishResultsWebsiteInput, PublishResultsWebsiteOutput,
    RefreshResultsPublicationIndexInput, RefreshResultsPublicationIndexOutput,
    ResolveResultsPublicationInput, ResolveResultsPublicationOutput,
    RevokeResultsPublicationInput, RevokeResultsPublicationOutput,
};

type RouteResult<T> = std::result::Result<Json<T>, (Status, String)>;

fn map_service_error(
    error: ResultsPublicationServiceError,
) -> (Status, String) {
    let status = match &error {
        ResultsPublicationServiceError::BadRequest(_) => Status::BadRequest,
        ResultsPublicationServiceError::Unauthorized(_) => Status::Unauthorized,
        ResultsPublicationServiceError::Forbidden(_) => Status::Forbidden,
        ResultsPublicationServiceError::NotFound(_) => Status::NotFound,
        ResultsPublicationServiceError::Conflict(_) => Status::Conflict,
        ResultsPublicationServiceError::Internal(_) => {
            Status::InternalServerError
        }
    };
    (status, error.to_string())
}

#[post("/configure-results-website-policy", format = "json", data = "<body>")]
pub async fn configure_results_website_policy(
    body: Json<ConfigureResultsWebsitePolicyInput>,
    claims: JwtClaims,
) -> RouteResult<ConfigureResultsWebsitePolicyOutput> {
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::PUBLISH_RESULTS_WRITE],
    )?;

    let output = configure_results_website_policy_request(
        &claims.hasura_claims.tenant_id,
        &body.into_inner(),
    )
    .await
    .map_err(map_service_error)?;
    Ok(Json(output))
}

#[post("/publish-results-website", format = "json", data = "<body>")]
pub async fn publish_results_website(
    body: Json<PublishResultsWebsiteInput>,
    claims: JwtClaims,
) -> RouteResult<PublishResultsWebsiteOutput> {
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::PUBLISH_RESULTS_WRITE],
    )?;

    let executed_by_user = claims
        .name
        .as_deref()
        .unwrap_or(&claims.hasura_claims.user_id);
    let output = request_results_website_publication(
        &claims.hasura_claims.tenant_id,
        &claims.hasura_claims.user_id,
        claims.preferred_username.clone(),
        executed_by_user,
        &body.into_inner(),
    )
    .await
    .map_err(map_service_error)?;
    Ok(Json(output))
}

#[post("/resolve-results-publication", format = "json", data = "<body>")]
pub async fn resolve_results_publication(
    body: Json<ResolveResultsPublicationInput>,
    claims: JwtClaims,
) -> RouteResult<Option<ResolveResultsPublicationOutput>> {
    let output =
        resolve_results_publication_request(&claims, &body.into_inner())
            .await
            .map_err(map_service_error)?;
    Ok(Json(output))
}

#[post("/fetch-results-artifact", format = "json", data = "<body>")]
pub async fn fetch_results_artifact(
    body: Json<FetchResultsArtifactInput>,
    claims: JwtClaims,
) -> RouteResult<FetchResultsArtifactOutput> {
    let output = fetch_results_artifact_request(&claims, &body.into_inner())
        .await
        .map_err(map_service_error)?;
    Ok(Json(output))
}

#[post("/revoke-results-publication", format = "json", data = "<body>")]
pub async fn revoke_results_publication(
    body: Json<RevokeResultsPublicationInput>,
    claims: JwtClaims,
) -> RouteResult<RevokeResultsPublicationOutput> {
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::PUBLISH_RESULTS_WRITE],
    )?;

    let output = revoke_results_publication_request(
        &claims.hasura_claims.tenant_id,
        &claims.hasura_claims.user_id,
        claims.preferred_username.clone(),
        &body.into_inner(),
    )
    .await
    .map_err(map_service_error)?;
    Ok(Json(output))
}

#[post("/refresh-results-publication-index", format = "json", data = "<body>")]
pub async fn refresh_results_publication_index(
    body: Json<RefreshResultsPublicationIndexInput>,
    claims: JwtClaims,
) -> RouteResult<RefreshResultsPublicationIndexOutput> {
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::PUBLISH_RESULTS_WRITE],
    )?;

    let output = refresh_results_publication_index_request(
        &claims.hasura_claims.tenant_id,
        &body.into_inner(),
    )
    .await
    .map_err(map_service_error)?;
    Ok(Json(output))
}
