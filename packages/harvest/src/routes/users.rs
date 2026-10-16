// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use crate::types::optional::OptionalId;
use crate::types::resources::{Aggregate, DataList, TotalAggregate};
use deadpool_postgres::Client as DbClient;
use rocket::futures::future::join_all;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt;
use sequent_core::services::keycloak::{get_event_realm, get_tenant_realm};
use sequent_core::services::keycloak::{GroupInfo, KeycloakAdminClient};
use sequent_core::types::keycloak::{
    User, UserProfileAttribute, MOBILE_PHONE_ATTR_NAME, PERMISSION_LABELS,
    TENANT_ID_ATTR_NAME,
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
use windmill::services::users::list_users_has_voted;
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

    if input.has_voted.is_some() {
        let (users, count) = list_users_has_voted(
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

        return Ok(Json(DataList {
            items: users,
            total: TotalAggregate {
                aggregate: Aggregate {
                    count: count as i64,
                },
            },
        }));
    }

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

const EMAIL_AND_OR_MOBILE_ATTR_NAME: &str = "emailAndOrMobile";
const EMAIL_TLF_ATTRIBUTES: [&str; 2] =
    [MOBILE_PHONE_ATTR_NAME, EMAIL_AND_OR_MOBILE_ATTR_NAME];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VoterEditScope {
    Full,
    EmailTlfOnly,
}

impl VoterEditScope {
    fn from_allowed_roles(allowed_roles: &[String]) -> Self {
        if allowed_roles.contains(&Permissions::VOTER_WRITE.to_string()) {
            Self::Full
        } else {
            Self::EmailTlfOnly
        }
    }
}

fn same_attribute_values(
    requested: &[String],
    current: Option<&Vec<String>>,
) -> bool {
    let mut requested = requested.to_vec();
    let mut current = current.cloned().unwrap_or_default();
    requested.sort();
    current.sort();
    requested == current
}

/// Fields the request would change besides the email and mobile number.
/// A field counts only when it is supplied and differs from the current
/// value, so a form that sends every field back unchanged passes. A supplied
/// password always counts, because this scope never changes credentials.
fn email_tlf_edit_violations(
    input: &EditUserBody,
    attributes: &HashMap<String, Vec<String>>,
    current: &User,
) -> Vec<String> {
    let mut violations = Vec::new();
    if input.password.is_some() {
        violations.push("password".to_string());
    }
    if input
        .enabled
        .is_some_and(|enabled| current.enabled != Some(enabled))
    {
        violations.push("enabled".to_string());
    }
    for (field, requested, current) in [
        ("first_name", &input.first_name, &current.first_name),
        ("last_name", &input.last_name, &current.last_name),
        ("username", &input.username, &current.username),
    ] {
        if requested.is_some() && requested != current {
            violations.push(field.to_string());
        }
    }
    let current_attributes = current.attributes.as_ref();
    let mut changed_attributes: Vec<String> = attributes
        .iter()
        .filter(|(name, values)| {
            !EMAIL_TLF_ATTRIBUTES.contains(&name.as_str())
                && !same_attribute_values(
                    values,
                    current_attributes.and_then(|current| current.get(*name)),
                )
        })
        .map(|(name, _)| format!("attributes.{name}"))
        .collect();
    changed_attributes.sort();
    violations.extend(changed_attributes);
    violations
}

/// Drops everything but the email and mobile number from a request that
/// passed `email_tlf_edit_violations`, so that fields the caller may not
/// change are not written back from a stale copy of the form.
fn keep_only_email_tlf(
    input: &mut EditUserBody,
    attributes: &mut HashMap<String, Vec<String>>,
) {
    input.enabled = None;
    input.first_name = None;
    input.last_name = None;
    input.username = None;
    attributes.retain(|name, _| EMAIL_TLF_ATTRIBUTES.contains(&name.as_str()));
}

#[instrument(skip(claims), ret)]
#[post("/edit-user", format = "json", data = "<body>")]
pub async fn edit_user(
    claims: jwt::JwtClaims,
    body: Json<EditUserBody>,
) -> Result<Json<User>, (Status, String)> {
    let mut input = body.into_inner();
    let mut required_perms = Vec::<Permissions>::new();
    let mut voter_voted_edit = false;
    let mut edit_scope = VoterEditScope::Full;
    if input.election_event_id.is_some() {
        voter_voted_edit = claims
            .hasura_claims
            .allowed_roles
            .contains(&Permissions::VOTER_VOTED_EDIT.to_string());
        edit_scope = VoterEditScope::from_allowed_roles(
            &claims.hasura_claims.allowed_roles,
        );
        required_perms.push(match edit_scope {
            VoterEditScope::Full => Permissions::VOTER_WRITE,
            VoterEditScope::EmailTlfOnly => Permissions::VOTER_EMAIL_TLF_EDIT,
        });
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

    let mut new_attributes = input.attributes.clone().unwrap_or(HashMap::new());

    // maintain current user attributes and do not allow to override tenant-id
    if new_attributes.contains_key(TENANT_ID_ATTR_NAME) {
        return Err((
            Status::BadRequest,
            "Cannot change tenant-id attribute".to_string(),
        ));
    }

    if edit_scope == VoterEditScope::EmailTlfOnly {
        let current_user = KeycloakAdminClient::new()
            .await
            .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?
            .get_user(&realm, &input.user_id)
            .await
            .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
        let violations =
            email_tlf_edit_violations(&input, &new_attributes, &current_user);
        if !violations.is_empty() {
            return Err((
                Status::Forbidden,
                format!(
                    "Without the {} permission only the email and mobile number can be changed: {}",
                    Permissions::VOTER_WRITE,
                    violations.join(", ")
                ),
            ));
        }
        keep_only_email_tlf(&mut input, &mut new_attributes);
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

#[cfg(test)]
mod tests {
    mod voter_edit_scope {
        use super::super::{
            email_tlf_edit_violations, keep_only_email_tlf, EditUserBody,
            VoterEditScope, EMAIL_AND_OR_MOBILE_ATTR_NAME,
        };
        use sequent_core::types::keycloak::{
            User, AUTHORIZED_ELECTION_IDS_NAME, MOBILE_PHONE_ATTR_NAME,
        };
        use sequent_core::types::permissions::Permissions;
        use serde_json::{json, Value};
        use std::collections::HashMap;

        const AREA_ID: &str = "area-id";

        fn current_attributes() -> HashMap<String, Vec<String>> {
            HashMap::from([
                (AREA_ID.to_string(), vec!["area-1".to_string()]),
                (
                    AUTHORIZED_ELECTION_IDS_NAME.to_string(),
                    vec!["election-1".to_string(), "election-2".to_string()],
                ),
                (
                    MOBILE_PHONE_ATTR_NAME.to_string(),
                    vec!["+34600000001".to_string()],
                ),
                (
                    EMAIL_AND_OR_MOBILE_ATTR_NAME.to_string(),
                    vec!["email".to_string()],
                ),
            ])
        }

        fn current_voter() -> User {
            User {
                id: Some("voter".to_string()),
                attributes: Some(current_attributes()),
                email: Some("voter@example.com".to_string()),
                enabled: Some(true),
                first_name: Some("Ada".to_string()),
                last_name: Some("Lovelace".to_string()),
                username: Some("ada".to_string()),
                ..Default::default()
            }
        }

        fn violations(fields: Value) -> Vec<String> {
            let mut body = json!({
                "tenant_id": "tenant",
                "user_id": "voter",
                "election_event_id": "event",
            });
            if let (Some(body), Some(fields)) =
                (body.as_object_mut(), fields.as_object())
            {
                body.extend(fields.clone());
            }
            let input: EditUserBody =
                serde_json::from_value(body).expect("valid edit body");
            let attributes = input.attributes.clone().unwrap_or_default();
            email_tlf_edit_violations(&input, &attributes, &current_voter())
        }

        fn attributes_with(name: &str, values: &[&str]) -> Value {
            let mut attributes = current_attributes();
            attributes.insert(
                name.to_string(),
                values.iter().map(|value| value.to_string()).collect(),
            );
            json!(attributes)
        }

        #[test]
        fn voter_write_keeps_the_full_edit_scope() {
            let roles = vec![
                "admin-user".to_string(),
                Permissions::VOTER_WRITE.to_string(),
                Permissions::VOTER_EMAIL_TLF_EDIT.to_string(),
            ];
            assert_eq!(
                VoterEditScope::from_allowed_roles(&roles),
                VoterEditScope::Full
            );
        }

        #[test]
        fn email_tlf_edit_without_voter_write_is_limited_to_email_and_mobile() {
            let roles = vec![
                "admin-user".to_string(),
                Permissions::VOTER_EMAIL_TLF_EDIT.to_string(),
            ];
            assert_eq!(
                VoterEditScope::from_allowed_roles(&roles),
                VoterEditScope::EmailTlfOnly
            );
        }

        #[test]
        fn email_tlf_scope_rejects_enabled_change() {
            assert_eq!(violations(json!({"enabled": false})), vec!["enabled"]);
        }

        #[test]
        fn email_tlf_scope_rejects_name_and_username_changes() {
            assert_eq!(
                violations(json!({
                    "first_name": "Grace",
                    "last_name": "Hopper",
                    "username": "grace",
                })),
                vec!["first_name", "last_name", "username"]
            );
        }

        #[test]
        fn email_tlf_scope_rejects_area_change() {
            assert_eq!(
                violations(json!({
                    "attributes": attributes_with(AREA_ID, &["area-2"]),
                })),
                vec!["attributes.area-id"]
            );
        }

        #[test]
        fn email_tlf_scope_rejects_authorized_elections_change() {
            assert_eq!(
                violations(json!({
                    "attributes": attributes_with(
                        AUTHORIZED_ELECTION_IDS_NAME,
                        &["election-1"],
                    ),
                })),
                vec!["attributes.authorized-election-ids"]
            );
        }

        #[test]
        fn email_tlf_scope_rejects_new_attribute() {
            assert_eq!(
                violations(json!({"attributes": {"vote-weight": ["5"]}})),
                vec!["attributes.vote-weight"]
            );
        }

        #[test]
        fn email_tlf_scope_allows_email_and_mobile_changes() {
            let mut attributes = current_attributes();
            attributes.insert(
                MOBILE_PHONE_ATTR_NAME.to_string(),
                vec!["+34600000002".to_string()],
            );
            attributes.insert(
                EMAIL_AND_OR_MOBILE_ATTR_NAME.to_string(),
                vec!["email".to_string(), "mobile".to_string()],
            );
            attributes.insert(
                AUTHORIZED_ELECTION_IDS_NAME.to_string(),
                vec!["election-2".to_string(), "election-1".to_string()],
            );
            assert!(violations(json!({
                "enabled": true,
                "email": "new@example.com",
                "first_name": "Ada",
                "last_name": "Lovelace",
                "temporary": true,
                "attributes": attributes,
            }))
            .is_empty());
        }

        #[test]
        fn email_tlf_scope_forwards_only_email_and_mobile() {
            let body = json!({
                "tenant_id": "tenant",
                "user_id": "voter",
                "election_event_id": "event",
                "enabled": true,
                "email": "new@example.com",
                "first_name": "Ada",
                "last_name": "Lovelace",
                "username": "ada",
                "attributes": current_attributes(),
            });
            let mut input: EditUserBody =
                serde_json::from_value(body).expect("valid edit body");
            let mut attributes = input.attributes.clone().unwrap_or_default();
            keep_only_email_tlf(&mut input, &mut attributes);
            assert_eq!(input.enabled, None);
            assert_eq!(input.first_name, None);
            assert_eq!(input.last_name, None);
            assert_eq!(input.username, None);
            assert_eq!(input.email.as_deref(), Some("new@example.com"));
            let mut names: Vec<&str> =
                attributes.keys().map(String::as_str).collect();
            names.sort();
            assert_eq!(
                names,
                vec![EMAIL_AND_OR_MOBILE_ATTR_NAME, MOBILE_PHONE_ATTR_NAME]
            );
        }

        #[test]
        fn email_tlf_scope_rejects_password_change() {
            assert_eq!(
                violations(json!({"password": "new-secret"})),
                vec!["password"]
            );
        }

        #[test]
        fn email_tlf_scope_ignores_fields_left_out() {
            assert!(violations(json!({})).is_empty());
            assert!(violations(json!({"enabled": null, "attributes": {}}))
                .is_empty());
        }
    }
}
