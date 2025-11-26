// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::{types::hasura_types::*, utils::read_config::read_config};
use clap::Args;
use graphql_client::{GraphQLQuery, Response};
use sequent_core::ballot::VotingStatus;
use std::str::FromStr;

#[derive(Args)]
#[command(about = "Update election event status", long_about = None)]
pub struct UpdateElectionEventStatus {
    /// Election event id - the election event to be associated with
    #[arg(long)]
    election_event_id: String,

    /// Status - the desired status
    #[arg(long)]
    status: String,
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/update_election_event_voting_status.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct UpdateEventVotingStatus;

impl UpdateElectionEventStatus {
    pub fn run(&self) {
        match update_status(&self.election_event_id, &self.status) {
            Ok(id) => {
                println!("Success! Successfully updated status to {}", &self.status);
            }
            Err(err) => {
                eprintln!("Error! Failed to update status: {}", err)
            }
        }
    }
}

pub fn update_status(
    election_event_id: &str,
    status: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = read_config()?;
    let client = reqwest::blocking::Client::new();

    // Validate status
    VotingStatus::from_str(status).map_err(|_| format!("Invalid status: {}", status))?;

    let variables = update_event_voting_status::Variables {
        election_event_id: election_event_id.to_string(),
        status: status.to_string(),
    };

    let request_body = UpdateEventVotingStatus::build_query(variables);

    let response = client
        .post(&config.endpoint_url)
        .bearer_auth(config.auth_token)
        .json(&request_body)
        .send()?;

    if response.status().is_success() {
        let response_body: Response<update_event_voting_status::ResponseData> = response.json()?;
        if let Some(data) = response_body.data {
            if let Some(_) = data.update_event_voting_status {
                Ok(())
            } else {
                Err(Box::from("failed updating status"))
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
