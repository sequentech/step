// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::{
    types::{hasura_types::*, tally::TallyExecutionStatus},
    utils::read_config::read_config,
};
use clap::Args;
use graphql_client::{GraphQLQuery, Response};
use std::str::FromStr;

#[derive(Args)]
#[command(about = "Update tally status", long_about = None)]
pub struct UpdateTallyStatus {
    /// Election event id - the election event to be associated with
    #[arg(long)]
    election_event_id: String,

    /// Tally Cremony Id - the tally ceremony to be associated with
    #[arg(long)]
    tally_id: String,

    /// Status - the desired status
    #[arg(long)]
    status: String,
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/update_tally_ceremony.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct UpdateTallyCeremony;

impl UpdateTallyStatus {
    pub fn run(&self) {
        match update_status(&self.election_event_id, &self.tally_id, &self.status) {
            Ok(id) => {
                println!(
                    "Success! Successfully updated status to {} for tally: {}",
                    &self.status, id
                );
            }
            Err(err) => {
                eprintln!("Error! Failed to update status: {}", err)
            }
        }
    }
}

pub fn update_status(
    election_event_id: &str,
    tally_id: &str,
    status: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let config = read_config()?;
    let client = reqwest::blocking::Client::new();

    // Validate status
    TallyExecutionStatus::from_str(status).map_err(|_| format!("Invalid status: {}", status))?;

    let variables = update_tally_ceremony::Variables {
        election_event_id: election_event_id.to_string(),
        tally_session_id: tally_id.to_string(),
        status: status.to_string(),
    };

    let request_body = UpdateTallyCeremony::build_query(variables);

    let response = client
        .post(&config.endpoint_url)
        .bearer_auth(config.auth_token)
        .json(&request_body)
        .send()?;

    if response.status().is_success() {
        let response_body: Response<update_tally_ceremony::ResponseData> = response.json()?;
        if let Some(data) = response_body.data {
            if let Some(e) = data.update_tally_ceremony {
                Ok(e.tally_session_id)
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
