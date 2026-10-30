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

pub use sequent_core::services::authorization::*;

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
