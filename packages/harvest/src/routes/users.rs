// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use crate::types::error_response::{ErrorCode, ErrorResponse, JsonError};
use crate::types::optional::OptionalId;
use crate::types::resources::{Aggregate, DataList, TotalAggregate};
use anyhow::{anyhow, Result};
use deadpool_postgres::Client as DbClient;
use rocket::futures::future::join_all;
use rocket::http::Status;
use rocket::response::{Responder, Result as ResponseResult};
use rocket::serde::json::Json;
use rocket::Request;
use sequent_core::services::jwt;
use sequent_core::services::keycloak::{
    get_event_realm, get_realm_password_policy, get_tenant_realm,
    get_user_profile_validation_errors, is_keycloak_bad_request,
    PasswordPolicyViolation, UserProfileValidationError,
};
use sequent_core::services::keycloak::{GroupInfo, KeycloakAdminClient};
use sequent_core::types::keycloak::{
    User, UserProfileAttribute, UserProfileConfiguration, PERMISSION_LABELS,
    TENANT_ID_ATTR_NAME,
};
use sequent_core::types::permissions::Permissions;
use serde::Deserialize;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::env;
use tracing::{info, instrument};
use uuid::Uuid;
use windmill::postgres::election_event::get_election_event_by_id;
use windmill::services::cast_votes::get_users_with_vote_info;
use windmill::services::celery_app::get_celery_app;
use windmill::services::database::{get_hasura_pool, get_keycloak_pool};
use windmill::services::datafix::utils::datafix_annotations;
use windmill::services::election::is_election_event_locked_down;
use windmill::services::electoral_log::{
    post_voter_password_change, ElectoralLogAdminContext,
    VoterPasswordChangeSource,
};
use windmill::services::export::export_users::{
    ExportBody, ExportTenantUsersBody, ExportUsersBody,
};
use windmill::services::keycloak_events::list_keycloak_events_by_type;
use windmill::services::tasks_execution::*;
use windmill::services::users::list_users_has_voted;
use windmill::services::users::{
    count_keycloak_users, list_users, list_users_with_vote_info,
};
use windmill::services::users::{FilterOption, ListUsersFilter};
use windmill::services::voter_secret_attributes::{
    decrypt_attribute_values, encrypt_secret_attribute_map, redact_user,
    secret_attribute_names, user_attribute_values,
};
use windmill::tasks::delete_users::{
    self as delete_users_task, DeleteUsersOutput,
};
use windmill::tasks::edit_user::{EditUserOutput, EditUserTaskBody};
use windmill::tasks::export_users::{self, ExportUsersOutput};
use windmill::tasks::import_users::{self, ImportUsersOutput};
use windmill::types::tasks::ETasksExecution;

#[derive(Deserialize, Debug)]
pub struct DeleteUserBody {
    tenant_id: String,
    election_event_id: Option<String>,
    user_id: String,
}

async fn ensure_election_event_not_locked(
    tenant_id: &str,
    election_event_id: &str,
) -> Result<(), (Status, String)> {
    match is_election_event_locked_down(tenant_id, election_event_id).await {
        Ok(false) => Ok(()),
        Ok(true) => Err((
            Status::Forbidden,
            "Election event is locked down".to_string(),
        )),
        Err(err) => Err((
            Status::InternalServerError,
            format!("Failed to check election event lockdown: {err}"),
        )),
    }
}

async fn get_event_secret_attribute_names(
    tenant_id: &str,
    election_event_id: &str,
) -> Result<HashSet<String>, (Status, String)> {
    let realm = get_event_realm(tenant_id, election_event_id);
    let client = KeycloakAdminClient::new().await.map_err(|error| {
        (
            Status::InternalServerError,
            format!("Error connecting to Keycloak: {error:?}"),
        )
    })?;
    let attributes =
        client
            .get_user_profile_attributes(&realm)
            .await
            .map_err(|error| {
                (
                    Status::InternalServerError,
                    format!(
                        "Error reading the Keycloak user profile: {error:?}"
                    ),
                )
            })?;
    secret_attribute_names(&attributes)
        .map_err(|error| (Status::BadRequest, error.to_string()))
}

fn ensure_secret_attributes_not_queried(
    input: &GetUsersBody,
    secret_names: &HashSet<String>,
) -> Result<(), (Status, String)> {
    let filtered_secret = input.attributes.as_ref().and_then(|attributes| {
        attributes.keys().find(|name| secret_names.contains(*name))
    });
    let sorted_secret = input
        .sort
        .as_ref()
        .and_then(|sort| sort.keys().find(|name| secret_names.contains(*name)));
    if let Some(name) = filtered_secret.or(sorted_secret) {
        return Err((
            Status::BadRequest,
            format!("Encrypted voter attribute `{name}` cannot be filtered or sorted"),
        ));
    }
    Ok(())
}

#[instrument(skip(claims))]
#[post("/delete-user", format = "json", data = "<body>")]
pub async fn delete_user(
    claims: jwt::JwtClaims,
    body: Json<DeleteUserBody>,
) -> Result<Json<OptionalId>, (Status, String)> {
    let input = body.into_inner();
    let required_perm: Permissions = if input.election_event_id.is_some() {
        Permissions::VOTER_DELETE
    } else {
        Permissions::USER_WRITE
    };
    authorize(
        &claims,
        true,
        Some(input.tenant_id.clone()),
        vec![required_perm],
    )?;
    if let Some(election_event_id) = input.election_event_id.as_deref() {
        ensure_election_event_not_locked(&input.tenant_id, election_event_id)
            .await?;
    }
    let realm = match input.election_event_id.as_ref() {
        Some(election_event_id) => {
            get_event_realm(&input.tenant_id, election_event_id)
        }
        None => get_tenant_realm(&input.tenant_id),
    };
    let client = KeycloakAdminClient::new().await.map_err(|e| {
        (
            Status::InternalServerError,
            format!("Error obtaining the client: {:?}", e),
        )
    })?;
    client
        .delete_user(&realm, &input.user_id)
        .await
        .map_err(|e| {
            (
                Status::InternalServerError,
                format!("Error deleting the user: {:?}", e),
            )
        })?;
    Ok(Json(Default::default()))
}

#[derive(Deserialize, Debug)]
pub struct DeleteUsersBody {
    tenant_id: String,
    election_event_id: Option<String>,
    election_id: Option<String>,
    /// The explicit selection. Absent when `select_all` is set.
    users_id: Option<Vec<String>>,
    /// Delete every voter matching the filters below rather than an explicit
    /// list. The browser only knows the page it has loaded, so "select all" has
    /// to be resolved server side.
    select_all: Option<bool>,
    /// The same filter set `get-users` accepts. It has to be the same set: any
    /// filter the list applies but the delete does not would resolve to MORE
    /// voters than the operator can see.
    first_name: Option<FilterOption>,
    last_name: Option<FilterOption>,
    username: Option<FilterOption>,
    email: Option<FilterOption>,
    attributes: Option<HashMap<String, String>>,
    has_voted: Option<bool>,
    enabled: Option<bool>,
    email_verified: Option<bool>,
    authorized_to_election_alias: Option<String>,
}

