// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::{types::hasura_types::*, utils::read_config::read_config};
use clap::Args;
use colored::Colorize;
use graphql_client::{GraphQLQuery, Response};
use sequent_core::ballot::{VotingStatus, VotingStatusChannel};
use update_event_voting_status::{
    VotingStatus as CliVotingStatus, VotingStatusChannel as CliVotingStatusChannel,
};

impl From<VotingStatus> for CliVotingStatus {
    fn from(v: VotingStatus) -> Self {
        match v {
            VotingStatus::OPEN => CliVotingStatus::OPEN,
            VotingStatus::CLOSED => CliVotingStatus::CLOSED,
            VotingStatus::PAUSED => CliVotingStatus::PAUSED,
            VotingStatus::NOT_STARTED => CliVotingStatus::NOT_STARTED,
        }
    }
}

impl From<VotingStatusChannel> for CliVotingStatusChannel {
    fn from(v: VotingStatusChannel) -> Self {
        match v {
            VotingStatusChannel::ONLINE => CliVotingStatusChannel::ONLINE,
            VotingStatusChannel::KIOSK => CliVotingStatusChannel::KIOSK,
            VotingStatusChannel::EARLY_VOTING => CliVotingStatusChannel::EARLY_VOTING,
            VotingStatusChannel::TELEPHONE => CliVotingStatusChannel::TELEPHONE,
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

    #[arg(long)]
    voting_channel: Option<VotingStatusChannel>,
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
        match update_event_voting_status(
            &self.election_event_id,
            &self.voting_status,
            &self.voting_channel,
        ) {
            Ok(Some(id)) => {
                println!(
                    "{} {}",
                    "Success! Updated successfully! ID:".green(),
                    id.cyan()
                );
            }
            Ok(None) => {
                eprintln!(
                    "Error! Failed to update election event: {} ",
                    self.election_event_id
                );
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
    voting_channel: &Option<VotingStatusChannel>,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let config = read_config()?;

    let client = reqwest::blocking::Client::new();

    let voting_channels: Option<Vec<Option<CliVotingStatusChannel>>> =
        voting_channel.map(|c| vec![Some(c.into())]);

    let variables = update_event_voting_status::Variables {
        election_event_id: election_event_id.to_string(),
        voting_status: (*voting_status).into(),
        voting_channels,
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
