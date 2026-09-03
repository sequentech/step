// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::{types::hasura_types::*, utils::read_config::read_config};
use graphql_client::{GraphQLQuery, Response};

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/get_keys_ceremony.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct GetKeysCeremony;

pub fn get_keys_ceremony_status(
    election_event_id: &str,
    key_ceremony_id: &str,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let config = read_config()?;
    let client = reqwest::blocking::Client::new();
    let variables = get_keys_ceremony::Variables {
        id: key_ceremony_id.to_string(),
        election_event_id: election_event_id.to_string(),
        tenant_id: config.tenant_id.clone(),
    };

    let request_body = GetKeysCeremony::build_query(variables);

    let response = client
        .post(&config.endpoint_url)
        .bearer_auth(config.auth_token)
        .json(&request_body)
        .send()?;

    if response.status().is_success() {
        let response_body: Response<get_keys_ceremony::ResponseData> = response.json()?;
        if let Some(data) = response_body.data {
            Ok(data
                .sequent_backend_keys_ceremony_by_pk
                .and_then(|k| k.execution_status))
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
