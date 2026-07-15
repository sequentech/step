// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use anyhow::Result;
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::ballot::ResultsWebsiteVisibilityScope;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::Permissions;
use serde_json::Value;
use windmill::postgres::election_event::get_election_event_by_id;
use windmill::postgres::tally_results_publication::{
    get_active_publication_for_route, get_publication_by_id,
    insert_publishing_publication, mark_publication_failed, revoke_publication,
    NewTallyResultsPublication, TallyResultsPublication,
};
use windmill::services::celery_app::get_celery_app;
use windmill::services::database::get_hasura_pool;
use windmill::services::documents::get_document_url;
use windmill::services::results_publication::{
    configure_results_website_policy as configure_results_website_policy_service,
    delete_public_publication_route_artifacts, is_results_website_enabled,
    publication_matches_results_website_policy, refresh_public_results_index,
    validate_results_website_policy,
};
use windmill::services::tasks_execution::post;
use windmill::types::results_publication::{
    ConfigureResultsWebsitePolicyInput, ConfigureResultsWebsitePolicyOutput,
    FetchResultsArtifactInput, FetchResultsArtifactOutput,
    PublishResultsWebsiteInput, PublishResultsWebsiteOutput,
    RefreshResultsPublicationIndexInput, RefreshResultsPublicationIndexOutput,
    ResolveResultsPublicationInput, ResolveResultsPublicationOutput,
    ResultsPublicationStatus, ResultsRouteScope, RevokeResultsPublicationInput,
    RevokeResultsPublicationOutput,
};
use windmill::types::tasks::ETasksExecution;

#[post("/configure-results-website-policy", format = "json", data = "<body>")]
pub async fn configure_results_website_policy(
    body: Json<ConfigureResultsWebsitePolicyInput>,
    claims: JwtClaims,
) -> Result<Json<ConfigureResultsWebsitePolicyOutput>, (Status, String)> {
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::PUBLISH_RESULTS_WRITE],
    )?;

    let input = body.into_inner();
    let tenant_id = claims.hasura_claims.tenant_id.clone();
    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    let transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    let output = configure_results_website_policy_service(
        &transaction,
        &tenant_id,
        &input,
    )
    .await
    .map_err(|e| (Status::BadRequest, format!("{:?}", e)))?;
    transaction
        .commit()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    Ok(Json(output))
}

#[post("/publish-results-website", format = "json", data = "<body>")]
pub async fn publish_results_website(
    body: Json<PublishResultsWebsiteInput>,
    claims: JwtClaims,
) -> Result<Json<PublishResultsWebsiteOutput>, (Status, String)> {
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::PUBLISH_RESULTS_WRITE],
    )?;

    let input = body.into_inner();
    input.validate().map_err(|err| {
        (
            Status::BadRequest,
            format!("Invalid publication request: {err:?}"),
        )
    })?;

    let tenant_id = claims.hasura_claims.tenant_id.clone();

    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    let transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    let election_event = get_election_event_by_id(
        &transaction,
        &tenant_id,
        &input.election_event_id,
    )
    .await
    .map_err(|e| (Status::BadRequest, format!("{:?}", e)))?;
    let presentation = election_event
        .get_presentation()
        .map_err(|e| (Status::BadRequest, format!("{:?}", e)))?
        .unwrap_or_default();
    validate_results_website_policy(
        &presentation,
        input.access,
        input.visibility_scope,
    )
    .map_err(|err| {
        (
            Status::BadRequest,
            format!("Invalid publication request: {err:?}"),
        )
    })?;
    transaction
        .commit()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    let executer_name = claims
        .name
        .clone()
        .unwrap_or_else(|| claims.hasura_claims.user_id.clone());
    let task_execution = post(
        &tenant_id,
        Some(&input.election_event_id),
        ETasksExecution::PUBLISH_RESULTS_WEBSITE,
        &executer_name,
    )
    .await
    .map_err(|error| {
        (
            Status::InternalServerError,
            format!("Failed to insert task execution record: {error:?}"),
        )
    })?;

    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    let transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    let publication = insert_publishing_publication(
        &transaction,
        NewTallyResultsPublication {
            tenant_id: &tenant_id,
            election_event_id: &input.election_event_id,
            tally_session_id: &input.tally_session_id,
            tally_session_execution_id: &input.tally_session_execution_id,
            results_event_id: &input.results_event_id,
            task_execution_id: &task_execution.id,
            route_scope: input.route_scope,
            route_election_id: input.route_election_id.as_deref(),
            election_ids: &input.election_ids,
            access: input.access,
            visibility_scope: input.visibility_scope,
            contest_ids: &input.contest_ids,
            published_by_user_id: Some(&claims.hasura_claims.user_id),
        },
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    transaction
        .commit()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    let publication_id = publication.id.clone();
    let task_tenant_id = tenant_id.clone();
    let task_election_event_id = input.election_event_id.clone();
    let celery_app = get_celery_app().await;
    let error_msg = match celery_app
        .send_task(
            windmill::tasks::publish_results_website::publish_results_website_task::new(
                task_tenant_id.clone(),
                task_election_event_id.clone(),
                publication_id.clone(),
                task_execution.clone(),
            ),
        )
        .await
    {
        Ok(_) => None,
        Err(err) => {
            let message = format!("Failed to send PUBLISH_RESULTS_WEBSITE task: {err:?}");
            let mut hasura_db_client: DbClient = get_hasura_pool()
                .await
                .get()
                .await
                .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
            let transaction = hasura_db_client
                .transaction()
                .await
                .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
            mark_publication_failed(
                &transaction,
                &task_tenant_id,
                &task_election_event_id,
                &publication_id,
                &message,
            )
            .await
            .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
            transaction
                .commit()
                .await
                .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
            Some(message)
        }
    };

    Ok(Json(PublishResultsWebsiteOutput {
        publication_id,
        task_execution_id: task_execution.id.clone(),
        publication_status: if error_msg.is_some() {
            ResultsPublicationStatus::Failed
        } else {
            ResultsPublicationStatus::Publishing
        },
        task_execution,
        error_msg,
    }))
}

