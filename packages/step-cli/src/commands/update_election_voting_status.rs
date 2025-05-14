use std::str::FromStr;
use anyhow::{Context, Result, anyhow};
use crate::{utils::read_config::read_config, types::hasura_types::*};
use clap::Args;
use graphql_client::{GraphQLQuery, Response};

use update_election_voting_status::VotingStatus;

const VOTING_STATUS_OPEN: &str = "OPEN";
const VOTING_STATUS_CLOSE: &str = "CLOSE";
const VOTING_STATUS_PAUSE: &str = "PAUSE";

impl FromStr for update_election_voting_status::VotingStatus {
    type Err = String;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input {
            VOTING_STATUS_OPEN => Ok(VotingStatus::OPEN),
            VOTING_STATUS_CLOSE => Ok(VotingStatus::CLOSED),
            VOTING_STATUS_PAUSE => Ok(VotingStatus::PAUSED),
            _ => Err(format!(
                "Invalid voting status, status must be one of: {}, {}, {}",
                VOTING_STATUS_OPEN, VOTING_STATUS_CLOSE, VOTING_STATUS_PAUSE
            )),
        }
    }
}

/// Command for updating the voting status of an election
#[derive(Args)]
#[command(about = "Update election voting status", long_about = None)]
pub struct UpdateElectionVotingStatusCommand {
    /// The ID of the election event
    #[arg(long)]
    election_event_id: String,

    /// The ID of the election
    #[arg(long)]
    election_id: String,

    /// The new voting status (OPEN, CLOSE, or PAUSE)
    #[arg(long)]
    voting_status: VotingStatus,
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/update_election_voting_status.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct UpdateElectionVotingStatus;

impl UpdateElectionVotingStatusCommand {
    pub fn run(&self) {
        match update_election_voting_status(&self.election_event_id, &self.election_id, &self.voting_status) {
            Ok(id) => {
                println!("Success! Updated successfully! ID: {}", id.unwrap_or_else(|| "None".to_string()));
            }
            Err(err) => {
                eprintln!("Error! Failed to update: {}", err);
            }
        }
    }
}

/// Updates the voting status of an election
///
/// # Arguments
///
/// * `election_event_id` - The ID of the election event
/// * `election_id` - The ID of the election
/// * `voting_status` - The new voting status to set
///
/// # Returns
///
/// * `Ok(Option<String>)` - The election ID if successful
/// * `Err(anyhow::Error)` - An error if the update failed
pub fn update_election_voting_status(
    election_event_id: &str,
    election_id: &str,
    voting_status: &VotingStatus,
) -> Result<Option<String>> {
    let config = read_config().map_err(|e| anyhow::anyhow!("{}", e))?;

    let client = reqwest::blocking::Client::new();

    let variables = update_election_voting_status::Variables {
        election_event_id: election_event_id.to_string(),
        election_id: election_id.to_string(),
        voting_status: voting_status.clone(),
        voting_channels: None,
    };

    let request_body = UpdateElectionVotingStatus::build_query(variables);
    
    let response = client
        .post(&config.endpoint_url)
        .bearer_auth(config.auth_token)
        .json(&request_body)
        .send()
        .context("Failed to send request to update election voting status")?;

    if response.status().is_success() {
        let response_body: Response<update_election_voting_status::ResponseData> = response
            .json()
            .context("Failed to parse response JSON")?;
        println!("Response body: {:?}", response_body);
        if let Some(data) = response_body.data {
            if let Some(update_election_voting_status) = data.update_election_voting_status {
                Ok(update_election_voting_status.election_id)
            } else {
                Err(anyhow!("No data found in the response"))
            }
        } else if let Some(errors) = response_body.errors {
            println!("Errors: {:?}", errors);
            let error_messages: Vec<String> = errors.into_iter().map(|e| e.message).collect();
            Err(anyhow!("GraphQL errors: {}", error_messages.join(", ")))
        } else {
            Err(anyhow!("Unknown error occurred"))
        }
    } else {
        let status = response.status();
        let error_message = response
            .text()
            .context("Failed to read error response body")?;
        
        Err(anyhow!(
            "Request failed with status {}: {}",
            status,
            error_message
        ))
    }
}


