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
use sequent_core::types::permissions::{Permissions, VoterPermissions};
use std::collections::HashSet;
use std::env;
use tracing::{error, info, instrument};
use windmill::postgres::election::get_election_by_id;
use windmill::services::database::get_hasura_pool;

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

#[cfg(test)]
pub mod test_claims {
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
}
