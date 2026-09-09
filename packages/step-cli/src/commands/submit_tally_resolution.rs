// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::{types::hasura_types::*, utils::read_config::read_config};
use clap::Args;
use colored::Colorize;
use graphql_client::{GraphQLQuery, Response};
use sequent_core::types::ceremonies::TallyResolution;

#[derive(Args)]
#[command(about = "Submit tally resolutions", long_about = None)]
pub struct SubmitTallyResolution {
    /// Election event id
    #[arg(long)]
    election_event_id: String,

    /// Tally session id
    #[arg(long)]
    tally_id: String,

    /// Tally resolutions in format: contest_id:candidate_id (can be repeated)
    #[arg(long = "resolution", value_name = "CONTEST_ID:CANDIDATE_ID")]
    resolutions: Vec<String>,
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/submit_tally_resolution.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct SubmitTallyResolutionMutation;

impl SubmitTallyResolution {
    pub fn run(&self) {
        // Parse resolutions
        let mut resolutions = Vec::new();
        for res_str in &self.resolutions {
            let parts: Vec<&str> = res_str.split(':').collect();
            if parts.len() != 2 {
                eprintln!(
                    "Error! Invalid resolution format: {}. Expected format: contest_id:candidate_id",
                    res_str
                );
                return;
            }
            resolutions.push(TallyResolution {
                contest_id: parts[0].to_string(),
                selected_candidate_id: parts[1].to_string(),
            });
        }

        if resolutions.is_empty() {
            eprintln!("Error! At least one resolution is required. Use --resolution CONTEST_ID:CANDIDATE_ID");
            return;
        }

        match submit_tally_resolution(&self.election_event_id, &self.tally_id, resolutions) {
            Ok(count) => {
                println!(
                    "{}",
                    format!("Success! {} tally resolution(s) submitted.", count).green()
                );
            }
            Err(err) => {
                eprintln!("Error! Failed to submit resolutions: {}", err)
            }
        }
    }
}

pub fn submit_tally_resolution(
    election_event_id: &str,
    tally_id: &str,
    resolutions: Vec<TallyResolution>,
) -> Result<usize, Box<dyn std::error::Error>> {
    let config = read_config()?;
    let client = reqwest::blocking::Client::new();

    let variables = submit_tally_resolution_mutation::Variables {
        election_event_id: election_event_id.to_string(),
        tally_session_id: tally_id.to_string(),
        resolutions: resolutions
            .into_iter()
            .map(|r| submit_tally_resolution_mutation::TallyResolutionInput {
                contest_id: r.contest_id,
                selected_candidate_id: r.selected_candidate_id,
            })
            .collect(),
    };

    let request_body = SubmitTallyResolutionMutation::build_query(variables);

    let response = client
        .post(&config.endpoint_url)
        .bearer_auth(config.auth_token)
        .json(&request_body)
        .send()?;

    if response.status().is_success() {
        let response_body: Response<submit_tally_resolution_mutation::ResponseData> =
            response.json()?;
        if let Some(data) = response_body.data {
            if let Some(result) = data.submit_tally_resolution {
                Ok(result.resolved_count as usize)
            } else {
                Err(Box::from("failed submitting tally resolutions"))
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
