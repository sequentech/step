// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Changes to roles in Users and Roles. A `sign-<action>` permission decides
//! who can sign the action, so a change to one is logged, as
//! SigningPermissionChanged, in every election event of the tenant with a
//! rule for the action:
//!
//! 1. a transaction takes those events' signing locks;
//! 2. what the role holds is read under the locks, so only real changes are
//!    logged, and their entries are staged;
//! 3. Keycloak makes the change;
//! 4. the entries are committed. If the commit fails, the Keycloak change
//!    is undone where it can be (a permission set or removed, a role
//!    created); a deleted role can't be restored, so that change stays
//!    without its entries.
//!
//! Every Keycloak call is bounded, and so is the wait for the locks, so a
//! hung Keycloak doesn't hold the events' signing for long.

use crate::services::dependencies::HarvestServices;
use deadpool_postgres::{Object, Transaction};
use rocket::http::Status;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::services::keycloak::{
    get_tenant_realm, validate_keycloak_path_segment,
};
use sequent_core::signing::SigningAction;
use sequent_core::types::keycloak::Role;
use sequent_core::types::permissions::Permissions;
use std::fmt::Debug;
use std::future::Future;
use std::time::Duration;
use strum::IntoEnumIterator;
use tracing::error;
use uuid::Uuid;
use windmill::services::signing::log::Actor;
use windmill::services::signing::permissions::{
    lock_permission_change_events, signing_action_of, signing_actions_in,
    stage_permission_changes, LockedEvents, RolePermissionChange,
    SigningPermissionChange,
};

/// How long one Keycloak call may take.
pub const KEYCLOAK_TIMEOUT: Duration = Duration::from_secs(10);

/// How long the transaction waits for an event's signing lock.
const LOCK_TIMEOUT: &str = "SET LOCAL lock_timeout = '10s'";

pub fn server_error(error: impl Debug) -> (Status, String) {
    (Status::InternalServerError, format!("{:?}", error))
}

/// A Keycloak call, bounded by [`KEYCLOAK_TIMEOUT`]. A call that takes
/// longer is a gateway timeout.
pub async fn keycloak<T, E: Debug>(
    call: impl Future<Output = Result<T, E>>,
) -> Result<T, (Status, String)> {
    match tokio::time::timeout(KEYCLOAK_TIMEOUT, call).await {
        Ok(result) => result.map_err(server_error),
        Err(_) => Err((
            Status::GatewayTimeout,
            format!(
                "Keycloak did not answer within {} s",
                KEYCLOAK_TIMEOUT.as_secs()
            ),
        )),
    }
}

fn tenant_uuid(tenant_id: &str) -> Result<Uuid, (Status, String)> {
    Uuid::parse_str(tenant_id).map_err(|error| {
        (Status::BadRequest, format!("Invalid tenant_id: {error}"))
    })
}

/// The realm and role id are interpolated into Keycloak admin paths, so each
/// must be a single path segment.
fn validate_role_path(
    realm: &str,
    role_id: &str,
) -> Result<(), (Status, String)> {
    validate_keycloak_path_segment(realm)
        .and_then(|()| validate_keycloak_path_segment(role_id))
        .map_err(|_| {
            (
                Status::BadRequest,
                "Invalid tenant_id or role_id".to_string(),
            )
        })
}

fn editor(claims: &JwtClaims) -> Actor {
    Actor {
        user_id: claims.hasura_claims.user_id.clone(),
        username: claims
            .preferred_username
            .clone()
            .unwrap_or_else(|| claims.hasura_claims.user_id.clone()),
    }
}

async fn database(
    services: &HarvestServices,
) -> Result<Object, (Status, String)> {
    services
        .databases
        .hasura()
        .await
        .get()
        .await
        .map_err(server_error)
}

/// A transaction holding the signing locks of the events with a rule for
/// one of `actions`.
async fn lock<'a>(
    client: &'a mut Object,
    tenant_id: Uuid,
    actions: &[SigningAction],
) -> Result<(Transaction<'a>, LockedEvents), (Status, String)> {
    let transaction = client.transaction().await.map_err(server_error)?;
    transaction
        .batch_execute(LOCK_TIMEOUT)
        .await
        .map_err(server_error)?;
    let locked =
        lock_permission_change_events(&transaction, tenant_id, actions)
            .await
            .map_err(server_error)?;
    Ok((transaction, locked))
}