#[instrument(skip(claims))]
#[post("/delete-users", format = "json", data = "<body>")]
pub async fn delete_users(
    claims: jwt::JwtClaims,
    body: Json<DeleteUsersBody>,
) -> Result<Json<DeleteUsersOutput>, (Status, String)> {
    let input = body.into_inner();
    let required_perm: Permissions = if input.election_event_id.is_some() {
        Permissions::VOTER_DELETE
    } else {
        Permissions::USER_WRITE
    };
    authorize(
        &claims,
        true,
        Some(input.tenant_id.clone()),
        vec![required_perm],
    )?;

    let select_all = input.select_all.unwrap_or(false);
    if !select_all && input.users_id.as_ref().map_or(true, |ids| ids.is_empty())
    {
        return Err((
            Status::BadRequest,
            "No voters selected and select_all was not set".to_string(),
        ));
    }
    // Without an election event there is nothing to scope the filters to, so
    // select_all would resolve every non-service account in the tenant realm --
    // every admin, including the caller -- and there is no task_execution row
    // to audit it either. The tenant user list deletes by explicit selection.
    if select_all && input.election_event_id.is_none() {
        return Err((
            Status::BadRequest,
            "select_all requires an election event".to_string(),
        ));
    }
    if let Some(election_event_id) = input.election_event_id.as_deref() {
        ensure_election_event_not_locked(&input.tenant_id, election_event_id)
            .await?;
    }

    let realm = match input.election_event_id.as_ref() {
        Some(election_event_id) => {
            get_event_realm(&input.tenant_id, election_event_id)
        }
        None => get_tenant_realm(&input.tenant_id),
    };

    // Tenant users do not have an election event to own a task_execution row.
    // Keep this path synchronous so Keycloak failures are returned to the
    // caller instead of disappearing in an untracked worker task.
    if input.election_event_id.is_none() {
        let client = KeycloakAdminClient::new()
            .await
            .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;
        let user_ids = input
            .users_id
            .as_ref()
            .ok_or((Status::BadRequest, "No users selected".to_string()))?;
        for id in user_ids {
            client.delete_user(&realm, id).await.map_err(|e| {
                (
                    Status::InternalServerError,
                    format!("Error deleting the user: {e:?}"),
                )
            })?;
        }
        return Ok(Json(DeleteUsersOutput {
            ids: None,
            error_msg: None,
            task_execution: None,
        }));
    }

    let executer_name = claims
        .name
        .clone()
        .unwrap_or_else(|| claims.hasura_claims.user_id.clone());

    let election_event_id = input
        .election_event_id
        .clone()
        .expect("tenant users returned above");
    let task_execution = Some(
        post(
            &input.tenant_id,
            Some(&election_event_id),
            ETasksExecution::DELETE_VOTERS,
            &executer_name,
        )
        .await
        .map_err(|error| {
            (
                Status::InternalServerError,
                format!("Failed to insert task execution record: {error:?}"),
            )
        })?,
    );

    let filter = if select_all {
        Some(ListUsersFilter {
            tenant_id: input.tenant_id.clone(),
            election_event_id: input.election_event_id.clone(),
            election_id: input.election_id.clone(),
            area_id: None,
            realm: realm.clone(),
            search: None,
            first_name: input.first_name,
            last_name: input.last_name,
            username: input.username,
            email: input.email,
            limit: None,
            offset: None,
            user_ids: None,
            attributes: input.attributes,
            enabled: input.enabled,
            email_verified: input.email_verified,
            sort: None,
            has_voted: input.has_voted,
            authorized_to_election_alias: input.authorized_to_election_alias,
        })
    } else {
        None
    };

    let celery_app = get_celery_app().await;
    if let Err(err) = celery_app
        .send_task(delete_users_task::delete_users::new(
            realm,
            if select_all { None } else { input.users_id },
            filter,
            task_execution.clone(),
        ))
        .await
    {
        // Otherwise the row sits IN_PROGRESS forever with nothing to run it.
        if let Some(task_execution) = &task_execution {
            let _ = update_fail(
                task_execution,
                &format!("Failed to enqueue the Delete Voters task: {err}"),
            )
            .await;
        }
        return Ok(Json(DeleteUsersOutput {
            ids: None,
            error_msg: Some(format!("Error sending Delete Voters task: {err}")),
            task_execution,
        }));
    }

    info!("Sent DELETE_VOTERS task");

    Ok(Json(DeleteUsersOutput {
        ids: None,
        error_msg: None,
        task_execution,
    }))
}

