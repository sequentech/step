// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

pub mod check_private_key;
pub mod get;
pub mod get_ceremony_status;
pub mod get_trustee_private_key;
pub mod store_private_key;

use reqwest::blocking::{Client, RequestBuilder};
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

#[cfg(test)]
mod tests {
    use super::*;

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
