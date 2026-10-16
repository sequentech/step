// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The audit entries of what routes change themselves, outside Hasura: an
//! `AdminChange` in the election event's log when the change is inside an
//! event, in the tenant's log otherwise.
//!
//! A route with a database transaction stages its entry in it. A change
//! Keycloak makes has none: the route queues the entry once Keycloak made
//! the change, so only a change that happened is logged. Keycloak's own
//! admin event covers a change whose entry could not be queued.

use crate::services::dependencies::HarvestServices;
use deadpool_postgres::Transaction;
use rocket::http::Status;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::services::keycloak::redacted_attributes;
use sequent_core::types::keycloak::{Permission, Role, User};
use serde_json::{json, Value};
use std::collections::HashMap;
use uuid::Uuid;
use windmill::postgres::audit_log_outbox::AuditLogTarget;
use windmill::services::audit_log::{
    self as audit_log, diff_values, stage, AdminAction, AdminChange,
    AuditActor, AuditObject, AuditOrigin, FieldChange,
};
use windmill::services::task_audit;
use windmill::tasks::audit_log_outbox::kick_audit_log_outbox;

pub const STAFF_ACCOUNT: &str = "staff_account";
pub const VOTER: &str = "voter";
pub const ROLE: &str = "role";
pub const PERMISSION: &str = "permission";
pub const PHONE_BLACKLIST: &str = "phone_blacklist";
pub const ELECTION_EVENT: &str = "election_event";
pub const CERTIFICATE_AUTHORITY: &str = "certificate_authority";

/// The id of an object that stands for several: a bulk deletion, an import.
pub const MANY: &str = "*";

const ENABLED_PATH: &str = "enabled";
const PASSWORD_PATH: &str = "password";
pub const ROLES_PATH: &str = "roles";
pub const PERMISSIONS_PATH: &str = "permissions";
pub const REALM_ATTRIBUTES_PATH: &str = "realm_attributes";
pub const PASSWORD_POLICY_PATH: &str = "password_policy";

type RouteError = (Status, String);

fn server_error(context: &str, error: impl std::fmt::Debug) -> RouteError {
    (Status::InternalServerError, format!("{context}: {error:?}"))
}

/// The admin a request comes from.
pub fn actor(claims: &JwtClaims) -> AuditActor {
    AuditActor {
        user_id: Some(claims.hasura_claims.user_id.clone()),
        username: claims.preferred_username.clone(),
        role: Some(claims.hasura_claims.default_role.clone()),
    }
}

/// The election event's log, or the tenant's without one.
pub fn target(
    tenant_id: &str,
    election_event_id: Option<&str>,
) -> Result<AuditLogTarget, RouteError> {
    let uuid = |name: &str, id: &str| {
        Uuid::parse_str(id).map_err(|error| {
            (Status::BadRequest, format!("Invalid {name}: {error}"))
        })
    };
    Ok(AuditLogTarget {
        tenant_id: uuid("tenant_id", tenant_id)?,
        election_event_id: election_event_id
            .map(|id| uuid("election_event_id", id))
            .transpose()?,
    })
}

/// A change this request makes.
pub fn change(
    claims: &JwtClaims,
    target: AuditLogTarget,
    object: AuditObject,
    action: AdminAction,
) -> AdminChange {
    AdminChange::new(
        target,
        object,
        action,
        actor(claims),
        AuditOrigin::Harvest,
    )
}

/// What the accounts of a realm are: an event realm holds voters, the
/// tenant realm staff.
pub fn account_kind(target: &AuditLogTarget) -> &'static str {
    match target.election_event_id {
        Some(_) => VOTER,
        None => STAFF_ACCOUNT,
    }
}

pub fn account(
    target: &AuditLogTarget,
    user_id: &str,
    username: Option<&str>,
) -> AuditObject {
    AuditObject::new(account_kind(target), user_id, username)
}

/// The fields of an account an entry compares.
fn account_fields(user: &User) -> Value {
    json!({
        "username": user.username,
        "email": user.email,
        "email_verified": user.email_verified,
        "enabled": user.enabled,
        "first_name": user.first_name,
        "last_name": user.last_name,
        "attributes": user.attributes,
    })
}

/// A created account, with every field it starts with and the roles it was
/// given.
pub fn account_created(
    claims: &JwtClaims,
    target: AuditLogTarget,
    user: &User,
    role_ids: &[String],
) -> AdminChange {
    let id = user.id.as_deref().unwrap_or_default();
    let mut changes = diff_values("", None, Some(&account_fields(user)));
    if !role_ids.is_empty() {
        changes.push(FieldChange::new(
            ROLES_PATH,
            None,
            Some(&json!(role_ids)),
        ));
    }
    change(
        claims,
        target,
        account(&target, id, user.username.as_deref()),
        AdminAction::Create,
    )
    .with_changes(changes)
}

/// Whether an edit set the account's password.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PasswordEdit {
    Kept,
    Set,
}