/// The names of the realm roles (permissions) a role holds directly.
async fn held_permissions(
    services: &HarvestServices,
    realm: &str,
    role_id: &str,
) -> Result<Vec<String>, (Status, String)> {
    let client = keycloak(services.identity.client()).await?;
    let held = keycloak(
        client
            .client
            .realm_groups_with_group_id_role_mappings_realm_get(realm, role_id),
    )
    .await?;
    Ok(held.iter().filter_map(|role| role.name.clone()).collect())
}

async fn role_name(
    services: &HarvestServices,
    realm: &str,
    role_id: &str,
) -> Result<String, (Status, String)> {
    let client = keycloak(services.identity.client()).await?;
    let group =
        keycloak(client.client.realm_groups_with_group_id_get(realm, role_id))
            .await?;
    Ok(group.name.unwrap_or_else(|| role_id.to_string()))
}

async fn apply_role_permission(
    services: &HarvestServices,
    realm: &str,
    role_id: &str,
    permission: &str,
    change: RolePermissionChange,
) -> Result<(), (Status, String)> {
    let client = keycloak(services.identity.client()).await?;
    match change {
        RolePermissionChange::Added => {
            keycloak(client.set_role_permission(realm, role_id, permission))
                .await
        }
        RolePermissionChange::Removed => {
            keycloak(client.delete_role_permission(realm, role_id, permission))
                .await
        }
    }
}

fn opposite(change: RolePermissionChange) -> RolePermissionChange {
    match change {
        RolePermissionChange::Added => RolePermissionChange::Removed,
        RolePermissionChange::Removed => RolePermissionChange::Added,
    }
}

/// Turns `permission` on or off for a role.
pub async fn change_role_permission(
    services: &HarvestServices,
    claims: &JwtClaims,
    tenant_id: &str,
    role_id: &str,
    permission: &str,
    change: RolePermissionChange,
    allowed_by: &[Permissions],
) -> Result<(), (Status, String)> {
    let realm = get_tenant_realm(tenant_id);
    validate_role_path(&realm, role_id)?;
    let Some(action) = signing_action_of(permission) else {
        return apply_role_permission(
            services, &realm, role_id, permission, change,
        )
        .await;
    };
    let tenant_id = tenant_uuid(tenant_id)?;
    let mut client = database(services).await?;
    let (transaction, locked) = lock(&mut client, tenant_id, &[action]).await?;
    let holds = held_permissions(services, &realm, role_id)
        .await?
        .iter()
        .any(|held| held == permission);
    let changes = match (change, holds) {
        (RolePermissionChange::Added, false)
        | (RolePermissionChange::Removed, true) => true,
        // Already so: Keycloak is still asked, nothing is logged.
        (RolePermissionChange::Added, true)
        | (RolePermissionChange::Removed, false) => false,
    };
    if changes {
        let role_name = role_name(services, &realm, role_id).await?;
        stage_permission_changes(
            &transaction,
            &locked,
            &[SigningPermissionChange {
                tenant_id,
                action,
                role_id: role_id.to_string(),
                role_name,
                change,
                editor: editor(claims),
                allowed_by: allowed_by.to_vec(),
            }],
        )
        .await
        .map_err(server_error)?;
    }
    apply_role_permission(services, &realm, role_id, permission, change)
        .await?;
    if let Err(commit_error) = transaction.commit().await {
        if changes {
            if let Err(undo_error) = apply_role_permission(
                services,
                &realm,
                role_id,
                permission,
                opposite(change),
            )
            .await
            {
                error!(
                    role_id,
                    permission,
                    "The permission change is not logged and could not be undone: {undo_error:?}"
                );
            }
        }
        return Err(server_error(commit_error));
    }
    Ok(())
}