#[derive(Deserialize, Debug)]
pub struct GetUsersBody {
    tenant_id: String,
    election_event_id: Option<String>,
    election_id: Option<String>,
    search: Option<String>,
    first_name: Option<FilterOption>,
    last_name: Option<FilterOption>,
    username: Option<FilterOption>,
    email: Option<FilterOption>,
    limit: Option<i32>,
    offset: Option<i32>,
    user_ids: Option<Vec<String>>,
    show_votes_info: Option<bool>,
    attributes: Option<HashMap<String, String>>,
    email_verified: Option<bool>,
    enabled: Option<bool>,
    sort: Option<HashMap<String, String>>,
    has_voted: Option<bool>,
    authorized_to_election_alias: Option<String>,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct CountUserOutput {
    count: i64,
}

#[instrument(skip(claims, body), ret)]
#[post("/count-users", format = "json", data = "<body>")]
pub async fn count_users(
    claims: jwt::JwtClaims,
    body: Json<GetUsersBody>,
) -> Result<Json<CountUserOutput>, (Status, String)> {
    let input = body.into_inner();
    let required_perm: Permissions = if input.election_event_id.is_some() {
        Permissions::VOTER_READ
    } else {
        Permissions::USER_READ
    };
    authorize(
        &claims,
        true,
        Some(input.tenant_id.clone()),
        vec![required_perm],
    )?;

    let realm = match input.election_event_id {
        Some(ref election_event_id) => {
            get_event_realm(&input.tenant_id, &election_event_id)
        }
        None => get_tenant_realm(&input.tenant_id),
    };
    if let Some(election_event_id) = input.election_event_id.as_deref() {
        let secret_names = get_event_secret_attribute_names(
            &input.tenant_id,
            election_event_id,
        )
        .await?;
        ensure_secret_attributes_not_queried(&input, &secret_names)?;
    }

    let mut keycloak_db_client: DbClient =
        get_keycloak_pool().await.get().await.map_err(|e| {
            (
                Status::InternalServerError,
                format!("Error acquiring keycloak db client from pool {:?}", e),
            )
        })?;
    let keycloak_transaction =
        keycloak_db_client.transaction().await.map_err(|e| {
            (
                Status::InternalServerError,
                format!("Error acquiring keycloak transaction {:?}", e),
            )
        })?;
    let mut hasura_db_client: DbClient =
        get_hasura_pool().await.get().await.map_err(|e| {
            (
                Status::InternalServerError,
                format!("Error acquiring hasura db client from pool {:?}", e),
            )
        })?;
    let hasura_transaction =
        hasura_db_client.transaction().await.map_err(|e| {
            (
                Status::InternalServerError,
                format!("Error acquiring hasura transaction {:?}", e),
            )
        })?;

    let filter = ListUsersFilter {
        tenant_id: input.tenant_id.clone(),
        election_event_id: input.election_event_id.clone(),
        election_id: input.election_id.clone(),
        area_id: None,
        realm: realm.clone(),
        search: input.search,
        first_name: input.first_name,
        last_name: input.last_name,
        username: input.username,
        email: input.email,
        limit: input.limit,
        offset: input.offset,
        user_ids: input.user_ids,
        attributes: input.attributes,
        enabled: input.enabled,
        email_verified: input.email_verified,
        sort: input.sort,
        has_voted: input.has_voted,
        authorized_to_election_alias: input.authorized_to_election_alias,
    };

    let count = count_keycloak_users(
        &hasura_transaction,
        &keycloak_transaction,
        filter,
    )
    .await
    .map_err(|e| {
        (
            Status::InternalServerError,
            format!("Error counting users {:?}", e),
        )
    })?;

    Ok(Json(CountUserOutput {
        count: count.into(),
    }))
}

#[instrument(skip(claims, body), ret)]
#[post("/get-users", format = "json", data = "<body>")]
pub async fn get_users(
    claims: jwt::JwtClaims,
    body: Json<GetUsersBody>,
) -> Result<Json<DataList<User>>, (Status, String)> {
    let input = body.into_inner();
    let required_perm: Permissions = if input.election_event_id.is_some() {
        Permissions::VOTER_READ
    } else {
        Permissions::USER_READ
    };
    authorize(
        &claims,
        true,
        Some(input.tenant_id.clone()),
        vec![required_perm],
    )?;

    let realm = match input.election_event_id {
        Some(ref election_event_id) => {
            get_event_realm(&input.tenant_id, &election_event_id)
        }
        None => get_tenant_realm(&input.tenant_id),
    };
    let secret_names =
        if let Some(election_event_id) = input.election_event_id.as_deref() {
            let names = get_event_secret_attribute_names(
                &input.tenant_id,
                election_event_id,
            )
            .await?;
            ensure_secret_attributes_not_queried(&input, &names)?;
            names
        } else {
            HashSet::new()
        };

    let mut keycloak_db_client: DbClient =
        get_keycloak_pool().await.get().await.map_err(|e| {
            (
                Status::InternalServerError,
                format!("Error acquiring keycloak db client from pool {:?}", e),
            )
        })?;
    let keycloak_transaction =
        keycloak_db_client.transaction().await.map_err(|e| {
            (
                Status::InternalServerError,
                format!("Error acquiring keycloak transaction {:?}", e),
            )
        })?;
    let mut hasura_db_client: DbClient =
        get_hasura_pool().await.get().await.map_err(|e| {
            (
                Status::InternalServerError,
                format!("Error acquiring hasura db client from pool {:?}", e),
            )
        })?;
    let hasura_transaction =
        hasura_db_client.transaction().await.map_err(|e| {
            (
                Status::InternalServerError,
                format!("Error acquiring hasura transaction {:?}", e),
            )
        })?;

    let filter = ListUsersFilter {
        tenant_id: input.tenant_id.clone(),
        election_event_id: input.election_event_id.clone(),
        election_id: input.election_id.clone(),
        area_id: None,
        realm: realm.clone(),
        search: input.search,
        first_name: input.first_name,
        last_name: input.last_name,
        username: input.username,
        email: input.email,
        limit: input.limit,
        offset: input.offset,
        user_ids: input.user_ids,
        attributes: input.attributes,
        enabled: input.enabled,
        email_verified: input.email_verified,
        sort: input.sort,
        has_voted: input.has_voted,
        authorized_to_election_alias: input.authorized_to_election_alias,
    };

    if input.has_voted.is_some() {
        let (mut users, count) = list_users_has_voted(
            &hasura_transaction,
            &keycloak_transaction,
            filter,
            &input.tenant_id,
        )
        .await
        .map_err(|e| {
            (
                Status::InternalServerError,
                format!("Error listing users that has_voted {:?}", e),
            )
        })?;

        for user in &mut users {
            redact_user(user, &secret_names);
        }
        return Ok(Json(DataList {
            items: users,
            total: TotalAggregate {
                aggregate: Aggregate {
                    count: count as i64,
                },
            },
        }));
    }

    let (mut users, count) = match input.show_votes_info.unwrap_or(false) {
        true =>
        // If show_vote_info is true, call list_users_with_vote_info()
        {
            list_users_with_vote_info(
                &hasura_transaction,
                &keycloak_transaction,
                filter,
            )
            .await
            .map_err(|e| {
                (
                    Status::InternalServerError,
                    format!("Error listing users with vote info {:?}", e),
                )
            })?
        }
        // If show_vote_info is false, call list_users() and return empty
        // votes_info
        false => list_users(&hasura_transaction, &keycloak_transaction, filter)
            .await
            .map_err(|e| {
                (
                    Status::InternalServerError,
                    format!("Error listing users {:?}", e),
                )
            })?,
    };

    for user in &mut users {
        redact_user(user, &secret_names);
    }
    Ok(Json(DataList {
        items: users,
        total: TotalAggregate {
            aggregate: Aggregate {
                count: count as i64,
            },
        },
    }))
}

/// Names a refused attribute and the constraint it broke, for logs and for any
/// consumer that does not read the structured extensions. The constraint's
/// arguments are left to those extensions, which the admin portal renders in
/// the admin's own language.
fn describe_user_profile_validation(
    validation: &UserProfileValidationError,
) -> String {
    let field = validation.field.as_deref().unwrap_or("unknown attribute");
    let reason = validation
        .error_message
        .as_deref()
        .unwrap_or("invalid value");

    format!("Invalid value for \"{field}\": {reason}")
}

/// How many refused attributes are reported at once. Keycloak reports every
/// one it refused, and a mis-mapped import can refuse most of a profile, which
/// is more than an error message can usefully carry.
const MAX_REPORTED_USER_PROFILE_ERRORS: usize = 10;

/// Client error naming the attributes Keycloak refused, listing at most
/// MAX_REPORTED_USER_PROFILE_ERRORS of them and saying how many were left out.
fn user_profile_error(validations: &[UserProfileValidationError]) -> JsonError {
    let reported: Vec<UserProfileValidationError> = validations
        .iter()
        .take(MAX_REPORTED_USER_PROFILE_ERRORS)
        .cloned()
        .collect();
    let mut message = reported
        .iter()
        .map(describe_user_profile_validation)
        .collect::<Vec<String>>()
        .join("; ");
    let unreported = validations.len() - reported.len();
    if unreported > 0 {
        message.push_str(&format!(" (and {unreported} more)"));
    }

    ErrorResponse::user_profile_validation(
        Status::BadRequest,
        &message,
        &reported,
        validations.len(),
    )
}

/// Turn a refused Keycloak write into a client error naming the attributes it
/// refused, and into an internal error when Keycloak did not say which.
fn keycloak_user_error(error: anyhow::Error, context: &str) -> JsonError {
    let validations = get_user_profile_validation_errors(&error);
    if validations.is_empty() {
        return ErrorResponse::new(
            Status::InternalServerError,
            &format!("{context}: {error:?}"),
            ErrorCode::InternalServerError,
        );
    }

    user_profile_error(&validations)
}

#[derive(Deserialize, Debug)]
pub struct CreateUserBody {
    tenant_id: String,
    election_event_id: Option<String>,
    user: User,
    user_roles_ids: Option<Vec<String>>,
    #[serde(default)]
    secret_attributes: Option<HashMap<String, Option<Vec<String>>>>,
}

#[instrument(skip(claims, body))]
#[post("/create-user", format = "json", data = "<body>")]
pub async fn create_user(
    claims: jwt::JwtClaims,
    body: Json<CreateUserBody>,
) -> Result<Json<User>, JsonError> {
    let input = body.into_inner();
    let mut required_perms = Vec::<Permissions>::new();
    if input.election_event_id.is_some() {
        required_perms.push(Permissions::VOTER_CREATE);
        if input
            .secret_attributes
            .as_ref()
            .is_some_and(|attributes| !attributes.is_empty())
        {
            required_perms.push(Permissions::VOTER_SECRET_ATTRIBUTE_WRITE);
        }
    } else {
        if input
            .secret_attributes
            .as_ref()
            .is_some_and(|attributes| !attributes.is_empty())
        {
            return Err(ErrorResponse::new(
                Status::BadRequest,
                "Encrypted attributes are only supported for election-event voters",
                ErrorCode::UnknownError,
            ));
        }
        required_perms.push(Permissions::USER_CREATE);
        if let Some(attributes) = &input.user.attributes {
            if attributes.contains_key(PERMISSION_LABELS) {
                // only user who has this permission can edit the user
                // permission_labels if it present in the body.
                required_perms.push(Permissions::PERMISSION_LABEL_WRITE);
            }
        }
    };
    authorize(&claims, true, Some(input.tenant_id.clone()), required_perms)
        .map_err(|(status, message)| {
            let code = if status == Status::InternalServerError {
                ErrorCode::InternalServerError
            } else {
                ErrorCode::Unauthorized
            };
            ErrorResponse::new(status, &message, code)
        })?;
    let realm = match input.election_event_id.clone() {
        Some(election_event_id) => {
            get_event_realm(&input.tenant_id, &election_event_id)
        }
        None => get_tenant_realm(&input.tenant_id),
    };
    let client = KeycloakAdminClient::new().await.map_err(|error| {
        ErrorResponse::new(
            Status::InternalServerError,
            &format!("Error connecting to Keycloak: {error:?}"),
            ErrorCode::InternalServerError,
        )
    })?;
    let secret_names = if input.election_event_id.is_some() {
        let profile_attributes = client
            .get_user_profile_attributes(&realm)
            .await
            .map_err(|error| {
                ErrorResponse::new(
                    Status::InternalServerError,
                    &format!(
                        "Error reading the Keycloak user profile: {error:?}"
                    ),
                    ErrorCode::InternalServerError,
                )
            })?;
        secret_attribute_names(&profile_attributes).map_err(|error| {
            ErrorResponse::new(
                Status::BadRequest,
                &error.to_string(),
                ErrorCode::UnknownError,
            )
        })?
    } else {
        HashSet::new()
    };
    if let Some(name) = input.user.attributes.as_ref().and_then(|attributes| {
        attributes.keys().find(|name| secret_names.contains(*name))
    }) {
        return Err(ErrorResponse::new(
            Status::BadRequest,
            &format!(
                "Encrypted voter attribute `{name}` must be supplied through secret_attributes"
            ),
            ErrorCode::UnknownError,
        ));
    }
    let (tenant_id_attribute, groups) = if input.election_event_id.is_some() {
        let voter_group_name =
            env::var("KEYCLOAK_VOTER_GROUP_NAME").map_err(|error| {
                ErrorResponse::new(
                    Status::InternalServerError,
                    &format!("Error reading voter group name: {error:?}"),
                    ErrorCode::InternalServerError,
                )
            })?;
        (
            Some(HashMap::from([(
                TENANT_ID_ATTR_NAME.to_string(),
                vec![input.tenant_id.clone()],
            )])),
            Some(vec![voter_group_name]),
        )
    } else {
        (
            Some(HashMap::from([(
                TENANT_ID_ATTR_NAME.to_string(),
                vec![input.tenant_id.clone()],
            )])),
            None,
        )
    };

    let user_attributes =
        match (&tenant_id_attribute, input.user.attributes.clone()) {
            (Some(tenant_id_attribute), Some(user_attributes)) => {
                let mut attributes = tenant_id_attribute.clone();
                for (key, mut values) in user_attributes {
                    attributes
                        .entry(key.clone())
                        .or_insert_with(Vec::new)
                        .append(&mut values);
                }
                Some(attributes)
            }
            (Some(tenant_id_attribute), None) => {
                Some(tenant_id_attribute.clone())
            }
            (None, Some(user_attributes)) => Some(user_attributes.clone()),
            (None, None) => None,
        };
    let mut user = input.user.clone();
    user.email_verified = Some(true);
    let has_secret_attributes = input
        .secret_attributes
        .as_ref()
        .is_some_and(|attributes| !attributes.is_empty());
    let requested_enabled = user.enabled.unwrap_or(true);
    if has_secret_attributes {
        // Do not expose a voter whose secret attributes have not been stored
        // yet. If the second Keycloak write fails, the incomplete voter stays
        // disabled and cannot authenticate.
        user.enabled = Some(false);
    }

    let mut user = client
        .create_user(&realm, &user, user_attributes, groups)
        .await
        .map_err(|error| {
            keycloak_user_error(error, "Error creating user in Keycloak")
        })?;

    match (user.id.clone(), &input.user_roles_ids) {
        (Some(id), Some(user_roles_ids)) => {
            let res: Vec<_> = user_roles_ids
                .into_iter()
                .map(|role_id| client.set_user_role(&realm, &id, &role_id))
                .collect();

            join_all(res).await;
        }
        _ => (),
    };

    if let Some(secret_attributes) = input.secret_attributes {
        if !secret_attributes.is_empty() {
            let user_id = user.id.as_deref().ok_or_else(|| {
                ErrorResponse::new(
                    Status::InternalServerError,
                    "Keycloak created the voter without returning its id",
                    ErrorCode::InternalServerError,
                )
            })?;
            let election_event_id =
                input.election_event_id.as_deref().ok_or_else(|| {
                    ErrorResponse::new(
                        Status::BadRequest,
                        "Encrypted attributes require an election event",
                        ErrorCode::UnknownError,
                    )
                })?;
            let encrypted = encrypt_secret_attribute_map(
                &input.tenant_id,
                election_event_id,
                user_id,
                &secret_names,
                secret_attributes,
            )
            .await
            .map_err(|error| {
                ErrorResponse::new(
                    Status::BadRequest,
                    &error.to_string(),
                    ErrorCode::UnknownError,
                )
            })?;
            user = KeycloakAdminClient::new()
                .await
                .map_err(|error| {
                    ErrorResponse::new(
                        Status::InternalServerError,
                        &format!("Error connecting to Keycloak: {error:?}"),
                        ErrorCode::InternalServerError,
                    )
                })?
                .edit_user(
                    &realm,
                    user_id,
                    Some(requested_enabled),
                    Some(encrypted),
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                )
                .await
                .map_err(|error| {
                    keycloak_user_error(
                        error,
                        "Voter was created, but its encrypted attributes could not be stored",
                    )
                })?;
        }
    }

    redact_user(&mut user, &secret_names);

    Ok(Json(user))
}

#[derive(Deserialize, Debug)]
pub struct EditUserBody {
    tenant_id: String,
    user_id: String,
    enabled: Option<bool>,
    election_event_id: Option<String>,
    attributes: Option<HashMap<String, Vec<String>>>,
    #[serde(default)]
    secret_attributes: Option<HashMap<String, Option<Vec<String>>>>,
    email: Option<String>,
    first_name: Option<String>,
    last_name: Option<String>,
    username: Option<String>,
    password: Option<String>,
    temporary: Option<bool>,
}

const MOBILE_NUMBER_ATTRIBUTE: &str = "sequent.read-only.mobile-number";

pub struct EditUserError(JsonError);

impl std::fmt::Debug for EditUserError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("EditUserError")
            .field("status", &self.0 .0)
            .field("code", &self.0 .1 .0.extensions.code)
            .finish()
    }
}

