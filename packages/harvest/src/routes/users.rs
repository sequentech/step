// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use crate::types::optional::OptionalId;
use crate::types::resources::{Aggregate, DataList, TotalAggregate};
use anyhow::{anyhow, Result};
use deadpool_postgres::Client as DbClient;
use rocket::futures::future::join_all;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt;
use sequent_core::services::keycloak::{get_event_realm, get_tenant_realm};
use sequent_core::services::keycloak::{GroupInfo, KeycloakAdminClient};
use sequent_core::types::keycloak::{
    User, UserProfileAttribute, PERMISSION_LABELS, TENANT_ID_ATTR_NAME,
};
use sequent_core::types::permissions::Permissions;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::env;
use tracing::instrument;
use uuid::Uuid;
use windmill::postgres::election_event::{
    get_election_event_by_id, ElectionEventDatafix,
};
use windmill::services::cast_votes::get_users_with_vote_info;
use windmill::services::celery_app::get_celery_app;
use windmill::services::database::{get_hasura_pool, get_keycloak_pool};
use windmill::services::datafix;
use windmill::services::datafix::types::SoapRequest;
use windmill::services::datafix::utils::is_datafix_election_event;
use windmill::services::export::export_users::{
    ExportBody, ExportTenantUsersBody, ExportUsersBody,
};
use windmill::services::keycloak_events::list_keycloak_events_by_type;
use windmill::services::tasks_execution::*;
use windmill::services::users::{
    count_keycloak_users, list_users, list_users_with_vote_info,
};
use windmill::services::users::{FilterOption, ListUsersFilter};
use windmill::tasks::export_users::{self, ExportUsersOutput};
use windmill::tasks::import_users::{self, ImportUsersOutput};
use windmill::types::tasks::ETasksExecution;

#[derive(Deserialize, Debug)]
pub struct DeleteUserBody {
    tenant_id: String,
    election_event_id: Option<String>,
    user_id: String,
}

#[instrument(skip(claims))]
#[post("/delete-user", format = "json", data = "<body>")]
pub async fn delete_user(
    claims: jwt::JwtClaims,
    body: Json<DeleteUserBody>,
) -> Result<Json<OptionalId>, (Status, String)> {
    let input = body.into_inner();
    let required_perm: Permissions = if input.election_event_id.is_some() {
        Permissions::VOTER_WRITE
    } else {
        Permissions::USER_WRITE
    };
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
    users_id: Vec<String>,
}

#[instrument(skip(claims))]
#[post("/delete-users", format = "json", data = "<body>")]
pub async fn delete_users(
    claims: jwt::JwtClaims,
    body: Json<DeleteUsersBody>,
) -> Result<Json<OptionalId>, (Status, String)> {
    let input = body.into_inner();
    let required_perm: Permissions = if input.election_event_id.is_some() {
        Permissions::VOTER_WRITE
    } else {
        Permissions::USER_WRITE
    };
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

    for id in input.users_id {
        client
            .delete_user(&realm, &id)
            .await
            .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    }
    Ok(Json(Default::default()))
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

#[instrument(skip(claims), ret)]
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
        user_ids: None,
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

#[instrument(skip(claims), ret)]
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
        user_ids: None,
        attributes: input.attributes,
        enabled: input.enabled,
        email_verified: input.email_verified,
        sort: input.sort,
        has_voted: input.has_voted,
        authorized_to_election_alias: input.authorized_to_election_alias,
    };

    let (users, count) = match input.show_votes_info.unwrap_or(false) {
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

    Ok(Json(DataList {
        items: users,
        total: TotalAggregate {
            aggregate: Aggregate {
                count: count as i64,
            },
        },
    }))
}

#[derive(Deserialize, Debug)]
pub struct CreateUserBody {
    tenant_id: String,
    election_event_id: Option<String>,
    user: User,
    user_roles_ids: Option<Vec<String>>,
}

