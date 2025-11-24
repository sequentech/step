// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::{types::hasura_types::*, utils::read_config::read_config};
use graphql_client::{GraphQLQuery, Response};

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/check_private_key.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]

pub struct CheckPrivateKey;

impl CheckPrivateKey {
    pub fn check(
        election_event_id: &str,
        key_ceremony_id: &str,
        private_key_base64: &str,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let config = read_config()?;
        let client = reqwest::blocking::Client::new();

        let variables = check_private_key::Variables {
            election_event_id: election_event_id.to_string(),
            keys_ceremony_id: key_ceremony_id.to_string(),
            private_key_base64: private_key_base64.to_string(),
        };

        let request_body = CheckPrivateKey::build_query(variables);

        let response = client
            .post(&config.endpoint_url)
            .bearer_auth(config.auth_token)
            .json(&request_body)
            .send()?;

        if response.status().is_success() {
            let response_body: Response<check_private_key::ResponseData> = response.json()?;
            if let Some(data) = response_body.data {
                if let Some(e) = data.check_private_key {
                    Ok(e.is_valid)
                } else {
                    Err(Box::from("Failed to get key"))
                }
            } else if let Some(errors) = response_body.errors {
                let error_messages: Vec<String> = errors.into_iter().map(|e| e.message).collect();
                Err(Box::from(error_messages.join(", ")))
            } else {
                Err(Box::from("Unknown error occurred"))
            }
        } else {
            let status = response.status();
            let error_message = response.text()?;
            let error = format!("HTTP Status: {}\nError Message: {}", status, error_message);
            Err(Box::from(error))
        }
    }
}
