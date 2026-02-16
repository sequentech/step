// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::{types::hasura_types::*, utils::read_config::read_config};
use clap::Args;
use colored::Colorize;
use graphql_client::{GraphQLQuery, Response};

#[derive(Args)]
#[command(about = "Submit tie-break decision for IRV tally", long_about = None)]
pub struct SubmitTieBreak {
    /// Election event id
    #[arg(long)]
    election_event_id: String,

    /// Tally session id
    #[arg(long)]
    tally_id: String,

    /// Selected candidate id to win the tie
    #[arg(long)]
    candidate_id: String,
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/submit_tie_break.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct SubmitTieBreakMutation;

impl SubmitTieBreak {
    pub fn run(&self) {
        match submit_tie_break(
            &self.election_event_id,
            &self.tally_id,
            &self.candidate_id,
        ) {
            Ok(_) => {
                println!(
                    "{}",
                    format!(
                        "Success! Tie-break decision submitted. Selected candidate: {}",
                        self.candidate_id
                    )
                    .green()
                );
            }
            Err(err) => {
                eprintln!("Error! Failed to submit tie-break: {}", err)
            }
        }
    }
}

pub fn submit_tie_break(
    election_event_id: &str,
    tally_id: &str,
    candidate_id: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let config = read_config()?;
    let client = reqwest::blocking::Client::new();

    let variables = submit_tie_break_mutation::Variables {
        election_event_id: election_event_id.to_string(),
        tally_session_id: tally_id.to_string(),
        selected_candidate_id: candidate_id.to_string(),
    };

    let request_body = SubmitTieBreakMutation::build_query(variables);

    let response = client
        .post(&config.endpoint_url)
        .bearer_auth(config.auth_token)
        .json(&request_body)
        .send()?;

    if response.status().is_success() {
        let response_body: Response<submit_tie_break_mutation::ResponseData> = response.json()?;
        if let Some(data) = response_body.data {
            if let Some(result) = data.submit_tie_break_decision {
                Ok(result.tally_session_id)
            } else {
                Err(Box::from("failed submitting tie-break decision"))
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