impl EditUserError {
    fn new(status: Status, message: &str, code: ErrorCode) -> Self {
        Self(ErrorResponse::new(status, message, code))
    }

    fn password_policy_violation() -> Self {
        Self::new(
            Status::BadRequest,
            "The password does not comply with the election event password policy",
            ErrorCode::PasswordPolicyViolation,
        )
    }

    fn password_policy_violation_with_details(
        violation: &PasswordPolicyViolation,
    ) -> Self {
        Self(ErrorResponse::password_policy_violation(
            Status::BadRequest,
            &violation.to_string(),
            violation.rule.as_str(),
            violation.required_count,
        ))
    }

    fn from_keycloak(error: anyhow::Error, context: &str) -> Self {
        Self(keycloak_user_error(error, context))
    }
}

impl From<(Status, String)> for EditUserError {
    fn from((status, message): (Status, String)) -> Self {
        let code =
            if status == Status::Unauthorized || status == Status::Forbidden {
                ErrorCode::Unauthorized
            } else if status == Status::InternalServerError {
                ErrorCode::InternalServerError
            } else {
                ErrorCode::UnknownError
            };
        Self::new(status, &message, code)
    }
}

impl<'r> Responder<'r, 'static> for EditUserError {
    fn respond_to(self, request: &'r Request<'_>) -> ResponseResult<'static> {
        self.0.respond_to(request)
    }
}

