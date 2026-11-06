// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

pub mod check_private_key;
pub mod get;
pub mod get_ceremony_status;
pub mod get_trustee_private_key;
pub mod store_private_key;

use reqwest::blocking::{Client, RequestBuilder};
use sequent_core::services::jwt::decode_jwt;
use sequent_core::types::permissions::Permissions;

const HASURA_ROLE_HEADER: &str = "x-hasura-role";

/// Starts a GraphQL request that runs under the trustee ceremony role.
pub fn trustee_graphql_request(
    client: &Client,
    endpoint_url: &str,
    auth_token: &str,
) -> RequestBuilder {
    client.post(endpoint_url).bearer_auth(auth_token).header(
        HASURA_ROLE_HEADER,
        Permissions::TRUSTEE_CEREMONY.to_string(),
    )
}

/// The role of a ceremony read shared by trustees and admins: a trustee's
/// token doesn't allow its default `admin-user` role, so it names its own. An
/// admin's request keeps the default role.
fn ceremony_read_role(auth_token: &str) -> Option<Permissions> {
    let allowed_roles = decode_jwt(auth_token).ok()?.hasura_claims.allowed_roles;
    let holds = |role: Permissions| allowed_roles.contains(&role.to_string());
    (!holds(Permissions::ADMIN_USER) && holds(Permissions::TRUSTEE_CEREMONY))
        .then_some(Permissions::TRUSTEE_CEREMONY)
}

/// Starts a GraphQL read of ceremony data for a trustee or an admin.
pub fn ceremony_read_request(
    client: &Client,
    endpoint_url: &str,
    auth_token: &str,
) -> RequestBuilder {
    let request = client.post(endpoint_url).bearer_auth(auth_token);
    match ceremony_read_role(auth_token) {
        Some(role) => request.header(HASURA_ROLE_HEADER, role.to_string()),
        None => request,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
    use serde_json::json;

    fn token_with_roles(roles: &[Permissions]) -> String {
        let roles: Vec<String> = roles.iter().map(ToString::to_string).collect();
        let claims = json!({
            "exp": 0, "iat": 0, "jti": "test", "iss": "test", "sub": "user",
            "typ": "Bearer", "azp": "client", "acr": "1", "allowed-origins": [],
            "scope": "openid", "email_verified": true,
            "https://hasura.io/jwt/claims": {
                "x-hasura-default-role": "admin-user", "x-hasura-tenant-id": "test",
                "x-hasura-user-id": "user", "x-hasura-allowed-roles": roles
            }
        });
        format!(
            "header.{}.signature",
            URL_SAFE_NO_PAD.encode(claims.to_string())
        )
    }

    fn ceremony_read_role_header(token: &str) -> Option<String> {
        ceremony_read_request(&Client::new(), "http://graphql-engine/v1/graphql", token)
            .build()
            .expect("the request should build")
            .headers()
            .get(HASURA_ROLE_HEADER)
            .and_then(|value| value.to_str().ok())
            .map(str::to_string)
    }

    #[test]
    fn a_trustees_ceremony_read_names_its_role() {
        let token = token_with_roles(&[Permissions::TRUSTEE_CEREMONY]);
        assert_eq!(
            ceremony_read_role_header(&token),
            Some(Permissions::TRUSTEE_CEREMONY.to_string())
        );
    }

    #[test]
    fn an_admins_ceremony_read_keeps_the_default_role() {
        let token = token_with_roles(&[Permissions::ADMIN_USER, Permissions::TRUSTEE_CEREMONY]);
        assert_eq!(ceremony_read_role_header(&token), None);
    }

    #[test]
    fn trustee_requests_run_as_trustee_ceremony() {
        let request =
            trustee_graphql_request(&Client::new(), "http://graphql-engine/v1/graphql", "token")
                .build()
                .expect("the request should build");

        let role = request
            .headers()
            .get(HASURA_ROLE_HEADER)
            .and_then(|value| value.to_str().ok());
        assert_eq!(
            role,
            Some(Permissions::TRUSTEE_CEREMONY.to_string().as_str())
        );
    }
}
