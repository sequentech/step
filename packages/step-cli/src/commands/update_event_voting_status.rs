

use std::str::FromStr;

use crate::{utils::read_config::read_config, types::hasura_types::*};
use clap::Args;
use graphql_client::{GraphQLQuery, Response};
use update_event_voting_status::VotingStatus;


impl FromStr for update_event_voting_status::VotingStatus {
    type Err = String;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input {
            "OPEN"   => Ok(VotingStatus::OPEN),
            "CLOSE" => Ok(VotingStatus::CLOSED),
            "PAUSE" => Ok(VotingStatus::PAUSED),
            // …and so on for every variant in your schema’s VotingStatus enum
            _ => Err(format!("Invalid voting status, status must be one of: OPEN, CLOSE, PAUSE")),
        }
    }
}

#[derive(Args)]
#[command(about = "Update election event voting status", long_about = None)]

pub struct UpdateElectionEventVotingStatus {
    #[arg(long)]
    election_event_id: String,

    #[arg(long)]
    voting_status: VotingStatus,
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/update_event_voting_status.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct UpdateEventVotingStatus;

impl UpdateElectionEventVotingStatus {
    pub fn run(&self) {
        match update_event_voting_status(&self.election_event_id, &self.voting_status) {
            Ok(id) => {
                println!("Success! Updated successfully! ID: {}", id.unwrap_or_else(|| "None".to_string()));
            }
            Err(err) => {
                eprintln!("Error! Failed to update: {}", err)
            }
        }
    }
}
pub fn update_event_voting_status(
    election_event_id: &str,
    voting_status: &VotingStatus,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let config = read_config()?;

    let client = reqwest::blocking::Client::new();

    let variables = update_event_voting_status::Variables {
        election_event_id: election_event_id.to_string(),
        voting_status: voting_status.clone(),
        voting_channel: None,
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
            if let Some(update_event_voting_status) = data.update_event_voting_status {
                Ok(update_event_voting_status.election_event_id)
            } else {
                Err(Box::from("No data found in the response"))
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


