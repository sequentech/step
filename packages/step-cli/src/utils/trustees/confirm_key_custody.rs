// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::{
    types::hasura_types::*,
    utils::{read_config::read_config, trustees::trustee_graphql_request},
};
use graphql_client::{GraphQLQuery, Response};

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/confirm_key_custody.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]

pub struct ConfirmKeyCustody;

impl ConfirmKeyCustody {
    pub fn check(
        election_event_id: &str,
        key_ceremony_id: &str,
        custody_proof_json: &str,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let config = read_config()?;
        let client = reqwest::blocking::Client::new();

        let variables = confirm_key_custody::Variables {
            election_event_id: election_event_id.to_string(),
            keys_ceremony_id: key_ceremony_id.to_string(),
            custody_proof_json: custody_proof_json.to_string(),
        };

        let request_body = ConfirmKeyCustody::build_query(variables);

        let response = trustee_graphql_request(&client, &config.endpoint_url, &config.auth_token)
            .json(&request_body)
            .send()?;

        if response.status().is_success() {
            let response_body: Response<confirm_key_custody::ResponseData> = response.json()?;
            if let Some(data) = response_body.data {
                if let Some(e) = data.confirm_key_custody {
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
