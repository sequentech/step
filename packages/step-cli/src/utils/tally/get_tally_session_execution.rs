// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::{types::hasura_types::*, utils::read_config::read_config};
use ::uuid::Uuid;
use graphql_client::{GraphQLQuery, Response};

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/get_tally_session_execution.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct GetTallySessionExecution;

pub fn get_tally_session_execution(
    tally_session_id: &str,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let config = read_config()?;
    let client = reqwest::blocking::Client::new();
    let variables = get_tally_session_execution::Variables {
        tally_session_id: Uuid::parse_str(tally_session_id)?.to_string(),
    };

    let request_body = GetTallySessionExecution::build_query(variables);

    let response = client
        .post(&config.endpoint_url)
        .bearer_auth(config.auth_token)
        .json(&request_body)
        .send()?;

    if response.status().is_success() {
        let response_body: Response<get_tally_session_execution::ResponseData> = response.json()?;
        if let Some(data) = response_body.data {
            if let Some(execution) = data.sequent_backend_tally_session_execution.first() {
                Ok(execution.results_event_id.clone())
            } else {
                Err(Box::from("No tally session execution found"))
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