pub async fn check_edit_email_tlf(
    client: &KeycloakAdminClient,
    input: &EditUserBody,
    realm: &str,
    attributes: &HashMap<String, Vec<String>>,
) -> Result<()> {
    let user = client.get_user(realm, &input.user_id).await?;
    let mut changes: Vec<String> = vec![];

    let mut current_attributes = user.attributes.unwrap_or_default();
    current_attributes.remove(MOBILE_NUMBER_ATTRIBUTE);
    let mut new_attributes = attributes.clone();
    new_attributes.remove(MOBILE_NUMBER_ATTRIBUTE);
    if current_attributes != new_attributes {
        changes.push("attributes".to_string());
    }

    if input.enabled != user.enabled {
        changes.push("enabled".to_string());
    }
    if input.first_name != user.first_name {
        changes.push("first_name".to_string());
    }
    if input.last_name != user.last_name {
        changes.push("last_name".to_string());
    }
    if input.username != user.username {
        changes.push("username".to_string());
    }
    if input.password.is_some() {
        changes.push("password".to_string());
    }
    if input.temporary.is_some() {
        changes.push("temporary".to_string());
    }

    if changes.len() > 0 {
        return Err(anyhow!("Can't change user properties: {:?}", changes));
    }

    Ok(())
}

#[instrument(skip(claims, body), ret)]
#[post("/edit-user", format = "json", data = "<body>")]
pub async fn edit_user(
    claims: jwt::JwtClaims,
    body: Json<EditUserBody>,
) -> Result<Json<EditUserOutput>, EditUserError> {
    let input = body.into_inner();
    let password_only = input.election_event_id.is_some()
        && input.password.is_some()
        && input.enabled.is_none()
        && input.attributes.is_none()
        && input.secret_attributes.is_none()
        && input.email.is_none()
        && input.first_name.is_none()
        && input.last_name.is_none()
        && input.username.is_none();
    let mut required_perms = Vec::<Permissions>::new();
    let has_secret_changes = input
        .secret_attributes
        .as_ref()
        .is_some_and(|attributes| !attributes.is_empty());
    let mut voter_voted_edit = false;
    let mut voter_email_tlf_edit = false;
    if input.election_event_id.is_some() {
        if password_only {
            required_perms.push(Permissions::VOTER_CHANGE_PASSWORD);
        }
        voter_voted_edit = claims
            .hasura_claims
            .allowed_roles
            .contains(&Permissions::VOTER_VOTED_EDIT.to_string());
        voter_email_tlf_edit = claims
            .hasura_claims
            .allowed_roles
            .contains(&Permissions::VOTER_EMAIL_TLF_EDIT.to_string());
        let voter_write = claims
            .hasura_claims
            .allowed_roles
            .contains(&Permissions::VOTER_WRITE.to_string());

        if !password_only {
            if voter_write {
                required_perms.push(Permissions::VOTER_WRITE);
            } else {
                required_perms.push(Permissions::VOTER_EMAIL_TLF_EDIT);
            }
            if input.password.is_some() {
                required_perms.push(Permissions::VOTER_CHANGE_PASSWORD);
            }
            if has_secret_changes {
                required_perms.push(Permissions::VOTER_SECRET_ATTRIBUTE_WRITE);
                if !voter_write {
                    required_perms.push(Permissions::VOTER_WRITE);
                }
            }
        }
    } else {
        if has_secret_changes {
            return Err((
                Status::BadRequest,
                "Encrypted attributes are only supported for election-event voters".to_string(),
            )
                .into());
        }
        required_perms.push(Permissions::USER_WRITE);
        if let Some(attributes) = &input.attributes {
            if attributes.contains_key(PERMISSION_LABELS) {
                // only user who has this permission can edit the user
                // permission_labels if it present in the body.
                required_perms.push(Permissions::PERMISSION_LABEL_WRITE);
            }
        }
    };

    authorize(&claims, true, Some(input.tenant_id.clone()), required_perms)?;
    let realm = match input.election_event_id.clone() {
        Some(election_event_id) => {
            get_event_realm(&input.tenant_id, &election_event_id)
        }
        None => get_tenant_realm(&input.tenant_id),
    };
    let secret_names = if let Some(election_event_id) =
        input.election_event_id.as_deref()
    {
        get_event_secret_attribute_names(&input.tenant_id, election_event_id)
            .await?
    } else {
        HashSet::new()
    };
    if let Some(name) = input.attributes.as_ref().and_then(|attributes| {
        attributes.keys().find(|name| secret_names.contains(*name))
    }) {
        return Err((
            Status::BadRequest,
            format!(
                "Encrypted voter attribute `{name}` must be supplied through secret_attributes"
            ),
        )
            .into());
    }

    if let (Some(election_event_id), Some(password)) = (
        input.election_event_id.as_deref(),
        input.password.as_deref(),
    ) {
        let password_policy =
            get_realm_password_policy(&input.tenant_id, election_event_id)
                .await
                .map_err(|error| {
                    (
                        Status::InternalServerError,
                        format!(
                    "Failed to read election event Password Policy: {error:#}"
                ),
                    )
                })?;
        password_policy
            .validate_password(password)
            .map_err(|violation| {
                EditUserError::password_policy_violation_with_details(
                    &violation,
                )
            })?;
    }

    let mut hasura_db_client: DbClient =
        get_hasura_pool().await.get().await.map_err(|e| {
            (
                Status::InternalServerError,
                format!("Error acquiring hasura db client from pool {:?}", e),
            )
        })?;

    let hasura_transaction =
        hasura_db_client.transaction().await.map_err(|e| {
            (
                Status::InternalServerError,
                format!("Error acquiring hasura transaction {:?}", e),
            )
        })?;

    // check if the voter has voted
    if !voter_voted_edit {
        if let Some(election_event_id) = input.election_event_id.clone() {
            let mut user = User::default();
            user.id = Some(input.user_id.clone());
            let voters = get_users_with_vote_info(
                &hasura_transaction,
                &input.tenant_id,
                &election_event_id,
                None,
                vec![user],
                None, // filter_by_has_voted
            )
            .await
            .map_err(|e| {
                (
                    Status::InternalServerError,
                    format!("Error listing users with vote info {:?}", e),
                )
            })?;
            let Some(voter) = voters.first() else {
                return Err((
                    Status::InternalServerError,
                    format!("Error listing voter with vote info"),
                )
                    .into());
            };
            if let Some(votes_info) = voter.votes_info.clone() {
                if votes_info.len() > 0 {
                    return Err((
                        Status::Unauthorized,
                        format!("Can't edit a voter that has already cast its ballot"),
                    )
                        .into());
                }
            }
        }
    }

    let mut new_attributes = input.attributes.clone().unwrap_or(HashMap::new());
    if let Some(secret_attributes) = input.secret_attributes.clone() {
        if !secret_attributes.is_empty() {
            let election_event_id =
                input.election_event_id.as_deref().ok_or_else(|| {
                    EditUserError::from((
                        Status::BadRequest,
                        "Encrypted attributes require an election event"
                            .to_string(),
                    ))
                })?;
            let encrypted = encrypt_secret_attribute_map(
                &input.tenant_id,
                election_event_id,
                &input.user_id,
                &secret_names,
                secret_attributes,
            )
            .await
            .map_err(|error| {
                EditUserError::from((Status::BadRequest, error.to_string()))
            })?;
            new_attributes.extend(encrypted);
        }
    }

    // maintain current user attributes and do not allow to override tenant-id
    if new_attributes.contains_key(TENANT_ID_ATTR_NAME) {
        return Err((
            Status::BadRequest,
            "Cannot change tenant-id attribute".to_string(),
        )
            .into());
    }

    if voter_email_tlf_edit {
        /*check_edit_email_tlf(&client, &input, &realm, &new_attributes)
        .await
        .map_err(|e| (Status::Unauthorized, format!("{:?}", e)))?;*/
    }

    let datafix_election_event = match input.election_event_id.as_deref() {
        Some(election_event_id) => {
            let election_event = get_election_event_by_id(
                &hasura_transaction,
                &input.tenant_id,
                election_event_id,
            )
            .await
            .map_err(|err| {
                (
                    Status::InternalServerError,
                    format!("Error getting election event: {err:?}"),
                )
            })?;
            datafix_annotations(&election_event)
                .map_err(|err| (Status::InternalServerError, err.to_string()))?
                .map(|_| election_event)
        }
        None => None,
    };

    hasura_transaction.commit().await.map_err(|err| {
        (
            Status::InternalServerError,
            format!("Error committing voter checks: {err:?}"),
        )
    })?;
    drop(hasura_db_client);

    // For Datafix election events the edit is offloaded to the `edit_user`
    // task, which notifies VoterView (SetNotVoted) and reconciles the voter's
    // cast votes under the per-voter lock. Deferring it keeps the Save button
    // from blocking on the (retried) VoterView round-trip, and the admin portal
    // tracks the outcome in the returned task widget. Non-Datafix edits stay
    // synchronous.
    if !password_only {
        if let (Some(election_event_id), Some(_election_event)) =
            (input.election_event_id.as_deref(), datafix_election_event)
        {
            let executer_name = claims
                .name
                .clone()
                .unwrap_or_else(|| claims.hasura_claims.user_id.clone());

            let task_execution = post(
                &input.tenant_id,
                Some(election_event_id),
                ETasksExecution::EDIT_USER,
                &executer_name,
            )
            .await
            .map_err(|error| {
                (
                    Status::InternalServerError,
                    format!(
                        "Failed to insert task execution record: {error:?}"
                    ),
                )
            })?;

            let task_body = EditUserTaskBody {
                tenant_id: input.tenant_id.clone(),
                user_id: input.user_id.clone(),
                election_event_id: election_event_id.to_string(),
                enabled: input.enabled,
                attributes: new_attributes,
                email: input.email.clone(),
                first_name: input.first_name.clone(),
                last_name: input.last_name.clone(),
                username: input.username.clone(),
                password: input.password.clone(),
                temporary: input.temporary,
                password_change_initiator: input.password.as_ref().map(|_| {
                    ElectoralLogAdminContext {
                        user_id: claims.hasura_claims.user_id.clone(),
                        username: claims.preferred_username.clone(),
                        authorized_election_ids: claims
                            .hasura_claims
                            .authorized_election_ids
                            .clone(),
                        area_id: claims.hasura_claims.area_id.clone(),
                    }
                }),
            };

            let celery_app = get_celery_app().await;
            if let Err(err) = celery_app
                .send_task(windmill::tasks::edit_user::edit_user::new(
                    task_body,
                    task_execution.clone(),
                ))
                .await
            {
                update_fail(
                    &task_execution,
                    &format!("Failed to send Edit Voter task: {err:?}"),
                )
                .await
                .ok();
                return Err((
                    Status::InternalServerError,
                    format!("Error sending Edit Voter task: {err:?}"),
                )
                    .into());
            }

            info!("Sent EDIT_USER task {}", task_execution.id);

            return Ok(Json(EditUserOutput {
                user: None,
                task_execution: Some(task_execution),
            }));
        }
    }

    let client = KeycloakAdminClient::new()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    let mut user = client
        .edit_user(
            &realm,
            &input.user_id,
            input.enabled,
            Some(new_attributes),
            input.email.clone(),
            input.first_name.clone(),
            input.last_name.clone(),
            input.username.clone(),
            input.password.clone(),
            input.temporary,
        )
        .await
        .map_err(|error| {
            if password_only && is_keycloak_bad_request(&error) {
                EditUserError::password_policy_violation()
            } else {
                EditUserError::from_keycloak(
                    error,
                    "Error editing user in Keycloak",
                )
            }
        })?;

    if let (Some(election_event_id), Some(_)) =
        (input.election_event_id.as_deref(), input.password.as_ref())
    {
        let admin = ElectoralLogAdminContext {
            user_id: claims.hasura_claims.user_id.clone(),
            username: claims.preferred_username.clone(),
            authorized_election_ids: claims
                .hasura_claims
                .authorized_election_ids
                .clone(),
            area_id: claims.hasura_claims.area_id.clone(),
        };
        post_voter_password_change(
            &input.tenant_id,
            election_event_id,
            &input.user_id,
            user.username.clone(),
            &admin,
            VoterPasswordChangeSource::AdminPortal,
        )
        .await
        .map_err(|error| -> EditUserError {
            (
                Status::InternalServerError,
                format!("Voter password changed, but its electoral-log entry failed: {error:#}"),
            )
                .into()
        })?;
    }

    redact_user(&mut user, &secret_names);
    Ok(Json(EditUserOutput {
        user: Some(user),
        task_execution: None,
    }))
}

