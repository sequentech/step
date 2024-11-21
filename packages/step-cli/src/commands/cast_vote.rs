// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::{types::hasura_types::*, utils::read_config::read_config};
use clap::Args;
use graphql_client::{GraphQLQuery, Response};

#[derive(Args)]
#[command(about = "Cast a vote", long_about = None)]
pub struct CastVote {
    /// Election id - the election to cast a vote for
    #[arg(long)]
    election_id: String,
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/insert_cast_vote.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct InsertCastVote;

impl CastVote {
    pub fn run(&self) {
        match cast_vote(&self.election_id) {
            Ok(id) => {
                println!("Success! Vote cast! ID: {}", id);
            }
            Err(err) => {
                eprintln!("Error! Failed to cast vote: {}", err)
            }
        }
    }
}

fn cast_vote(election_id: &str) -> Result<String, Box<dyn std::error::Error>> {
    let config = read_config()?;
    let client = reqwest::blocking::Client::new();

    let variables = insert_cast_vote::Variables {
        election_id: election_id.to_string(),
        ballot_id: String::new(),
        content: String::new(),
    };

    let request_body = InsertCastVote::build_query(variables);

    let response = client
        .post(&config.endpoint_url)
        .bearer_auth(config.auth_token)
        .json(&request_body)
        .send()?;

    if response.status().is_success() {
        let response_body: Response<insert_cast_vote::ResponseData> = response.json()?;
        if let Some(data) = response_body.data {
            if let Some(e) = data.insert_cast_vote {
                Ok(e.id)
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
