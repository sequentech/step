// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::Context;
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use rocket::response::status::Unauthorized;
use sequent_core::ballot::{VotingStatus, VotingStatusChannel};
use sequent_core::services::jwt::JwtClaims;
use sequent_core::services::uuid_validation::parse_uuid_v4_field;
use sequent_core::types::permissions::{
    Permissions, RealmRolePolicy, VoterPermissions,
};
use std::collections::HashSet;
use std::env;
use tracing::{error, info, instrument};
use windmill::postgres::election::get_election_by_id;
use windmill::services::database::get_hasura_pool;
use windmill::services::import::import_users::{
    GroupAssignmentPolicy, ImportUsersPrivileges, PermissionLabelPolicy,
};

#[instrument(skip(claims))]
pub fn authorize(
    claims: &JwtClaims,
    allow_super_admin_auth: bool, // Allow authorizing super admin tenant
    tenant_id_opt: Option<String>,
    permissions: Vec<Permissions>,
) -> Result<(), (Status, String)> {
    // Verify tenant id
    let allowed = match (tenant_id_opt.clone(), allow_super_admin_auth) {
        (Some(tenant_id), _)
            if tenant_id.eq(&claims.hasura_claims.tenant_id) =>
        {
            true // is valid tenant
        }

        (_, true) => {
            let super_admin_tenant_id = env::var("SUPER_ADMIN_TENANT_ID")
                .map_err(|_| {
                    (
                        Status::Unauthorized,
                        format!("SUPER_ADMIN_TENANT_ID must be set"),
                    )
                })?;
            info!("super_admin_tenant_id: {super_admin_tenant_id}");
            super_admin_tenant_id == claims.hasura_claims.tenant_id // is super admin?
        }
        (_, _) => false, // Is not valid tenant nor super admin
    };

    if !allowed {
        error!(
            "Not authorized: allow_super_admin_auth: {allow_super_admin_auth}, 
            tenant_id_opt: {tenant_id_opt:?}, claims tenant_id: {}",
            claims.hasura_claims.tenant_id
        );
        return Err((Status::Unauthorized, format!("Unathorized: not a super admin or invalid tenant_id {tenant_id_opt:?}")));
    }

    let perms_str: Vec<String> = permissions
        .into_iter()
        .map(|permission| permission.to_string())
        .collect();
    let permissions_set: HashSet<_> =
        claims.hasura_claims.allowed_roles.iter().collect();
    let all_contained =
        perms_str.iter().all(|item| permissions_set.contains(&item));

    if !all_contained {
        Err((
            Status::Unauthorized,
            format!("Unathorized: {perms_str:?} not in {permissions_set:?}"),
        ))
    } else {
        Ok(())
    }
}

/// Rejects a permission name that is reserved for the platform.
pub fn require_ordinary_permission(name: &str) -> Result<(), (Status, String)> {
    RealmRolePolicy::require_ordinary(name)
        .map_err(|error| (Status::BadRequest, error.to_string()))
}

// returns area_id
#[instrument(skip(claims))]
pub fn authorize_voter_election(
    claims: &JwtClaims,
    permissions: Vec<VoterPermissions>,
    election_id: &String,
) -> Result<(String, VotingStatusChannel), (Status, String)> {
    let perms_str: Vec<String> = permissions
        .into_iter()
        .map(|permission| permission.to_string())
        .collect();
    let permissions_set: HashSet<_> =
        claims.hasura_claims.allowed_roles.iter().collect();
    let all_contained =
        perms_str.iter().all(|item| permissions_set.contains(&item));

    if !all_contained {
        return Err((Status::Unauthorized, "".into()));
    }

    let Some(area_id) = claims.hasura_claims.area_id.clone() else {
        return Err((Status::Unauthorized, "Missing area_id".into()));
    };

    // Check election id checks
    if claims.hasura_claims.authorized_election_ids.is_none()
        || !claims
            .hasura_claims
            .authorized_election_ids
            .as_ref()
            .unwrap_or(&Vec::new())
            .contains(election_id)
    {
        return Err((
            Status::Unauthorized,
            "Not authorized to election".into(),
        ));
    }

    match claims.azp.as_str() {
        "voting-portal" => Ok((area_id, VotingStatusChannel::ONLINE)),
        "voting-portal-kiosk" => Ok((area_id, VotingStatusChannel::KIOSK)),
        _ => Err((Status::Unauthorized, "Unknown Client".into())),
    }
}