#[derive(Deserialize, Debug)]
pub struct GetUserBody {
    tenant_id: String,
    election_event_id: Option<String>,
    user_id: String,
}

#[instrument(skip(claims))]
#[post("/get-user", format = "json", data = "<body>")]
pub async fn get_user(
    claims: jwt::JwtClaims,
    body: Json<GetUserBody>,
) -> Result<Json<User>, (Status, String)> {
    let input = body.into_inner();
    let required_perm: Permissions = if input.election_event_id.is_some() {
        Permissions::VOTER_READ
    } else {
        Permissions::USER_READ
    };
    authorize(
        &claims,
        true,
        Some(input.tenant_id.clone()),
        vec![required_perm],
    )?;
    let realm = match input.election_event_id.as_ref() {
        Some(election_event_id) => {
            get_event_realm(&input.tenant_id, election_event_id)
        }
        None => get_tenant_realm(&input.tenant_id),
    };
    let client = KeycloakAdminClient::new()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    let mut user = client
        .get_user(&realm, &input.user_id)
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    if let Some(election_event_id) = input.election_event_id.as_deref() {
        let secret_names = get_event_secret_attribute_names(
            &input.tenant_id,
            election_event_id,
        )
        .await?;
        redact_user(&mut user, &secret_names);
    }

    Ok(Json(user))
}

