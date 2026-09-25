// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::ports::graphql::GraphqlClient;
use crate::utils::read_config::read_config;
use graphql_client::Response;
use serde::{de::DeserializeOwned, Serialize};
use std::error::Error;

/// The Hasura endpoint in the stored configuration. The configuration is read,
/// and the HTTP client built, for every request: refreshing the token rewrites
/// the configuration file between the requests of a polling command.
#[derive(Clone, Copy, Debug, Default)]
pub struct HasuraGraphql;

impl GraphqlClient for HasuraGraphql {
    fn post<T, B>(&self, request_body: &B) -> Result<Response<T>, Box<dyn Error>>
    where
        T: DeserializeOwned,
        B: Serialize + ?Sized,
    {
        let config = read_config()?;
        let client = reqwest::blocking::Client::new();
        let response = client
            .post(&config.endpoint_url)
            .bearer_auth(config.auth_token)
            .json(request_body)
            .send()?;

        if response.status().is_success() {
            Ok(response.json()?)
        } else {
            let status = response.status();
            let error_message = response.text()?;
            Err(Box::from(format!(
                "HTTP Status: {}\nError Message: {}",
                status, error_message
            )))
        }
    }
}