fn manifest_public_path(
    publication: &TallyResultsPublication,
) -> Option<String> {
    publication
        .documents
        .get("manifest")
        .and_then(|manifest| {
            manifest
                .get("latest_public_path")
                .or_else(|| manifest.get("public_path"))
        })
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn publication_matches_requested_route(
    publication: &TallyResultsPublication,
    election_id: Option<&str>,
) -> bool {
    match publication.route_scope {
        ResultsRouteScope::Election => {
            publication.route_election_id.as_deref().is_some()
                && publication.route_election_id.as_deref() == election_id
        }
        ResultsRouteScope::Event => election_id
            .map(|id| {
                publication
                    .election_ids
                    .iter()
                    .any(|election| election == id)
            })
            .unwrap_or(true),
    }
}

#[post("/resolve-results-publication", format = "json", data = "<body>")]
pub async fn resolve_results_publication(
    body: Json<ResolveResultsPublicationInput>,
    claims: JwtClaims,
) -> Result<Json<Option<ResolveResultsPublicationOutput>>, (Status, String)> {
    let input = body.into_inner();
    let tenant_id = claims.hasura_claims.tenant_id.clone();
    let route_scope = if input.election_id.is_some() {
        ResultsRouteScope::Election
    } else {
        ResultsRouteScope::Event
    };

    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    let transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    let election_event =
        get_election_event_by_id(&transaction, &tenant_id, &input.ee_id)
            .await
            .map_err(|e| (Status::BadRequest, format!("{:?}", e)))?;
    let presentation = election_event
        .get_presentation()
        .map_err(|e| (Status::BadRequest, format!("{:?}", e)))?
        .unwrap_or_default();

    if !is_results_website_enabled(&presentation)
        .map_err(|e| (Status::BadRequest, format!("{:?}", e)))?
    {
        transaction
            .commit()
            .await
            .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
        return Ok(Json(None));
    }

    let mut publication = get_active_publication_for_route(
        &transaction,
        &tenant_id,
        &input.ee_id,
        route_scope,
        input.election_id.as_deref(),
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    if publication.is_none() && input.election_id.is_some() {
        publication = get_active_publication_for_route(
            &transaction,
            &tenant_id,
            &input.ee_id,
            ResultsRouteScope::Event,
            None,
        )
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    }

    let should_hide_publication =
        if let Some(publication) = publication.as_ref() {
            !publication_matches_requested_route(
                publication,
                input.election_id.as_deref(),
            ) || !publication_matches_results_website_policy(
                &presentation,
                publication,
            )
            .map_err(|e| (Status::BadRequest, format!("{:?}", e)))?
        } else {
            false
        };
    if should_hide_publication {
        publication = None;
    }
    transaction
        .commit()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    let Some(publication) = publication else {
        return Ok(Json(None));
    };
    let manifest_public_path = manifest_public_path(&publication);
    let manifest = publication.manifest.clone();

    Ok(Json(Some(ResolveResultsPublicationOutput {
        tenant_id: publication.tenant_id,
        election_event_id: publication.election_event_id,
        access: publication.access,
        route_scope: publication.route_scope,
        election_ids: publication.election_ids,
        publication_id: publication.id,
        manifest_public_path,
        manifest_url: None,
        manifest,
    })))
}

fn get_document_id_from_value(value: &Value) -> Option<String> {
    value
        .get("document_id")
        .and_then(Value::as_str)
        .map(str::to_string)
}

#[post("/fetch-results-artifact", format = "json", data = "<body>")]
pub async fn fetch_results_artifact(
    body: Json<FetchResultsArtifactInput>,
    claims: JwtClaims,
) -> Result<Json<FetchResultsArtifactOutput>, (Status, String)> {
    let input = body.into_inner();
    let tenant_id = claims.hasura_claims.tenant_id.clone();

    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    let transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    let election_event = get_election_event_by_id(
        &transaction,
        &tenant_id,
        &input.election_event_id,
    )
    .await
    .map_err(|e| (Status::BadRequest, format!("{:?}", e)))?;
    let presentation = election_event
        .get_presentation()
        .map_err(|e| (Status::BadRequest, format!("{:?}", e)))?
        .unwrap_or_default();

    if !is_results_website_enabled(&presentation)
        .map_err(|e| (Status::BadRequest, format!("{:?}", e)))?
    {
        return Err((
            Status::NotFound,
            "Results publication is not available".to_string(),
        ));
    }

    let publication = get_publication_by_id(
        &transaction,
        &tenant_id,
        &input.election_event_id,
        &input.publication_id,
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    if publication.publication_status != ResultsPublicationStatus::Published {
        return Err((
            Status::NotFound,
            "Results publication is not available".to_string(),
        ));
    }

    if !publication_matches_requested_route(
        &publication,
        input.election_id.as_deref(),
    ) {
        return Err((
            Status::NotFound,
            "Results publication is not available for this route".to_string(),
        ));
    }

    if !publication_matches_results_website_policy(&presentation, &publication)
        .map_err(|e| (Status::BadRequest, format!("{:?}", e)))?
    {
        return Err((
            Status::NotFound,
            "Results publication is not available".to_string(),
        ));
    }

    let document_ids = if publication.visibility_scope
        == ResultsWebsiteVisibilityScope::AreaBased
    {
        let area_id = claims.hasura_claims.area_id.clone().ok_or((
            Status::Forbidden,
            "No voter area is available".to_string(),
        ))?;
        publication
            .documents
            .get("area_sqlite")
            .and_then(|areas| areas.get(&area_id))
            .and_then(get_document_id_from_value)
            .map(|document_id| vec![document_id])
            .ok_or((
                Status::Forbidden,
                "No results artifact is available for this voter area"
                    .to_string(),
            ))?
    } else {
        publication
            .documents
            .get("full_sqlite")
            .and_then(get_document_id_from_value)
            .map(|document_id| vec![document_id])
            .ok_or((
                Status::NotFound,
                "No results artifact is available".to_string(),
            ))?
    };

    let mut urls = Vec::new();
    for document_id in document_ids {
        let url = get_document_url(
            &transaction,
            &tenant_id,
            Some(&input.election_event_id),
            &document_id,
        )
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?
        .ok_or((Status::NotFound, "Document not found".to_string()))?;
        urls.push(url);
    }
    transaction
        .commit()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    Ok(Json(FetchResultsArtifactOutput { urls }))
}

#[post("/revoke-results-publication", format = "json", data = "<body>")]
pub async fn revoke_results_publication(
    body: Json<RevokeResultsPublicationInput>,
    claims: JwtClaims,
) -> Result<Json<RevokeResultsPublicationOutput>, (Status, String)> {
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::PUBLISH_RESULTS_WRITE],
    )?;

    let input = body.into_inner();
    let tenant_id = claims.hasura_claims.tenant_id.clone();
    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    let transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    let publication = get_publication_by_id(
        &transaction,
        &tenant_id,
        &input.election_event_id,
        &input.publication_id,
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    revoke_publication(
        &transaction,
        &tenant_id,
        &input.election_event_id,
        &input.publication_id,
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    delete_public_publication_route_artifacts(&publication)
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    refresh_public_results_index(
        &transaction,
        &tenant_id,
        &input.election_event_id,
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    transaction
        .commit()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    Ok(Json(RevokeResultsPublicationOutput {
        publication_id: input.publication_id,
        publication_status: ResultsPublicationStatus::Revoked,
    }))
}

#[post("/refresh-results-publication-index", format = "json", data = "<body>")]
pub async fn refresh_results_publication_index(
    body: Json<RefreshResultsPublicationIndexInput>,
    claims: JwtClaims,
) -> Result<Json<RefreshResultsPublicationIndexOutput>, (Status, String)> {
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::PUBLISH_RESULTS_WRITE],
    )?;

    let input = body.into_inner();
    let tenant_id = claims.hasura_claims.tenant_id.clone();
    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    let transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    let election_event = get_election_event_by_id(
        &transaction,
        &tenant_id,
        &input.election_event_id,
    )
    .await
    .map_err(|e| (Status::BadRequest, format!("{:?}", e)))?;
    let presentation = election_event
        .get_presentation()
        .map_err(|e| (Status::BadRequest, format!("{:?}", e)))?
        .unwrap_or_default();
    let results_enabled = is_results_website_enabled(&presentation)
        .map_err(|e| (Status::BadRequest, format!("{:?}", e)))?;

    refresh_public_results_index(
        &transaction,
        &tenant_id,
        &input.election_event_id,
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    transaction
        .commit()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    Ok(Json(RefreshResultsPublicationIndexOutput {
        election_event_id: input.election_event_id,
        results_enabled,
    }))
}