/// Creates the role `role`, with its permissions; sign permissions among
/// them are logged as added.
pub async fn create_role(
    services: &HarvestServices,
    claims: &JwtClaims,
    tenant_id: &str,
    role: &Role,
) -> Result<Role, (Status, String)> {
    let realm = get_tenant_realm(tenant_id);
    let actions = signing_actions_in(
        role.permissions.iter().flatten().map(String::as_str),
    );
    let logged_tenant = match actions.is_empty() {
        true => None,
        false => Some(tenant_uuid(tenant_id)?),
    };
    let mut database_client = match logged_tenant {
        Some(_) => Some(database(services).await?),
        None => None,
    };
    let locked = match (database_client.as_mut(), logged_tenant) {
        (Some(client), Some(tenant_id)) => {
            Some(lock(client, tenant_id, &actions).await?)
        }
        _ => None,
    };

    let client = keycloak(services.identity.client()).await?;
    let created = keycloak(client.create_role(&realm, role)).await?;
    let client = keycloak(services.identity.client()).await?;
    let with_id = keycloak(client.get_role_by_name(&realm, &created)).await?;
    let (Some(permissions), Some(id)) =
        (created.permissions.clone(), with_id.id)
    else {
        return Ok(created);
    };
    if let (Some((transaction, locked)), Some(tenant_id)) =
        (locked.as_ref(), logged_tenant)
    {
        let changes: Vec<SigningPermissionChange> = actions
            .iter()
            .map(|action| SigningPermissionChange {
                tenant_id,
                action: *action,
                role_id: id.clone(),
                role_name: created.name.clone().unwrap_or_else(|| id.clone()),
                change: RolePermissionChange::Added,
                editor: editor(claims),
                allowed_by: vec![Permissions::ROLE_CREATE],
            })
            .collect();
        stage_permission_changes(transaction, locked, &changes)
            .await
            .map_err(server_error)?;
    }
    let client = keycloak(services.identity.client()).await?;
    keycloak(client.set_role_permissions(&realm, &id, &permissions)).await?;
    if let Some((transaction, _)) = locked {
        if let Err(commit_error) = transaction.commit().await {
            // Without its entries the role mustn't keep its sign permissions.
            let undone = match keycloak(services.identity.client()).await {
                Ok(client) => keycloak(client.delete_role(&realm, &id)).await,
                Err(error) => Err(error),
            };
            if let Err(undo_error) = undone {
                error!(
                    role_id = id,
                    "The created role is not logged and could not be deleted: {undo_error:?}"
                );
            }
            return Err(server_error(commit_error));
        }
    }
    Ok(created)
}

/// Deletes a role; the sign permissions it held are logged as removed. A
/// deleted role can't be restored, so if the entries can't be committed
/// after Keycloak deleted it, the deletion stays unlogged.
pub async fn delete_role(
    services: &HarvestServices,
    claims: &JwtClaims,
    tenant_id: &str,
    role_id: &str,
) -> Result<(), (Status, String)> {
    let realm = get_tenant_realm(tenant_id);
    validate_role_path(&realm, role_id)?;
    let held = held_permissions(services, &realm, role_id).await?;
    if signing_actions_in(held.iter().map(String::as_str)).is_empty() {
        let client = keycloak(services.identity.client()).await?;
        return keycloak(client.delete_role(&realm, role_id)).await;
    }
    // A sign permission may be added meanwhile, so every action's events
    // are locked before the role is read again.
    let tenant_id = tenant_uuid(tenant_id)?;
    let actions: Vec<SigningAction> = SigningAction::iter().collect();
    let mut client = database(services).await?;
    let (transaction, locked) = lock(&mut client, tenant_id, &actions).await?;
    let held = held_permissions(services, &realm, role_id).await?;
    let held_actions = signing_actions_in(held.iter().map(String::as_str));
    if !held_actions.is_empty() {
        let role_name = role_name(services, &realm, role_id).await?;
        let changes: Vec<SigningPermissionChange> = held_actions
            .iter()
            .map(|action| SigningPermissionChange {
                tenant_id,
                action: *action,
                role_id: role_id.to_string(),
                role_name: role_name.clone(),
                change: RolePermissionChange::Removed,
                editor: editor(claims),
                allowed_by: vec![Permissions::ROLE_WRITE],
            })
            .collect();
        stage_permission_changes(&transaction, &locked, &changes)
            .await
            .map_err(server_error)?;
    }
    let keycloak_client = keycloak(services.identity.client()).await?;
    keycloak(keycloak_client.delete_role(&realm, role_id)).await?;
    transaction.commit().await.map_err(|commit_error| {
        error!(
            role_id,
            "The role was deleted but its sign permissions are not logged: {commit_error:?}"
        );
        server_error(commit_error)
    })
}
