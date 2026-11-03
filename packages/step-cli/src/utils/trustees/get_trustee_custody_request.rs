// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::{
    types::hasura_types::*,
    utils::{
        read_config::read_config,
        trustees::{
            get_trustee_custody_request::{self, get_custody_request::GetCustodyRequestInput},
            trustee_graphql_request,
        },
    },
};
use graphql_client::{GraphQLQuery, Response};
use serde_json::Value;
use std::error::Error;
use windmill::services::ceremonies::keys_ceremony::CustodyRequestDownloadUnavailable;

// The code Harvest answers with once the key can no longer be downloaded
const CUSTODY_REQUEST_DOWNLOAD_UNAVAILABLE_CODE: &str = "CustodyRequestDownloadUnavailable";

/// Whether a GraphQL error says the key can no longer be downloaded. Hasura
/// promotes Harvest's code when it can parse the answer, and otherwise keeps
/// the answer as text in the error's internal response body.
fn is_download_unavailable(error: &Value) -> bool {
    let has_code = |value: &Value| {
        value.pointer("/extensions/code").and_then(Value::as_str)
            == Some(CUSTODY_REQUEST_DOWNLOAD_UNAVAILABLE_CODE)
    };

    has_code(error)
        || error
            .pointer("/extensions/internal/response/body")
            .and_then(Value::as_str)
            .and_then(|body| serde_json::from_str::<Value>(body).ok())
            .is_some_and(|body| has_code(&body))
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/get_custody_request.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]

pub struct GetCustodyRequest;

pub struct GetTrusteeCustodyRequest;

impl GetTrusteeCustodyRequest {
    pub fn get_trustee_custody_request(
        election_event_id: &str,
        key_ceremony_id: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let config = read_config()?;
        let client = reqwest::blocking::Client::new();

        let variables = get_custody_request::Variables {
            object: GetCustodyRequestInput {
                election_event_id: election_event_id.to_string(),
                keys_ceremony_id: key_ceremony_id.to_string(),
            },
        };

        let request_body = GetCustodyRequest::build_query(variables);

        let response = trustee_graphql_request(&client, &config.endpoint_url, &config.auth_token)
            .json(&request_body)
            .send()?;

        if response.status().is_success() {
            // Check for graphql errors - as it returns 200 when the action has errors
            let json: Value = response.json().map_err(|err| {
                Box::<dyn Error>::from(format!("Error parsing JSON response: {:?}", err))
            })?;
            if let Some(errors) = json.get("errors").and_then(Value::as_array) {
                if errors.iter().any(is_download_unavailable) {
                    return Err(Box::new(CustodyRequestDownloadUnavailable));
                }

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
            let response_body: Response<get_custody_request::ResponseData> =
                serde_json::from_value(json)?;
            if let Some(data) = response_body.data {
                if let Some(e) = data.get_custody_request {
                    Ok(e.custody_request_json)
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
