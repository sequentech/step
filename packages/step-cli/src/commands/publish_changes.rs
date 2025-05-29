// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::{
    thread::sleep,
    time::{Duration, Instant},
};

use crate::{
    types::hasura_types::*,
    utils::{
        publication::{generate::GenerateBallotPublication, get::GetBallotPublicationStatus},
        read_config::read_config,
    },
};
use clap::Args;
use graphql_client::{GraphQLQuery, Response};

#[derive(Args)]
#[command(about = "Publish election event ballot changes", long_about = None)]
pub struct PublishChanges {
    #[arg(long)]
    election_event_id: String,

    #[arg(long)]
    election_id: Option<String>,
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/publish_ballot.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct PublishBallot;

impl PublishChanges {
    pub fn run(&self) {
        match publish_changes(
            &self.election_event_id,
            self.election_id.as_ref().map(String::as_str),
        ) {
            Ok(id) => {
                println!("Success! Published successfully! ID: {}", id);
            }
            Err(err) => {
                eprintln!("Error! Failed to publish: {}", err)
            }
        }
    }
}

pub fn publish_changes(
    election_event_id: &str,
    election_id: Option<&str>,
) -> Result<String, Box<dyn std::error::Error>> {
    let config = read_config()?;

    let client = reqwest::blocking::Client::new();

    let ballot_publication_id =
        GenerateBallotPublication::generate(election_event_id, election_id)?;
    println!("Ballot Publication ID: {}", ballot_publication_id);

    // Wait for publication to generate
    // Poll for the publication to be available
    let start_time = Instant::now();
    let timeout = Duration::from_secs(60); // Set a timeout of 60 seconds
    let polling_interval = Duration::from_secs(3); // Poll every 3 seconds

    loop {
        match GetBallotPublicationStatus::get(&ballot_publication_id) {
            Ok(is_generated) => {
                if is_generated {
                    break;
                } else {
                    // Publication is not yet generated, retry after interval
                    if Instant::now().duration_since(start_time) >= timeout {
                        return Err(Box::from(
                            "Timeout while waiting for publication to be available",
                        ));
                    }
                    sleep(polling_interval);
                }
            }
            Err(e) => {
                // Error occurred while checking publication
                return Err(e);
            }
        }
    }

    let variables = publish_ballot::Variables {
        election_event_id: election_event_id.to_string(),
        ballot_publication_id,
    };

    let request_body = PublishBallot::build_query(variables);

    let response = client
        .post(&config.endpoint_url)
        .bearer_auth(config.auth_token)
        .json(&request_body)
        .send()?;

    if response.status().is_success() {
        let response_body: Response<publish_ballot::ResponseData> = response.json()?;
        if let Some(data) = response_body.data {
            if let Some(e) = data.publish_ballot {
                Ok(e.ballot_publication_id)
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
