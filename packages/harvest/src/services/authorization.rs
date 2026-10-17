// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::Context;
use rocket::http::Status;
use rocket::response::status::Unauthorized;
use sequent_core::ballot::{VotingStatus, VotingStatusChannel};
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::{Permissions, VoterPermissions};
use std::collections::HashSet;
use std::env;
use tracing::{error, info, instrument};
use windmill::services::import::import_users::{
    GroupAssignmentPolicy, ImportUsersPrivileges, PermissionLabelPolicy,
};

pub use sequent_core::services::authorization::*;

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
