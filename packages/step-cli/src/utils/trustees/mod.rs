// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

pub mod check_private_key;
pub mod get;
pub mod get_private_key;
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

/// Whether a request shared with admins must name `role`: a restricted token
/// doesn't allow its default `admin-user` role, so it names the one it holds.
/// An admin's request keeps the default role.
fn must_name_role(auth_token: &str, role: &Permissions) -> bool {
    decode_jwt(auth_token).is_ok_and(|claims| {
        let allowed_roles = claims.hasura_claims.allowed_roles;
        !allowed_roles.contains(&Permissions::ADMIN_USER.to_string())
            && allowed_roles.contains(&role.to_string())
    })
}

/// Starts a GraphQL request shared by admins and restricted users holding `role`.
pub fn held_role_request(
    client: &Client,
    endpoint_url: &str,
    auth_token: &str,
    role: Permissions,
) -> RequestBuilder {
    let request = client.post(endpoint_url).bearer_auth(auth_token);
    if must_name_role(auth_token, &role) {
        request.header(HASURA_ROLE_HEADER, role.to_string())
    } else {
        request
    }
}

/// Starts a GraphQL read of ceremony data for a trustee or an admin.
pub fn ceremony_read_request(
    client: &Client,
    endpoint_url: &str,
    auth_token: &str,
) -> RequestBuilder {
    held_role_request(
        client,
        endpoint_url,
        auth_token,
        Permissions::TRUSTEE_CEREMONY,
    )
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

    fn publication_role_header(token: &str) -> Option<String> {
        held_role_request(
            &Client::new(),
            "http://graphql-engine/v1/graphql",
            token,
            Permissions::PUBLISH_WRITE,
        )
        .build()
        .expect("the request should build")
        .headers()
        .get(HASURA_ROLE_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string)
    }

    #[test]
    fn a_lockdown_users_publication_request_names_its_role() {
        let token = token_with_roles(&[Permissions::PUBLISH_READ, Permissions::PUBLISH_WRITE]);
        assert_eq!(
            publication_role_header(&token),
            Some(Permissions::PUBLISH_WRITE.to_string())
        );
    }

    #[test]
    fn an_admins_publication_request_keeps_the_default_role() {
        let token = token_with_roles(&[Permissions::ADMIN_USER, Permissions::PUBLISH_WRITE]);
        assert_eq!(publication_role_header(&token), None);
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
