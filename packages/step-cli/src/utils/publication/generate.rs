// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::{types::hasura_types::*, utils::read_config::read_config};
use graphql_client::{GraphQLQuery, Response};

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/generate_ballot_publication.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]

pub struct GenerateBallotPublication;

impl GenerateBallotPublication {
    pub fn generate(election_event_id: &str) -> Result<String, Box<dyn std::error::Error>> {
        let config = read_config()?;
        let client = reqwest::blocking::Client::new();

        let variables = generate_ballot_publication::Variables {
            election_event_id: election_event_id.to_string(),
            election_id: None,
        };

        let request_body = GenerateBallotPublication::build_query(variables);

        let response = client
            .post(&config.endpoint_url)
            .bearer_auth(config.auth_token)
            .json(&request_body)
            .send()?;

        if response.status().is_success() {
            let response_body: Response<generate_ballot_publication::ResponseData> =
                response.json()?;
            if let Some(data) = response_body.data {
                if let Some(e) = data.generate_ballot_publication {
                    Ok(e.ballot_publication_id)
                } else {
                    Err(Box::from("failed generating publication"))
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
