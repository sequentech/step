// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::{types::hasura_types::uuid, utils::read_config::read_config};
use graphql_client::{GraphQLQuery, Response};

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/get_ballot_publication_status.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
/// Get ballot publication status query
pub struct GetBallotPublicationStatus;

impl GetBallotPublicationStatus {
    /// Get ballot publication status
    pub fn get(ballot_publication_id: &str) -> Result<bool, Box<dyn std::error::Error>> {
        let config = read_config()?;
        let client = reqwest::blocking::Client::new();

        let variables = get_ballot_publication_status::Variables {
            id: ballot_publication_id.to_string(),
        };

        let request_body = GetBallotPublicationStatus::build_query(variables);

        let response = client
            .post(&config.endpoint_url)
            .bearer_auth(config.auth_token)
            .json(&request_body)
            .send()?;

        if response.status().is_success() {
            let response_body: Response<get_ballot_publication_status::ResponseData> =
                response.json()?;
            if let Some(data) = response_body.data {
                if let Some(ballot_publication) = data.sequent_backend_ballot_publication.first() {
                    Ok(ballot_publication.is_generated)
                } else {
                    Err(Box::from("No ballot publication found"))
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
            let error = format!("HTTP Status: {status}\nError Message: {error_message}");
            Err(Box::from(error))
        }
    }
}
