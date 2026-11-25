// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::Context;
use deadpool_postgres::Transaction;
use rocket::http::Status;
use rocket::response::status::Unauthorized;
use sequent_core::ballot::{VotingStatus, VotingStatusChannel};
use sequent_core::services::jwt::{decode_permission_labels, JwtClaims};
use sequent_core::types::permissions::{Permissions, VoterPermissions};
use std::collections::HashSet;
use std::env;
use tracing::{error, info, instrument};
use windmill::postgres::election_event::get_election_event_permission_label;
use windmill::services::database::get_hasura_pool;
use windmill::services::import::import_users::{
    GroupAssignmentPolicy, ImportUsersPrivileges, PermissionLabelPolicy,
};

pub use sequent_core::services::authorization::*;

const ELECTION_EVENT_OUTSIDE_PERMISSION_LABELS: &str =
    "Election event outside the user's permission labels";

/// Whether the caller's permission labels allow an election event with this
/// label, as Hasura applies `X-Hasura-Permission-Labels`: an event without a
/// label is open to every admin user of the tenant, a labelled one only to
/// admin users who hold its label.
pub fn permission_labels_allow(
    claims: &JwtClaims,
    event_permission_label: Option<&str>,
) -> bool {
    event_permission_label.map_or(true, |label| {
        decode_permission_labels(claims)
            .iter()
            .any(|held| held == label)
    })
}

fn ensure_event_permitted(
    claims: &JwtClaims,
    event_permission_label: Option<&str>,
) -> Result<(), (Status, String)> {
    if permission_labels_allow(claims, event_permission_label) {
        Ok(())
    } else {
        Err((
            Status::Forbidden,
            ELECTION_EVENT_OUTSIDE_PERMISSION_LABELS.to_string(),
        ))
    }
}

