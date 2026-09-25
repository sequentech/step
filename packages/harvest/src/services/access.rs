// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The permissions each family of routes requires. Routes pass these vectors
//! to `authorize` unchanged, so their order is the order a denial lists.

use rocket::http::Status;
use sequent_core::types::hasura::core::DocumentAnnotations;
use sequent_core::types::keycloak::PERMISSION_LABELS;
use sequent_core::types::permissions::Permissions;
use std::collections::HashMap;
use std::fmt;

pub type Attributes = HashMap<String, Vec<String>>;
pub type SecretAttributes = HashMap<String, Option<Vec<String>>>;

/// Whose accounts a user-management request addresses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UserScope {
    /// The voters of an election event.
    Voters,
    /// The tenant's own users.
    Tenant,
}

impl UserScope {
    pub fn of(election_event_id: Option<&str>) -> Self {
        match election_event_id {
            Some(_) => Self::Voters,
            None => Self::Tenant,
        }
    }
}

pub fn read_permission(scope: UserScope) -> Permissions {
    match scope {
        UserScope::Voters => Permissions::VOTER_READ,
        UserScope::Tenant => Permissions::USER_READ,
    }
}

pub fn create_permission(scope: UserScope) -> Permissions {
    match scope {
        UserScope::Voters => Permissions::VOTER_CREATE,
        UserScope::Tenant => Permissions::USER_CREATE,
    }
}

pub fn delete_permission(scope: UserScope) -> Permissions {
    match scope {
        UserScope::Voters => Permissions::VOTER_DELETE,
        UserScope::Tenant => Permissions::USER_WRITE,
    }
}

/// Encrypted attributes only exist for election-event voters. A request for
/// tenant users that sets them is refused before its permissions are checked.
#[derive(Debug, PartialEq, Eq)]
pub struct SecretAttributesOutsideElectionEvent;

impl fmt::Display for SecretAttributesOutsideElectionEvent {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(
            "Encrypted attributes are only supported for election-event voters",
        )
    }
}

/// An empty map writes no encrypted attribute; clearing one is a write.
pub fn writes_secret_attributes(
    secret_attributes: Option<&SecretAttributes>,
) -> bool {
    secret_attributes.is_some_and(|attributes| !attributes.is_empty())
}

fn sets_permission_labels(attributes: Option<&Attributes>) -> bool {
    attributes
        .is_some_and(|attributes| attributes.contains_key(PERMISSION_LABELS))
}

pub fn create_user_permissions(
    scope: UserScope,
    secret_attributes: Option<&SecretAttributes>,
    attributes: Option<&Attributes>,
) -> Result<Vec<Permissions>, SecretAttributesOutsideElectionEvent> {
    let mut permissions = vec![create_permission(scope)];
    match scope {
        UserScope::Voters => {
            if writes_secret_attributes(secret_attributes) {
                permissions.push(Permissions::VOTER_SECRET_ATTRIBUTE_WRITE);
            }
        }
        UserScope::Tenant => {
            if writes_secret_attributes(secret_attributes) {
                return Err(SecretAttributesOutsideElectionEvent);
            }
            // Only a caller who may label users can set their permission
            // labels.
            if sets_permission_labels(attributes) {
                permissions.push(Permissions::PERMISSION_LABEL_WRITE);
            }
        }
    }
    Ok(permissions)
}

/// The fields of an edit-user request its permissions depend on.
pub struct UserEdit<'a> {
    pub scope: UserScope,
    pub enabled: Option<bool>,
    pub attributes: Option<&'a Attributes>,
    pub secret_attributes: Option<&'a SecretAttributes>,
    pub email: Option<&'a str>,
    pub first_name: Option<&'a str>,
    pub last_name: Option<&'a str>,
    pub username: Option<&'a str>,
    pub password: Option<&'a str>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct UserEditAccess {
    pub permissions: Vec<Permissions>,
    /// The request only sets a voter's password.
    pub password_only: bool,
    /// The caller holds VOTER_VOTED_EDIT, so voters who have voted can be
    /// edited.
    pub voter_voted_edit: bool,
    /// The caller holds VOTER_EMAIL_TLF_EDIT.
    pub voter_email_tlf_edit: bool,
}

pub fn edit_user_access(
    edit: &UserEdit,
    roles: &[String],
) -> Result<UserEditAccess, SecretAttributesOutsideElectionEvent> {
    let holds =
        |permission: Permissions| roles.contains(&permission.to_string());
    let writes_secrets = writes_secret_attributes(edit.secret_attributes);
    match edit.scope {
        UserScope::Tenant => {
            if writes_secrets {
                return Err(SecretAttributesOutsideElectionEvent);
            }
            let mut permissions = vec![Permissions::USER_WRITE];
            if sets_permission_labels(edit.attributes) {
                permissions.push(Permissions::PERMISSION_LABEL_WRITE);
            }
            Ok(UserEditAccess {
                permissions,
                password_only: false,
                voter_voted_edit: false,
                voter_email_tlf_edit: false,
            })
        }
        UserScope::Voters => {
            let password_only = edit.password.is_some()
                && edit.enabled.is_none()
                && edit.attributes.is_none()
                && edit.secret_attributes.is_none()
                && edit.email.is_none()
                && edit.first_name.is_none()
                && edit.last_name.is_none()
                && edit.username.is_none();
            let voter_write = holds(Permissions::VOTER_WRITE);
            let permissions = if password_only {
                vec![Permissions::VOTER_CHANGE_PASSWORD]
            } else {
                // Without VOTER_WRITE, VOTER_EMAIL_TLF_EDIT takes its place.
                let mut permissions = vec![if voter_write {
                    Permissions::VOTER_WRITE
                } else {
                    Permissions::VOTER_EMAIL_TLF_EDIT
                }];
                if edit.password.is_some() {
                    permissions.push(Permissions::VOTER_CHANGE_PASSWORD);
                }
                if writes_secrets {
                    permissions.push(Permissions::VOTER_SECRET_ATTRIBUTE_WRITE);
                    if !voter_write {
                        permissions.push(Permissions::VOTER_WRITE);
                    }
                }
                permissions
            };
            Ok(UserEditAccess {
                permissions,
                password_only,
                voter_voted_edit: holds(Permissions::VOTER_VOTED_EDIT),
                voter_email_tlf_edit: holds(Permissions::VOTER_EMAIL_TLF_EDIT),
            })
        }
    }
}

/// What reading a document requires beyond the route's own permissions.
pub fn document_extra_permissions(
    annotations: Option<&DocumentAnnotations>,
) -> Vec<Permissions> {
    if annotations
        .is_some_and(DocumentAnnotations::requires_voter_secret_attribute_read)
    {
        vec![Permissions::VOTER_SECRET_ATTRIBUTE_READ]
    } else {
        vec![]
    }
}

/// Grants access when either check passed. Both checks have already run, so
/// both logged their outcome; when both failed, the second failure is the one
/// returned.
pub fn authorize_any(
    first: Result<(), (Status, String)>,
    second: Result<(), (Status, String)>,
) -> Result<(), (Status, String)> {
    first.or(second)
}

#[cfg(test)]
#[path = "../../tests/support/access_policy.rs"]
mod tests;
