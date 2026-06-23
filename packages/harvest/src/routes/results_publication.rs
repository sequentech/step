// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use anyhow::{anyhow, Result};
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::services::uuid_validation::parse_uuid_v4;
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use windmill::postgres::election_event::get_election_event_by_id;
use windmill::postgres::tally_results_publication::{
    get_active_publication_for_route, get_publication_by_id,
    insert_publishing_publication, mark_publication_failed, revoke_publication,
    TallyResultsPublication,
};
use windmill::services::celery_app::get_celery_app;
use windmill::services::database::get_hasura_pool;
use windmill::services::documents::get_document_url;
use windmill::services::results_publication::{
    delete_public_publication_route_artifacts, is_results_website_enabled,
    publication_matches_results_website_policy, refresh_public_results_index,
};
use windmill::services::tasks_execution::post;
use windmill::types::tasks::ETasksExecution;

#[derive(Serialize, Deserialize, Debug)]
pub struct PublishResultsWebsiteInput {
    election_event_id: String,
    tally_session_id: String,
    tally_session_execution_id: String,
    results_event_id: String,
    route_scope: String,
    route_election_id: Option<String>,
    election_ids: Vec<String>,
    contest_ids: Vec<String>,
    access: String,
    visibility_scope: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PublishResultsWebsiteOutput {
    publication_id: String,
    task_execution_id: String,
    publication_status: String,
    task_execution: TasksExecution,
    error_msg: Option<String>,
}

fn validate_publish_input(input: &PublishResultsWebsiteInput) -> Result<()> {
    if input.contest_ids.is_empty() {
        return Err(anyhow!("At least one contest must be selected"));
    }

    if input.route_scope != "event" && input.route_scope != "election" {
        return Err(anyhow!("Invalid route scope"));
    }

    if input.route_scope == "event" && input.route_election_id.is_some() {
        return Err(anyhow!("Event route cannot include route_election_id"));
    }

    if input.route_scope == "election" && input.route_election_id.is_none() {
        return Err(anyhow!("Election route requires route_election_id"));
    }

    if input.access != "public" && input.access != "authenticated" {
        return Err(anyhow!("Invalid results access"));
    }

    if input.visibility_scope != "full_event"
        && input.visibility_scope != "area_based"
    {
        return Err(anyhow!("Invalid visibility scope"));
    }

    if input.access == "public" && input.visibility_scope != "full_event" {
        return Err(anyhow!("Public results must use full_event visibility"));
    }

    Ok(())
}

fn validate_results_website_policy_values(
    status: &str,
    access: &str,
    visibility_scope: &str,
) -> Result<()> {
    if status != "enabled" && status != "disabled" {
        return Err(anyhow!("Invalid results website status"));
    }

    if access != "public" && access != "authenticated" {
        return Err(anyhow!("Invalid results access"));
    }

    if visibility_scope != "full_event" && visibility_scope != "area_based" {
        return Err(anyhow!("Invalid visibility scope"));
    }

    if access == "public" && visibility_scope != "full_event" {
        return Err(anyhow!("Public results must use full_event visibility"));
    }

    Ok(())
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ConfigureResultsWebsitePolicyInput {
    election_event_id: String,
    status: String,
    access: String,
    visibility_scope: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ConfigureResultsWebsitePolicyOutput {
    election_event_id: String,
    status: String,
    access: String,
    visibility_scope: String,
}

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
    validate_results_website_policy_values(
        &input.status,
        &input.access,
        &input.visibility_scope,
    )
    .map_err(|err| {
        (
            Status::BadRequest,
            format!("Invalid results website policy: {err:?}"),
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
    let mut presentation = election_event
        .presentation
        .unwrap_or_else(|| Value::Object(Map::new()));

    let Some(presentation_object) = presentation.as_object_mut() else {
        return Err((
            Status::BadRequest,
            "Election event presentation must be a JSON object".to_string(),
        ));
    };

    presentation_object.insert(
        "results_website".to_string(),
        json!({
            "status": input.status.clone(),
            "access": input.access.clone(),
            "visibility_scope": input.visibility_scope.clone(),
        }),
    );

    let statement = transaction
        .prepare(
            r#"
                UPDATE sequent_backend.election_event
                SET presentation = $3
                WHERE tenant_id = $1
                  AND id = $2
                RETURNING id;
            "#,
        )
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    transaction
        .query_one(
            &statement,
            &[
                &parse_uuid_v4(&tenant_id)
                    .map_err(|e| (Status::BadRequest, format!("{:?}", e)))?,
                &parse_uuid_v4(&input.election_event_id)
                    .map_err(|e| (Status::BadRequest, format!("{:?}", e)))?,
                &presentation,
            ],
        )
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

    Ok(Json(ConfigureResultsWebsitePolicyOutput {
        election_event_id: input.election_event_id,
        status: input.status,
        access: input.access,
        visibility_scope: input.visibility_scope,
    }))
}

fn results_website_policy_value<'a>(
    presentation: Option<&'a Value>,
    key: &str,
) -> Option<&'a str> {
    presentation
        .and_then(|value| value.get("results_website"))
        .and_then(|value| value.get(key))
        .and_then(Value::as_str)
}

fn validate_results_website_policy(
    presentation: Option<&Value>,
    input: &PublishResultsWebsiteInput,
) -> Result<()> {
    if results_website_policy_value(presentation, "status") != Some("enabled") {
        return Err(anyhow!(
            "Results website publishing is disabled for this election event"
        ));
    }

    if let Some(policy_access) =
        results_website_policy_value(presentation, "access")
    {
        if policy_access != input.access {
            return Err(anyhow!(
                "Results access does not match the election event results website policy"
            ));
        }
    }

    if let Some(policy_visibility_scope) =
        results_website_policy_value(presentation, "visibility_scope")
    {
        if policy_visibility_scope != input.visibility_scope {
            return Err(anyhow!(
                "Results visibility does not match the election event results website policy"
            ));
        }
    }

    Ok(())
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
    validate_publish_input(&input).map_err(|err| {
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
    validate_results_website_policy(
        election_event.presentation.as_ref(),
        &input,
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
        &tenant_id,
        &input.election_event_id,
        &input.tally_session_id,
        &input.tally_session_execution_id,
        &input.results_event_id,
        &task_execution.id,
        &input.route_scope,
        input.route_election_id.as_deref(),
        &input.election_ids,
        &input.access,
        &input.visibility_scope,
        &input.contest_ids,
        Some(&claims.hasura_claims.user_id),
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
            "Failed".to_string()
        } else {
            "Publishing".to_string()
        },
        task_execution,
        error_msg,
    }))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ResolveResultsPublicationInput {
    ee_id: String,
    election_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ResolveResultsPublicationOutput {
    tenant_id: String,
    election_event_id: String,
    access: String,
    route_scope: String,
    election_ids: Vec<String>,
    publication_id: String,
    manifest_public_path: Option<String>,
    manifest_url: Option<String>,
    manifest: Option<Value>,
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
    match publication.route_scope.as_str() {
        "election" => {
            publication.route_election_id.as_deref().is_some()
                && publication.route_election_id.as_deref() == election_id
        }
        _ => election_id
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
        "election"
    } else {
        "event"
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

    if !is_results_website_enabled(election_event.presentation.as_ref()) {
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
            "event",
            None,
        )
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    }

    let should_hide_publication =
        publication.as_ref().is_some_and(|publication| {
            !publication_matches_requested_route(
                publication,
                input.election_id.as_deref(),
            ) || !publication_matches_results_website_policy(
                election_event.presentation.as_ref(),
                publication,
            )
        });
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

#[derive(Serialize, Deserialize, Debug)]
pub struct FetchResultsArtifactInput {
    election_event_id: String,
    election_id: Option<String>,
    publication_id: String,
    area_ids: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FetchResultsArtifactOutput {
    url: Option<String>,
    urls: Vec<String>,
    artifacts: Value,
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

    if !is_results_website_enabled(election_event.presentation.as_ref()) {
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

    if publication.publication_status != "Published" {
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

    if !publication_matches_results_website_policy(
        election_event.presentation.as_ref(),
        &publication,
    ) {
        return Err((
            Status::NotFound,
            "Results publication is not available".to_string(),
        ));
    }

    let document_ids = if publication.visibility_scope == "area_based" {
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

    Ok(Json(FetchResultsArtifactOutput {
        url: urls.first().cloned(),
        urls,
        artifacts: json!({}),
    }))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RevokeResultsPublicationInput {
    election_event_id: String,
    publication_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RevokeResultsPublicationOutput {
    publication_id: String,
    publication_status: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RefreshResultsPublicationIndexInput {
    election_event_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RefreshResultsPublicationIndexOutput {
    election_event_id: String,
    results_enabled: bool,
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
        publication_status: "Revoked".to_string(),
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
    let results_enabled =
        is_results_website_enabled(election_event.presentation.as_ref());

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