/// Refuses unless the caller's permission labels allow the election event,
/// as Hasura applies them to the event and to every row that belongs to it.
/// An event the tenant does not have has nothing to protect: the step that
/// uses it reports it as missing.
#[instrument(skip(hasura_transaction, claims))]
pub async fn authorize_election_event_permission_label(
    hasura_transaction: &Transaction<'_>,
    claims: &JwtClaims,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<(), (Status, String)> {
    let Some(permission_label) = get_election_event_permission_label(
        hasura_transaction,
        tenant_id,
        election_event_id,
    )
    .await
    .map_err(|error| (Status::InternalServerError, format!("{error:?}")))?
    else {
        return Ok(());
    };
    ensure_event_permitted(claims, permission_label.as_deref())
}

/// `authorize_election_event_permission_label` for a route that has no
/// database transaction of its own at the point where it authorizes.
#[instrument(skip(claims))]
pub async fn authorize_election_event_access(
    claims: &JwtClaims,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<(), (Status, String)> {
    let internal = |error: String| (Status::InternalServerError, error);
    let mut client = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|error| internal(format!("{error:?}")))?;
    let transaction = client
        .transaction()
        .await
        .map_err(|error| internal(format!("{error:?}")))?;
    authorize_election_event_permission_label(
        &transaction,
        claims,
        tenant_id,
        election_event_id,
    )
    .await
}

/// What a user import started by `claims` may grant: assigning arbitrary
/// groups needs the same permissions as the set-user-role route, and setting
/// permission labels needs permission-label-write.
pub fn import_users_privileges(
    claims: &JwtClaims,
    tenant_id: &str,
) -> ImportUsersPrivileges {
    let holds = |permissions: Vec<Permissions>| {
        authorize(claims, true, Some(tenant_id.to_string()), permissions)
            .is_ok()
    };
    ImportUsersPrivileges {
        group_assignment: if holds(vec![
            Permissions::USER_WRITE,
            Permissions::ROLE_WRITE,
        ]) {
            GroupAssignmentPolicy::AnyGroup
        } else {
            GroupAssignmentPolicy::DefaultGroupOnly
        },
        permission_labels: if holds(vec![Permissions::PERMISSION_LABEL_WRITE]) {
            PermissionLabelPolicy::Allowed
        } else {
            PermissionLabelPolicy::Forbidden
        },
    }
}

#[cfg(test)]
mod import_users_privileges_tests {
    use super::*;

    const TENANT_ID: &str = "tenant";

    /// Claims of a user of `TENANT_ID` whose roles are exactly `permissions`.
    fn admin_with(permissions: &[Permissions]) -> JwtClaims {
        let allowed_roles = permissions
            .iter()
            .map(|permission| permission.to_string())
            .collect::<Vec<String>>();
        serde_json::from_value(serde_json::json!({
            "exp": 1, "iat": 0, "jti": "test", "iss": "test",
            "sub": "admin", "typ": "Bearer", "azp": "admin-portal",
            "acr": "1", "allowed-origins": [], "scope": "openid",
            "email_verified": false,
            "https://hasura.io/jwt/claims": {
                "x-hasura-default-role": "admin-user",
                "x-hasura-tenant-id": TENANT_ID,
                "x-hasura-user-id": "admin",
                "x-hasura-allowed-roles": allowed_roles
            }
        }))
        .expect("valid claims")
    }

    /// The create permission of either scope does not allow other groups or
    /// permission labels in an import.
    #[test]
    fn create_permission_alone_grants_only_the_default_group_and_no_labels() {
        for permission in [Permissions::USER_CREATE, Permissions::VOTER_CREATE]
        {
            assert_eq!(
                import_users_privileges(&admin_with(&[permission]), TENANT_ID),
                ImportUsersPrivileges {
                    group_assignment: GroupAssignmentPolicy::DefaultGroupOnly,
                    permission_labels: PermissionLabelPolicy::Forbidden,
                }
            );
        }
    }

    /// Importing users into any group needs the permissions of the
    /// set-user-role route, and one of them is not enough.
    #[test]
    fn any_group_needs_both_user_write_and_role_write() {
        for (permissions, expected) in [
            (
                vec![Permissions::USER_CREATE, Permissions::ROLE_WRITE],
                GroupAssignmentPolicy::DefaultGroupOnly,
            ),
            (
                vec![Permissions::USER_CREATE, Permissions::USER_WRITE],
                GroupAssignmentPolicy::DefaultGroupOnly,
            ),
            (
                vec![
                    Permissions::USER_CREATE,
                    Permissions::USER_WRITE,
                    Permissions::ROLE_WRITE,
                ],
                GroupAssignmentPolicy::AnyGroup,
            ),
        ] {
            assert_eq!(
                import_users_privileges(&admin_with(&permissions), TENANT_ID)
                    .group_assignment,
                expected,
                "{permissions:?}"
            );
        }
    }

    /// Permission labels in an import need `permission-label-write`.
    #[test]
    fn labels_need_permission_label_write() {
        let privileges = import_users_privileges(
            &admin_with(&[
                Permissions::USER_CREATE,
                Permissions::PERMISSION_LABEL_WRITE,
            ]),
            TENANT_ID,
        );
        assert_eq!(
            privileges.permission_labels,
            PermissionLabelPolicy::Allowed
        );
    }

    /// Permissions held for one tenant grant nothing in an import into another
    /// tenant.
    #[test]
    fn privileges_in_another_tenant_are_not_granted() {
        let privileges = import_users_privileges(
            &admin_with(&[
                Permissions::USER_WRITE,
                Permissions::ROLE_WRITE,
                Permissions::PERMISSION_LABEL_WRITE,
            ]),
            "another-tenant",
        );
        assert_eq!(privileges, ImportUsersPrivileges::default());
    }
}

#[cfg(test)]
pub(crate) mod test_claims {
    use sequent_core::services::jwt::JwtClaims;

    pub(crate) const CALLER_TENANT_ID: &str =
        "4f1c6b38-9f5e-4a59-8d3b-2a7f0c1e5d61";
    pub(crate) const OTHER_TENANT_ID: &str =
        "b7e2d915-3c4a-4e8f-9a61-5d0f2c8b7e43";

    pub(crate) fn admin_claims(tenant_id: &str, roles: &[&str]) -> JwtClaims {
        serde_json::from_value(serde_json::json!({
            "exp": 1, "iat": 0, "jti": "test", "iss": "test",
            "sub": "admin", "typ": "Bearer", "azp": "admin-portal",
            "acr": "1", "allowed-origins": [], "scope": "openid",
            "email_verified": false,
            "https://hasura.io/jwt/claims": {
                "x-hasura-default-role": "admin-user",
                "x-hasura-tenant-id": tenant_id,
                "x-hasura-user-id": "admin",
                "x-hasura-allowed-roles": roles
            }
        }))
        .expect("test claims must deserialize")
    }
}

#[cfg(test)]
mod event_permission_label_tests {
    use super::test_claims::{admin_claims, CALLER_TENANT_ID};
    use super::*;

    fn holding(permission_labels: Option<&str>) -> JwtClaims {
        let mut claims = admin_claims(CALLER_TENANT_ID, &[]);
        claims.hasura_claims.permission_labels =
            permission_labels.map(str::to_string);
        claims
    }

    #[test]
    fn refuses_a_labelled_event_outside_the_permission_labels() {
        let north = holding(Some(r#"{"north"}"#));
        assert_eq!(ensure_event_permitted(&north, None), Ok(()));
        assert_eq!(ensure_event_permitted(&north, Some("north")), Ok(()));
        assert_eq!(ensure_event_permitted(&holding(None), None), Ok(()));
        for caller in [north, holding(Some("{}")), holding(None)] {
            assert_eq!(
                ensure_event_permitted(&caller, Some("south")),
                Err((
                    Status::Forbidden,
                    ELECTION_EVENT_OUTSIDE_PERMISSION_LABELS.to_string()
                ))
            );
        }
    }
}