/// An edited account: enabling or disabling it and resetting its password
/// are actions of their own when the edit did nothing else. The password
/// is named, never recorded.
pub fn account_edited(
    claims: &JwtClaims,
    target: AuditLogTarget,
    before: &User,
    after: &User,
    password: PasswordEdit,
) -> AdminChange {
    let id = after
        .id
        .as_deref()
        .or(before.id.as_deref())
        .unwrap_or_default();
    let mut changes = diff_values(
        "",
        Some(&account_fields(before)),
        Some(&account_fields(after)),
    );
    if password == PasswordEdit::Set {
        changes.push(FieldChange::new(PASSWORD_PATH, None, None));
    }
    let action = match changes.as_slice() {
        [only] if only.path == PASSWORD_PATH => AdminAction::ResetPassword,
        [only] if only.path == ENABLED_PATH => match after.enabled {
            Some(false) => AdminAction::Disable,
            _ => AdminAction::Enable,
        },
        _ => AdminAction::Update,
    };
    change(
        claims,
        target,
        account(&target, id, after.username.as_deref()),
        action,
    )
    .with_changes(changes)
}

/// Several accounts deleted at once: one entry with their ids.
pub fn accounts_deleted(
    claims: &JwtClaims,
    target: AuditLogTarget,
    user_ids: &[String],
) -> AdminChange {
    let kind = account_kind(&target);
    change(
        claims,
        target,
        AuditObject::new(kind, MANY, None),
        AdminAction::Delete,
    )
    .with_extra(json!({"count": user_ids.len(), "ids": user_ids}))
    .with_description(format!(
        "Deleted {} {} accounts",
        user_ids.len(),
        match kind {
            VOTER => "voter",
            _ => "staff",
        }
    ))
}

/// A role given to an account or taken from it.
pub fn account_role(
    claims: &JwtClaims,
    target: AuditLogTarget,
    user: AuditObject,
    action: AdminAction,
    role: &str,
) -> AdminChange {
    let role = json!(role);
    let (old, new) = match action {
        AdminAction::Revoke => (Some(&role), None),
        _ => (None, Some(&role)),
    };
    change(claims, target, user, action)
        .with_changes(vec![FieldChange::new(ROLES_PATH, old, new)])
}

pub fn role(role_id: &str, name: Option<&str>) -> AuditObject {
    AuditObject::new(ROLE, role_id, name)
}

/// A created role, with the permissions it starts with. Keycloak names the
/// role's id only when asked again: without it the role goes by its name.
pub fn role_created(
    claims: &JwtClaims,
    target: AuditLogTarget,
    created: &Role,
    id: Option<&str>,
) -> AdminChange {
    let fields = json!({
        "name": created.name,
        "permissions": created.permissions,
        "attributes": created.attributes,
    });
    let id = id.or(created.name.as_deref()).unwrap_or_default();
    change(
        claims,
        target,
        role(id, created.name.as_deref()),
        AdminAction::Create,
    )
    .with_changes(diff_values("", None, Some(&fields)))
}

/// A deleted role, with the permissions it held.
pub fn role_deleted(
    claims: &JwtClaims,
    target: AuditLogTarget,
    role_id: &str,
    name: Option<&str>,
    held: &[String],
) -> AdminChange {
    let mut held = held.to_vec();
    held.sort();
    let changes = match held.is_empty() {
        true => vec![],
        false => {
            vec![FieldChange::new(PERMISSIONS_PATH, Some(&json!(held)), None)]
        }
    };
    change(claims, target, role(role_id, name), AdminAction::Delete)
        .with_changes(changes)
}

/// A permission given to a role or taken from it.
pub fn role_permission(
    claims: &JwtClaims,
    target: AuditLogTarget,
    role: AuditObject,
    action: AdminAction,
    permission: &str,
) -> AdminChange {
    let permission = json!(permission);
    let (old, new) = match action {
        AdminAction::Revoke => (Some(&permission), None),
        _ => (None, Some(&permission)),
    };
    change(claims, target, role, action).with_changes(vec![FieldChange::new(
        PERMISSIONS_PATH,
        old,
        new,
    )])
}

/// An election event, for a change to its own settings or its realm's.
pub fn election_event(election_event_id: &str) -> AuditObject {
    AuditObject::new(ELECTION_EVENT, election_event_id, None)
}

/// A change of a secret: its path, never its values.
pub fn secret_change(path: &str) -> FieldChange {
    FieldChange {
        path: path.to_string(),
        old: None,
        new: None,
        secret: true,
        old_sha256: None,
        new_sha256: None,
    }
}

