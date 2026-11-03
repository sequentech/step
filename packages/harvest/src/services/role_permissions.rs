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
//!
//! A created or deleted role and a permission a role gains or loses are
//! also AdminChange entries of the tenant's log (see [`audit`]): committed
//! with the signing entries when there are any, queued once Keycloak made
//! the change otherwise.

use crate::services::audit;
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
use windmill::postgres::audit_log_outbox::AuditLogTarget;
use windmill::services::audit_log::{stage, AdminAction, AdminChange};
use windmill::services::signing::log::Actor;
use windmill::services::signing::permissions::{
    lock_permission_change_events, signing_action_of, signing_actions_in,
    stage_permission_changes, LockedEvents, RolePermissionChange,
    SigningPermissionChange,
};
use windmill::tasks::audit_log_outbox::kick_audit_log_outbox;

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

fn admin_action(change: RolePermissionChange) -> AdminAction {
    match change {
        RolePermissionChange::Added => AdminAction::Grant,
        RolePermissionChange::Removed => AdminAction::Revoke,
    }
}

/// Whether `change` changes what the role holds: adding a permission it
/// holds or removing one it lacks changes nothing. Keycloak is still asked;
/// nothing is logged.
async fn changes_role(
    services: &HarvestServices,
    realm: &str,
    role_id: &str,
    permission: &str,
    change: RolePermissionChange,
) -> Result<bool, (Status, String)> {
    let holds = held_permissions(services, realm, role_id)
        .await?
        .iter()
        .any(|held| held == permission);
    Ok(match change {
        RolePermissionChange::Added => !holds,
        RolePermissionChange::Removed => holds,
    })
}

/// The tenant log's entry of a permission given to a role or taken from it.
async fn audit_entry(
    services: &HarvestServices,
    claims: &JwtClaims,
    target: AuditLogTarget,
    realm: &str,
    role_id: &str,
    permission: &str,
    change: RolePermissionChange,
) -> Result<AdminChange, (Status, String)> {
    let role_name = role_name(services, realm, role_id).await?;
    Ok(audit::role_permission(
        claims,
        target,
        audit::role(role_id, Some(&role_name)),
        admin_action(change),
        permission,
    ))
}

/// Turns `permission` on or off for a role. A change is logged in the
/// tenant's log once Keycloak made it; a sign permission's entry is
/// committed with its signing entries.
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
    let target = audit::target(tenant_id, None)?;
    let Some(action) = signing_action_of(permission) else {
        let entry =
            match changes_role(services, &realm, role_id, permission, change)
                .await?
            {
                true => Some(
                    audit_entry(
                        services, claims, target, &realm, role_id, permission,
                        change,
                    )
                    .await?,
                ),
                false => None,
            };
        apply_role_permission(services, &realm, role_id, permission, change)
            .await?;
        return match entry {
            Some(entry) => audit::record(services, &entry).await,
            None => Ok(()),
        };
    };
    let tenant_id = tenant_uuid(tenant_id)?;
    let mut client = database(services).await?;
    let (transaction, locked) = lock(&mut client, tenant_id, &[action]).await?;
    let changes =
        changes_role(services, &realm, role_id, permission, change).await?;
    if changes {
        let role_name = role_name(services, &realm, role_id).await?;
        stage_permission_changes(
            &transaction,
            &locked,
            &[SigningPermissionChange {
                tenant_id,
                action,
                role_id: role_id.to_string(),
                role_name: role_name.clone(),
                change,
                editor: editor(claims),
                allowed_by: allowed_by.to_vec(),
            }],
        )
        .await
        .map_err(server_error)?;
        stage(
            &transaction,
            &audit::role_permission(
                claims,
                target,
                audit::role(role_id, Some(&role_name)),
                admin_action(change),
                permission,
            ),
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
    if changes {
        kick_audit_log_outbox();
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
    let target = audit::target(tenant_id, None)?;
    let (Some(permissions), Some(id)) =
        (created.permissions.clone(), with_id.id.clone())
    else {
        audit::record(
            services,
            &audit::role_created(
                claims,
                target,
                &created,
                with_id.id.as_deref(),
            ),
        )
        .await?;
        return Ok(created);
    };
    let entry = audit::role_created(claims, target, &created, Some(&id));
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
        stage(transaction, &entry).await.map_err(server_error)?;
    }
    let client = keycloak(services.identity.client()).await?;
    keycloak(client.set_role_permissions(&realm, &id, &permissions)).await?;
    if locked.is_none() {
        audit::record(services, &entry).await?;
    }
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
        kick_audit_log_outbox();
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
    let target = audit::target(tenant_id, None)?;
    let held = held_permissions(services, &realm, role_id).await?;
    if signing_actions_in(held.iter().map(String::as_str)).is_empty() {
        let name = role_name(services, &realm, role_id).await?;
        let client = keycloak(services.identity.client()).await?;
        keycloak(client.delete_role(&realm, role_id)).await?;
        return audit::record(
            services,
            &audit::role_deleted(claims, target, role_id, Some(&name), &held),
        )
        .await;
    }
    // A sign permission may be added meanwhile, so every action's events
    // are locked before the role is read again.
    let tenant_id = tenant_uuid(tenant_id)?;
    let actions: Vec<SigningAction> = SigningAction::iter().collect();
    let mut client = database(services).await?;
    let (transaction, locked) = lock(&mut client, tenant_id, &actions).await?;
    let held = held_permissions(services, &realm, role_id).await?;
    let held_actions = signing_actions_in(held.iter().map(String::as_str));
    let role_name = role_name(services, &realm, role_id).await?;
    stage(
        &transaction,
        &audit::role_deleted(claims, target, role_id, Some(&role_name), &held),
    )
    .await
    .map_err(server_error)?;
    if !held_actions.is_empty() {
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
    })?;
    kick_audit_log_outbox();
    Ok(())
}
