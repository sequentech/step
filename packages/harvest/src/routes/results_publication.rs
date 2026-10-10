// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::{
    authorize, authorize_election_permission_labels,
};
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::Permissions;
use tracing::error;
use windmill::postgres::tally_results_publication::get_publication_by_id;
use windmill::services::database::get_hasura_pool;
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
        ResultsPublicationServiceError::Internal(internal) => {
            // Clients get a generic message; keep the cause for operators.
            error!("Results publication failed: {internal:#}");
            return (
                Status::InternalServerError,
                "Internal server error".into(),
            );
        }
    };
    (status, error.to_string())
}

async fn hasura_client() -> Result<DbClient, (Status, String)> {
    get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))
}

async fn authorize_results_permission_labels(
    claims: &JwtClaims,
    input: &PublishResultsWebsiteInput,
) -> Result<(), (Status, String)> {
    let mut hasura_db_client = hasura_client().await?;
    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;
    authorize_election_permission_labels(
        &hasura_transaction,
        claims,
        &input.election_event_id,
        Some(&input.election_ids),
    )
    .await
}

async fn authorize_publication_permission_labels(
    claims: &JwtClaims,
    input: &RevokeResultsPublicationInput,
) -> Result<(), (Status, String)> {
    let mut hasura_db_client = hasura_client().await?;
    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;
    let publication = get_publication_by_id(
        &hasura_transaction,
        &claims.hasura_claims.tenant_id,
        &input.election_event_id,
        &input.publication_id,
    )
    .await
    .map_err(|e| (Status::InternalServerError, e.to_string()))?;
    authorize_election_permission_labels(
        &hasura_transaction,
        claims,
        &input.election_event_id,
        Some(&publication.election_ids),
    )
    .await
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
    let input = body.into_inner();
    authorize_results_permission_labels(&claims, &input).await?;

    let executed_by_user = claims
        .name
        .as_deref()
        .unwrap_or(&claims.hasura_claims.user_id);
    let output = request_results_website_publication(
        &claims.hasura_claims.tenant_id,
        &claims.hasura_claims.user_id,
        claims.preferred_username.clone(),
        executed_by_user,
        &input,
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
    let input = body.into_inner();
    authorize_publication_permission_labels(&claims, &input).await?;

    let output = revoke_results_publication_request(
        &claims.hasura_claims.tenant_id,
        &claims.hasura_claims.user_id,
        claims.preferred_username.clone(),
        &input,
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

#[cfg(test)]
#[path = "../../tests/support/publication_errors.rs"]
mod boundary_tests;

#[cfg(test)]
#[path = "../../tests/support/results_service_errors.rs"]
mod results_service_errors;