/// What an edit changes in a realm's attributes. An empty value removes
/// the attribute. An attribute Keycloak's own redaction hides is a secret
/// whatever its name.
pub fn realm_attribute_changes(
    current: &HashMap<String, String>,
    updates: &HashMap<String, String>,
) -> Vec<FieldChange> {
    let shown = redacted_attributes(updates);
    let mut keys: Vec<&String> = updates
        .keys()
        .filter(|key| {
            let old = current.get(*key).filter(|value| !value.is_empty());
            let new = updates.get(*key).filter(|value| !value.is_empty());
            old != new
        })
        .collect();
    keys.sort();
    keys.into_iter()
        .map(|key| {
            let path = format!("{REALM_ATTRIBUTES_PATH}.{key}");
            if shown.get(key) != updates.get(key) {
                return secret_change(&path);
            }
            let value = |attributes: &HashMap<String, String>| {
                attributes
                    .get(key)
                    .filter(|value| !value.is_empty())
                    .map(|value| json!(value))
            };
            FieldChange::new(
                &path,
                value(current).as_ref(),
                value(updates).as_ref(),
            )
        })
        .collect()
}

/// A blacklisted number goes by the number.
pub fn phone_blacklist(entry_id: &str, phone_e164: &str) -> AuditObject {
    AuditObject::new(PHONE_BLACKLIST, entry_id, Some(phone_e164))
}

/// A permission goes by its name.
pub fn permission(name: &str) -> AuditObject {
    AuditObject::new(PERMISSION, name, Some(name))
}

pub fn permission_created(
    claims: &JwtClaims,
    target: AuditLogTarget,
    created: &Permission,
) -> AdminChange {
    let fields = json!({
        "name": created.name,
        "description": created.description,
        "attributes": created.attributes,
    });
    change(
        claims,
        target,
        permission(created.name.as_deref().unwrap_or_default()),
        AdminAction::Create,
    )
    .with_changes(diff_values("", None, Some(&fields)))
}

/// Objects of `kind` imported from a document: one entry for all of them,
/// with the document and the checksum its admin declared.
pub fn imported(
    claims: &JwtClaims,
    target: AuditLogTarget,
    kind: &str,
    document_id: &str,
    declared_sha256: Option<&str>,
    description: &str,
) -> AdminChange {
    change(
        claims,
        target,
        AuditObject::new(kind, MANY, None),
        AdminAction::Import,
    )
    .with_extra(task_audit::source_document(document_id, declared_sha256))
    .with_description(description)
}

/// An export a route queued: what is exported and, when the route has them,
/// the task that writes the file and the document it writes.
pub fn exported(
    claims: &JwtClaims,
    target: AuditLogTarget,
    object: AuditObject,
    description: &str,
    task_execution_id: Option<&str>,
    document_id: Option<&str>,
) -> AdminChange {
    let mut extra = json!({});
    if let Some(task_execution_id) = task_execution_id {
        extra["task_execution_id"] = json!(task_execution_id);
    }
    if let Some(document_id) = document_id {
        extra["document_id"] = json!(document_id);
    }
    change(claims, target, object, AdminAction::Export)
        .with_extra(extra)
        .with_description(description)
}

/// Queues the entry of work that committed outside the route's own
/// transaction (a task, a file), and asks a worker to post it.
pub async fn record_after(change: AdminChange) -> Result<(), RouteError> {
    task_audit::record(change).await.map_err(|error| {
        server_error(
            "The change was made, but its audit entry could not be queued",
            error,
        )
    })
}

/// The rows `transaction` changes from now on, in the tables with the
/// `audit_change` trigger, are logged as the request's admin's, with their
/// old and new values, when the transaction commits. Call
/// `kick_audit_log_outbox()` after the commit.
pub async fn act_for(
    transaction: &Transaction<'_>,
    claims: &JwtClaims,
) -> Result<(), RouteError> {
    let tenant_id =
        Uuid::parse_str(&claims.hasura_claims.tenant_id).map_err(|error| {
            (Status::BadRequest, format!("Invalid tenant_id: {error}"))
        })?;
    audit_log::act_for(
        transaction,
        &actor(claims),
        AuditOrigin::Harvest,
        tenant_id,
    )
    .await
    .map_err(|error| {
        server_error("The admin of the change could not be named", error)
    })
}

/// Queues the entry of a change that was made outside the database, and
/// asks a worker to post it.
pub async fn record(
    services: &HarvestServices,
    change: &AdminChange,
) -> Result<(), RouteError> {
    const CONTEXT: &str =
        "The change was made, but its audit entry could not be queued";
    let mut client = services
        .databases
        .hasura()
        .await
        .get()
        .await
        .map_err(|error| server_error(CONTEXT, error))?;
    let transaction = client
        .transaction()
        .await
        .map_err(|error| server_error(CONTEXT, error))?;
    stage(&transaction, change)
        .await
        .map_err(|error| server_error(CONTEXT, error))?;
    transaction
        .commit()
        .await
        .map_err(|error| server_error(CONTEXT, error))?;
    kick_audit_log_outbox();
    Ok(())
}

#[cfg(test)]
#[path = "../../tests/support/audit_entries.rs"]
mod tests;

#[cfg(test)]
#[path = "../../tests/support/audited_routes.rs"]
mod audited_routes;
