// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
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

pub use sequent_core::services::authorization::*;

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

    /// Builds claims for the given client, with the super admin tenant set.
    fn claims(azp: &str, hasura_claims: serde_json::Value) -> JwtClaims {
        std::env::set_var("SUPER_ADMIN_TENANT_ID", SUPER_ADMIN_TENANT_ID);
        serde_json::from_value(serde_json::json!({
            "exp": 1, "iat": 0, "jti": "test", "iss": "test",
            "sub": "user", "typ": "Bearer", "azp": azp,
            "acr": "1", "allowed-origins": [], "scope": "openid",
            "email_verified": false,
            "https://hasura.io/jwt/claims": hasura_claims
        }))
        .expect("valid test claims")
    }

    /// Admin portal claims for a tenant with the given Hasura roles.
    pub fn admin(tenant_id: &str, roles: &[String]) -> JwtClaims {
        claims(
            "admin-portal",
            serde_json::json!({
                "x-hasura-default-role": "admin-user",
                "x-hasura-tenant-id": tenant_id,
                "x-hasura-user-id": "admin",
                "x-hasura-allowed-roles": roles
            }),
        )
    }

    /// Voting portal claims for one election event and its authorized
    /// elections.
    pub fn voter(election_event_id: &str, election_ids: &[&str]) -> JwtClaims {
        claims(
            "voting-portal",
            serde_json::json!({
                "x-hasura-default-role": "user",
                "x-hasura-tenant-id": "tenant",
                "x-hasura-user-id": "voter",
                "x-hasura-area-id": "area",
                "x-hasura-election-event-id": election_event_id,
                "authorized-election-ids": election_ids,
                "x-hasura-allowed-roles": ["user"]
            }),
        )
    }
}
