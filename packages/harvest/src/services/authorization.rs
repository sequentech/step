// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::Context;
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use rocket::response::status::Unauthorized;
use sequent_core::ballot::{VotingStatus, VotingStatusChannel};
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::{Permissions, VoterPermissions};
use std::collections::HashSet;
use std::env;
use tracing::{error, info, instrument};
use windmill::postgres::election::get_election_by_id;
use windmill::services::database::get_hasura_pool;

pub use sequent_core::services::authorization::*;

pub async fn ensure_election_in_event(
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
) -> Result<(), (Status, String)> {
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
pub mod test_claims {
    use sequent_core::services::jwt::JwtClaims;

    pub const SUPER_ADMIN_TENANT_ID: &str = "super-admin-tenant";

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
