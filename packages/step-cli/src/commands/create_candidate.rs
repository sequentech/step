// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::{types::hasura_types::*, utils::read_config::read_config};
use clap::Args;
use graphql_client::{GraphQLQuery, Response};

#[derive(Args)]
#[command(about = "Create a new candidate", long_about = None)]
pub struct CreateCandidate {
    /// Name of the candidate
    #[arg(long)]
    name: String,

    /// Description of the candidate
    #[arg(long, default_value = "")]
    description: String,

    /// Election event id - the election event to be associated with
    #[arg(long)]
    election_event_id: String,

    /// Contest id - the contest to be associated with
    #[arg(long)]
    contest_id: String,
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/insert_candidate.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct InsertCandidate;

impl CreateCandidate {
    pub fn run(&self) {
        match create_candidate(
            &self.name,
            &self.description,
            &self.election_event_id,
            &self.contest_id,
        ) {
            Ok(id) => {
                println!("Success! Candidate created successfully! ID: {}", id);
            }
            Err(err) => {
                eprintln!("Error! Failed to create candidate: {}", err)
            }
        }
    }
}

fn create_candidate(
    name: &str,
    description: &str,
    election_event_id: &str,
    contest_id: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let config = read_config()?;
    let client = reqwest::blocking::Client::new();

    let variables = insert_candidate::Variables {
        name: name.to_string(),
        description: Some(description.to_string()),
        election_event_id: election_event_id.to_string(),
        contest_id: contest_id.to_string(),
        tenant_id: config.tenant_id.clone(),

        presentation: None,
    };

    let request_body = InsertCandidate::build_query(variables);

    let response = client
        .post(&config.endpoint_url)
        .bearer_auth(config.auth_token)
        .json(&request_body)
        .send()?;

    if response.status().is_success() {
        let response_body: Response<insert_candidate::ResponseData> = response.json()?;
        if let Some(data) = response_body.data {
            if let Some(e) = data.insert_sequent_backend_candidate {
                Ok(e.returning[0].id.clone())
            } else {
                Err(Box::from("failed generating id"))
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
