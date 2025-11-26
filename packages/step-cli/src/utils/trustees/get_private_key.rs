// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::error::Error;

use crate::{types::hasura_types::*, utils::read_config::read_config};
use graphql_client::{GraphQLQuery, Response};
use serde_json::Value;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/get_private_key.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]

pub struct GetPrivateKey;

impl GetPrivateKey {
    pub fn get_trustee_private_key(
        election_event_id: &str,
        key_ceremony_id: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let config = read_config()?;
        let client = reqwest::blocking::Client::new();

        let variables = get_private_key::Variables {
            election_event_id: election_event_id.to_string(),
            keys_ceremony_id: key_ceremony_id.to_string(),
        };

        let request_body = GetPrivateKey::build_query(variables);

        let response = client
            .post(&config.endpoint_url)
            .bearer_auth(config.auth_token)
            .json(&request_body)
            .send()?;

        if response.status().is_success() {
            // Check for graphql errors - as it returns 200 when the action has errors
            let json: Value = response.json().map_err(|err| {
                Box::<dyn Error>::from(format!("Error parsing JSON response: {:?}", err))
            })?;
            if let Some(errors) = json.get("errors").and_then(Value::as_array) {
                let error_statuses: Vec<String> = errors
                    .iter()
                    .filter_map(|e| {
                        e.get("extensions")
                            .and_then(|ext| ext.get("internal"))
                            .and_then(|internal| internal.get("response"))
                            .and_then(|resp| resp.get("status"))
                            .and_then(Value::as_u64)
                            .map(|s| s.to_string())
                    })
                    .collect();

                if !error_statuses.is_empty() {
                    return Err(Box::from(format!("Status {}", error_statuses.join(", "))));
                }
            }

            // Continue normally
            let response_body: Response<get_private_key::ResponseData> =
                serde_json::from_value(json)?;
            if let Some(data) = response_body.data {
                if let Some(e) = data.get_private_key {
                    Ok(e.private_key_base64)
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