#[instrument(skip(claims))]
#[post("/create-user", format = "json", data = "<body>")]
pub async fn create_user(
    claims: jwt::JwtClaims,
    body: Json<CreateUserBody>,
) -> Result<Json<User>, (Status, String)> {
    let input = body.into_inner();
    let mut required_perms = Vec::<Permissions>::new();
    if input.election_event_id.is_some() {
        required_perms.push(Permissions::VOTER_CREATE)
    } else {
        required_perms.push(Permissions::USER_CREATE);
        if let Some(attributes) = &input.user.attributes {
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
    let client = KeycloakAdminClient::new()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    let (tenant_id_attribute, groups) = if input.election_event_id.is_some() {
        let voter_group_name = env::var("KEYCLOAK_VOTER_GROUP_NAME")
            .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
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

    let user = client
        .create_user(&realm, &user, user_attributes, groups)
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

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

    Ok(Json(user))
}

#[derive(Deserialize, Debug)]
pub struct EditUserBody {
    tenant_id: String,
    user_id: String,
    enabled: Option<bool>,
    election_event_id: Option<String>,
    attributes: Option<HashMap<String, Vec<String>>>,
    email: Option<String>,
    first_name: Option<String>,
    last_name: Option<String>,
    username: Option<String>,
    password: Option<String>,
    temporary: Option<bool>,
}

const MOBILE_NUMBER_ATTRIBUTE: &str = "sequent.read-only.mobile-number";

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

#[instrument(skip(claims), ret)]
#[post("/edit-user", format = "json", data = "<body>")]
pub async fn edit_user(
    claims: jwt::JwtClaims,
    body: Json<EditUserBody>,
) -> Result<Json<User>, (Status, String)> {
    let input = body.into_inner();
    let mut required_perms = Vec::<Permissions>::new();
    let mut voter_voted_edit = false;
    let mut voter_email_tlf_edit = false;
    if input.election_event_id.is_some() {
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

        if voter_write {
            required_perms.push(Permissions::VOTER_WRITE);
        } else {
            required_perms.push(Permissions::VOTER_EMAIL_TLF_EDIT);
        }
    } else {
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
                ));
            };
            if let Some(votes_info) = voter.votes_info.clone() {
                if votes_info.len() > 0 {
                    return Err((
                        Status::Unauthorized,
                        format!("Can't edit a voter that has already cast its ballot"),
                    ));
                }
            }
        }
    }

    let client = KeycloakAdminClient::new()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    let new_attributes = input.attributes.clone().unwrap_or(HashMap::new());

    // maintain current user attributes and do not allow to override tenant-id
    if new_attributes.contains_key(TENANT_ID_ATTR_NAME) {
        return Err((
            Status::BadRequest,
            "Cannot change tenant-id attribute".to_string(),
        ));
    }

    if voter_email_tlf_edit {
        /*check_edit_email_tlf(&client, &input, &realm, &new_attributes)
        .await
        .map_err(|e| (Status::Unauthorized, format!("{:?}", e)))?;*/
    }

    let user = client
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
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    // If the user is disabled via EDIT: send a SetNotVoted request to
    // VoterView, it is a Datafix requirement
    match (input.election_event_id, input.enabled) {
        (Some(election_event_id), Some(enabled)) if !enabled => {
            let election_event = get_election_event_by_id(
                &hasura_transaction,
                &input.tenant_id,
                &election_event_id,
            )
            .await
            .map_err(|e| {
                (
                    Status::InternalServerError,
                    format!("Error get_election_event_by_id {e:?}"),
                )
            })?;
            if is_datafix_election_event(&election_event) {
                let res = datafix::voterview_requests::send(
                    SoapRequest::SetNotVoted,
                    ElectionEventDatafix(election_event),
                    &user.username,
                )
                .await;
                // TODO: Post the result in the electoral_log
            }
        }
        _ => {}
    }

    Ok(Json(user))
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
    let realm = match input.election_event_id {
        Some(election_event_id) => {
            get_event_realm(&input.tenant_id, &election_event_id)
        }
        None => get_tenant_realm(&input.tenant_id),
    };
    let client = KeycloakAdminClient::new()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    let user = client
        .get_user(&realm, &input.user_id)
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    Ok(Json(user))
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

#[instrument(skip(claims))]
#[post("/export-users", format = "json", data = "<input>")]
pub async fn export_users_f(
    claims: jwt::JwtClaims,
    input: Json<ExportUsersBody>,
) -> Result<Json<ExportUsersOutput>, (Status, String)> {
    let body = input.into_inner();
    let tenant_id = claims.hasura_claims.tenant_id.clone();
    let executer_name = claims
        .name
        .clone()
        .unwrap_or_else(|| claims.hasura_claims.user_id.clone());

    let required_perm = if body.election_event_id.clone().is_some() {
        Permissions::VOTER_READ
    } else {
        Permissions::USER_READ
    };

    // Create task execution record only if election_event_id is present
    let task_execution =
        if let Some(ref election_event_id) = body.election_event_id {
            Some(
                post(
                    &tenant_id,
                    Some(election_event_id),
                    ETasksExecution::EXPORT_VOTERS,
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
                })?,
            )
        } else {
            None
        };

    authorize(
        &claims,
        true,
        Some(body.tenant_id.clone()),
        vec![required_perm],
    )?;

    let document_id = Uuid::new_v4().to_string();
    let celery_app = get_celery_app().await;

    let celery_task = match celery_app
        .send_task(export_users::export_users::new(
            ExportBody::Users {
                tenant_id: body.tenant_id,
                election_event_id: body.election_event_id.clone(),
                election_id: body.election_id,
            },
            document_id.clone(),
            task_execution.clone(),
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