#[derive(Deserialize, Debug)]
pub struct RevealSecretAttributeBody {
    tenant_id: String,
    election_event_id: String,
    user_id: String,
    attribute_name: String,
}

#[derive(Serialize, Debug)]
pub struct RevealSecretAttributeOutput {
    attribute_name: String,
    values: Vec<String>,
}

#[instrument(skip(claims, body))]
#[post("/reveal-voter-secret-attribute", format = "json", data = "<body>")]
pub async fn reveal_voter_secret_attribute(
    claims: jwt::JwtClaims,
    body: Json<RevealSecretAttributeBody>,
) -> Result<Json<RevealSecretAttributeOutput>, (Status, String)> {
    let input = body.into_inner();
    authorize(
        &claims,
        true,
        Some(input.tenant_id.clone()),
        vec![
            Permissions::VOTER_READ,
            Permissions::VOTER_SECRET_ATTRIBUTE_READ,
        ],
    )?;
    let secret_names = get_event_secret_attribute_names(
        &input.tenant_id,
        &input.election_event_id,
    )
    .await?;
    if !secret_names.contains(&input.attribute_name) {
        return Err((
            Status::BadRequest,
            format!(
                "User-profile attribute `{}` is not configured as encrypted",
                input.attribute_name
            ),
        ));
    }

    let realm = get_event_realm(&input.tenant_id, &input.election_event_id);
    let user = KeycloakAdminClient::new()
        .await
        .map_err(|error| {
            (
                Status::InternalServerError,
                format!("Error connecting to Keycloak: {error:?}"),
            )
        })?
        .get_user(&realm, &input.user_id)
        .await
        .map_err(|error| {
            (
                Status::InternalServerError,
                format!("Error reading voter from Keycloak: {error:?}"),
            )
        })?;
    let encrypted_values = user_attribute_values(&user, &input.attribute_name);
    let values = decrypt_attribute_values(
        &input.tenant_id,
        &input.election_event_id,
        &input.user_id,
        &input.attribute_name,
        &encrypted_values,
    )
    .await
    .map_err(|error| {
        (
            Status::InternalServerError,
            format!("Error decrypting voter attribute: {error:#}"),
        )
    })?;

    Ok(Json(RevealSecretAttributeOutput {
        attribute_name: input.attribute_name,
        values,
    }))
}

#[instrument(skip(claims))]
#[post("/import-users", format = "json", data = "<body>")]
pub async fn import_users_f(
    claims: jwt::JwtClaims,
    body: Json<import_users::ImportUsersBody>,
) -> Result<Json<ImportUsersOutput>, (Status, String)> {
    let input = body.clone().into_inner();
    let tenant_id = claims.hasura_claims.tenant_id.clone();
    let election_event_id = input.election_event_id.clone().unwrap_or_default();
    let is_admin = election_event_id.is_empty();
    info!("Calculated is_admin: {}", is_admin);

    let executer_name = claims
        .name
        .clone()
        .unwrap_or_else(|| claims.hasura_claims.user_id.clone());
    let required_perm: Permissions = if input.election_event_id.is_some() {
        Permissions::VOTER_CREATE
    } else {
        Permissions::USER_CREATE
    };

    // Insert the task execution record
    let task_execution = post(
        &tenant_id,
        Some(&election_event_id),
        ETasksExecution::IMPORT_USERS,
        &executer_name,
    )
    .await
    .map_err(|error| {
        (
            Status::InternalServerError,
            format!("Failed to insert task execution record: {error:?}"),
        )
    })?;

    authorize(
        &claims,
        true,
        Some(input.tenant_id.clone()),
        vec![required_perm],
    )?;
    let celery_app = get_celery_app().await;

    let mut task_input = input.clone();
    task_input.is_admin = is_admin;
    task_input.may_write_secret_attributes = input.election_event_id.is_some()
        && authorize(
            &claims,
            true,
            Some(input.tenant_id.clone()),
            vec![Permissions::VOTER_SECRET_ATTRIBUTE_WRITE],
        )
        .is_ok();

    let _celery_task = match celery_app
        .send_task(import_users::import_users::new(
            task_input,
            task_execution.clone(),
        ))
        .await
    {
        Ok(celery_task) => celery_task,
        Err(_) => {
            return Ok(Json(ImportUsersOutput {
                task_execution: task_execution.clone(),
            }));
        }
    };

    info!("Sent IMPORT_USERS task {}", task_execution.id);

    let output = ImportUsersOutput {
        task_execution: task_execution.clone(),
    };

    Ok(Json(output))
}

#[instrument(skip(claims, input))]
#[post("/export-users", format = "json", data = "<input>")]
pub async fn export_users_f(
    claims: jwt::JwtClaims,
    input: Json<ExportUsersBody>,
) -> Result<Json<ExportUsersOutput>, (Status, String)> {
    let body = input.into_inner();
    let tenant_id = body.tenant_id.clone();
    let executer_name = claims
        .name
        .clone()
        .unwrap_or_else(|| claims.hasura_claims.user_id.clone());

    let required_perm = if body.election_event_id.clone().is_some() {
        Permissions::VOTER_READ
    } else {
        Permissions::USER_READ
    };

    authorize(
        &claims,
        true,
        Some(body.tenant_id.clone()),
        vec![required_perm],
    )?;

    let may_read_secret_attributes = if body.include_secret_attributes {
        if body.election_event_id.is_none() {
            return Err((
                Status::BadRequest,
                "Secret attributes can only be included in an election-event voter export"
                    .to_string(),
            ));
        }
        authorize(
            &claims,
            true,
            Some(body.tenant_id.clone()),
            vec![Permissions::VOTER_SECRET_ATTRIBUTE_READ],
        )?;
        true
    } else {
        false
    };

    let document_id = Uuid::new_v4().to_string();

    // Authorize before creating the task row, then persist a task-bound grant.
    // The worker reloads this row and never trusts a broker-supplied boolean.
    let task_execution =
        if let Some(ref election_event_id) = body.election_event_id {
            Some(
                post_with_annotations(
                    &tenant_id,
                    Some(election_event_id),
                    ETasksExecution::EXPORT_VOTERS,
                    &executer_name,
                    secret_export_task_annotations(
                        &document_id,
                        may_read_secret_attributes,
                    ),
                )
                .await
                .map_err(|error| {
                    (
                        Status::InternalServerError,
                        format!(
                            "Failed to insert task execution record: {error:?}"
                        ),
                    )
                })?,
            )
        } else {
            None
        };

    let celery_app = get_celery_app().await;

    let celery_task = match celery_app
        .send_task(export_users::export_users::new(
            ExportBody::Users {
                tenant_id: body.tenant_id,
                election_event_id: body.election_event_id.clone(),
                election_id: body.election_id,
                include_secret_attributes: body.include_secret_attributes,
            },
            document_id.clone(),
            task_execution.clone(),
        ))
        .await
    {
        Ok(celery_task) => celery_task,
        Err(err) => {
            if let Some(task_execution) = &task_execution {
                update_fail(
                    task_execution,
                    &format!("Failed to enqueue voter export: {err:?}"),
                )
                .await
                .map_err(|update_error| {
                    (
                        Status::InternalServerError,
                        format!(
                            "Failed to revoke voter export authorization: {update_error:?}"
                        ),
                    )
                })?;
            }
            return Ok(Json(ExportUsersOutput {
                document_id,
                error_msg: Some(format!(
                    "Error sending Export Users task: ${err}"
                )),
                task_execution: task_execution.clone(),
            }));
        }
    };

    let output = ExportUsersOutput {
        document_id,
        error_msg: None,
        task_execution: task_execution.clone(),
    };

    info!("Sent EXPORT_USERS task");

    Ok(Json(output))
}