/// Returns 400 for malformed ids and 404 unless the election belongs to the
/// election event of the given tenant.
pub async fn ensure_election_in_event(
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
) -> Result<(), (Status, String)> {
    for (value, field) in [
        (election_event_id, "election_event_id"),
        (election_id, "election_id"),
    ] {
        parse_uuid_v4_field(value, field)
            .map_err(|error| (Status::BadRequest, error.to_string()))?;
    }
    let mut hasura_db_client: DbClient =
        get_hasura_pool().await.get().await.map_err(|error| {
            (
                Status::InternalServerError,
                format!("Error obtaining hasura client: {error:?}"),
            )
        })?;
    let hasura_transaction =
        hasura_db_client.transaction().await.map_err(|error| {
            (
                Status::InternalServerError,
                format!("Error obtaining hasura transaction: {error:?}"),
            )
        })?;
    get_election_by_id(
        &hasura_transaction,
        tenant_id,
        election_event_id,
        election_id,
    )
    .await
    .map_err(|error| {
        (
            Status::InternalServerError,
            format!("Error getting election: {error:?}"),
        )
    })?
    .ok_or_else(|| (Status::NotFound, "Election not found".to_string()))?;
    Ok(())
}

#[cfg(test)]
mod ensure_election_in_event_tests {
    use super::*;
    use uuid::Uuid;

    /// Malformed ids are a client error, rejected before any database access.
    #[tokio::test]
    async fn ensure_election_in_event_rejects_malformed_ids() {
        let valid_id = Uuid::new_v4().to_string();
        assert_eq!(
            ensure_election_in_event(&valid_id, "event", &valid_id)
                .await
                .unwrap_err()
                .0,
            Status::BadRequest
        );
        assert_eq!(
            ensure_election_in_event(&valid_id, &valid_id, "election")
                .await
                .unwrap_err()
                .0,
            Status::BadRequest
        );
    }
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
mod permission_name_tests {
    use super::*;

    #[test]
    fn a_reserved_permission_name_is_a_bad_request() {
        let (status, message) =
            require_ordinary_permission("service-account").unwrap_err();

        assert_eq!(status, Status::BadRequest);
        assert!(message.contains("service-account"), "{message}");
    }

    #[test]
    fn an_ordinary_permission_name_is_accepted() {
        assert_eq!(require_ordinary_permission("election-event-read"), Ok(()));
    }
}

#[cfg(test)]
pub(crate) mod test_claims {
    use sequent_core::services::jwt::JwtClaims;

    pub const SUPER_ADMIN_TENANT_ID: &str = "super-admin-tenant";

    /// Admin portal claims for a tenant with the given Hasura roles, with the
    /// super admin tenant set.
    pub fn admin(tenant_id: &str, roles: &[String]) -> JwtClaims {
        std::env::set_var("SUPER_ADMIN_TENANT_ID", SUPER_ADMIN_TENANT_ID);
        serde_json::from_value(serde_json::json!({
            "exp": 1, "iat": 0, "jti": "test", "iss": "test",
            "sub": "user", "typ": "Bearer", "azp": "admin-portal",
            "acr": "1", "allowed-origins": [], "scope": "openid",
            "email_verified": false,
            "https://hasura.io/jwt/claims": {
                "x-hasura-default-role": "admin-user",
                "x-hasura-tenant-id": tenant_id,
                "x-hasura-user-id": "admin",
                "x-hasura-allowed-roles": roles
            }
        }))
        .expect("valid test claims")
    }

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