#[instrument(skip(claims))]
#[post("/export-tenant-users", format = "json", data = "<input>")]
pub async fn export_tenant_users_f(
    claims: jwt::JwtClaims,
    input: Json<ExportTenantUsersBody>,
) -> Result<Json<export_users::ExportUsersOutput>, (Status, String)> {
    let body = input.into_inner();
    let required_perm = Permissions::USER_READ;

    authorize(
        &claims,
        true,
        Some(body.tenant_id.clone()),
        vec![Permissions::USER_READ],
    )?;
    let document_id = Uuid::new_v4().to_string();
    let celery_app = get_celery_app().await;
    let celery_task = match celery_app
        .send_task(export_users::export_users::new(
            ExportBody::TenantUsers {
                tenant_id: body.tenant_id,
            },
            document_id.clone(),
            None,
        ))
        .await
    {
        Ok(celery_task) => celery_task,
        Err(err) => {
            return Ok(Json(ExportUsersOutput {
                document_id,
                error_msg: Some(format!(
                    "Error sending Export Users task: ${err}"
                )),
                task_execution: None,
            }));
        }
    };

    let output = export_users::ExportUsersOutput {
        document_id: document_id,
        error_msg: None,
        task_execution: None,
    };
    info!("Sent EXPORT_TENANT_USERS task {}", celery_task.task_id);

    Ok(Json(output))
}

#[derive(Deserialize, Debug)]
pub struct GetUserProfileAttributesBody {
    tenant_id: String,
    election_event_id: Option<String>,
}

#[instrument(skip(claims))]
#[post("/get-user-profile-attributes", format = "json", data = "<body>")]
pub async fn get_user_profile_attributes(
    claims: jwt::JwtClaims,
    body: Json<GetUserProfileAttributesBody>,
) -> Result<Json<Vec<UserProfileAttribute>>, (Status, String)> {
    let required_perm = if body.election_event_id.is_some() {
        Permissions::VOTER_READ
    } else {
        Permissions::USER_READ
    };

    let input = body.into_inner();
    authorize(
        &claims,
        true,
        Some(input.tenant_id.clone()),
        vec![required_perm],
    )?;

    let realm = match input.election_event_id {
        Some(election_event_id) => {
            get_event_realm(&input.tenant_id, &election_event_id)
        }
        None => get_tenant_realm(&input.tenant_id),
    };

    let client = KeycloakAdminClient::new()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    let attributes_res = client
        .get_user_profile_attributes(&realm)
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    Ok(Json(attributes_res))
}

#[instrument(skip(claims))]
#[post("/get-user-profile-configuration", format = "json", data = "<body>")]
pub async fn get_user_profile_configuration(
    claims: jwt::JwtClaims,
    body: Json<GetUserProfileAttributesBody>,
) -> Result<Json<UserProfileConfiguration>, (Status, String)> {
    let required_perm = if body.election_event_id.is_some() {
        Permissions::VOTER_READ
    } else {
        Permissions::USER_READ
    };

    let input = body.into_inner();
    authorize(
        &claims,
        true,
        Some(input.tenant_id.clone()),
        vec![required_perm],
    )?;

    let realm = match input.election_event_id {
        Some(election_event_id) => {
            get_event_realm(&input.tenant_id, &election_event_id)
        }
        None => get_tenant_realm(&input.tenant_id),
    };

    let client = KeycloakAdminClient::new()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    let configuration = client
        .get_user_profile_configuration(&realm)
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    Ok(Json(configuration))
}

#[cfg(test)]
mod tests {
    use super::{
        user_profile_error, EditUserError, MAX_REPORTED_USER_PROFILE_ERRORS,
    };
    use rocket::http::Status;
    use sequent_core::services::keycloak::{
        PasswordPolicyRule, PasswordPolicyViolation, UserProfileValidationError,
    };

    fn refused(field: &str) -> UserProfileValidationError {
        UserProfileValidationError {
            field: Some(field.to_string()),
            error_message: Some("error-invalid-length".to_string()),
            params: Some(vec![field.into(), 1.into(), 2.into()]),
        }
    }

    #[test]
    fn a_refused_attribute_is_a_structured_bad_request() {
        let response = user_profile_error(&[refused("roll")]);
        let extensions = &response.1 .0.extensions;

        assert_eq!(response.0, Status::BadRequest);
        assert_eq!(extensions.code, "UserProfileValidation");
        assert_eq!(extensions.user_profile_errors_total, Some(1));
        assert!(response.1 .0.message.contains("roll"));
        assert!(response.1 .0.message.contains("error-invalid-length"));
    }

    #[test]
    fn every_refused_attribute_is_reported_in_order() {
        let response = user_profile_error(&[refused("ward"), refused("roll")]);
        let reported = response
            .1
             .0
            .extensions
            .user_profile_errors
            .as_ref()
            .unwrap();

        assert_eq!(reported.len(), 2);
        assert_eq!(reported[0].field.as_deref(), Some("ward"));
        assert_eq!(reported[1].field.as_deref(), Some("roll"));
        let message = &response.1 .0.message;
        assert!(message.find("ward") < message.find("roll"));
    }

    #[test]
    fn a_long_list_of_refused_attributes_is_capped_and_counted() {
        let validations: Vec<UserProfileValidationError> = (0..15)
            .map(|index| refused(&format!("field_{index}")))
            .collect();

        let response = user_profile_error(&validations);
        let extensions = &response.1 .0.extensions;
        let reported = extensions.user_profile_errors.as_ref().unwrap();

        assert_eq!(reported.len(), MAX_REPORTED_USER_PROFILE_ERRORS);
        // The count is of everything refused, not of what was listed.
        assert_eq!(extensions.user_profile_errors_total, Some(15));
        assert!(response.1 .0.message.contains("(and 5 more)"));
        assert!(!response.1 .0.message.contains("field_10"));
    }

    #[test]
    fn a_short_list_does_not_claim_there_are_more() {
        let response = user_profile_error(&[refused("roll")]);

        assert!(!response.1 .0.message.contains("more"));
    }

    #[test]
    fn password_policy_violation_is_a_structured_bad_request() {
        let response = EditUserError::password_policy_violation_with_details(
            &PasswordPolicyViolation {
                rule: PasswordPolicyRule::Digits,
                required_count: 3,
            },
        );

        assert_eq!(response.0 .0, Status::BadRequest);
        assert_eq!(response.0 .1 .0.extensions.code, "PasswordPolicyViolation");
        assert_eq!(
            response.0 .1 .0.extensions.password_policy_rule.as_deref(),
            Some("digits")
        );
        assert_eq!(
            response.0 .1 .0.extensions.password_policy_required_count,
            Some(3)
        );
        assert_eq!(
            response.0 .1 .0.message,
            "Password does not contain enough digits"
        );
    }
}
