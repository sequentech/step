// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::{
    types::hasura_types::*,
    utils::{elections::get::GetElections, read_config::read_config},
};
use clap::Args;
use graphql_client::{GraphQLQuery, Response};

#[derive(Args)]
#[command(about = "Start Tally Ceremony", long_about = None)]
pub struct StartTallyCeremony {
    /// Election event id - the election event to start the key ceremony for
    #[arg(long)]
    election_event_id: String,

    /// Election ids- optional specific elections to start the tally for - if not provided - all elections will be tallied
    #[arg(long)]
    election_ids: Option<Vec<String>>,

    /// Tally type - the type of tally to perform (ELECTORAL_RESULTS or INITIALIZATION_REPORT)
    #[arg(long)]
    tally_type: String,
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/create_tally_ceremony.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct CreateTallyCeremony;

impl StartTallyCeremony {
    pub fn run(&self) {
        match start_ceremony(&self.election_event_id, self.election_ids.clone(), &self.tally_type) {
            Ok(id) => {
                println!("Success! Successfully started Tally ceremony. ID: {}", id);
            }
            Err(err) => {
                eprintln!("Error! Failed to start key ceremony: {}", err)
            }
        }
    }
}

pub fn start_ceremony(
    election_event_id: &str,
    election_ids: Option<Vec<String>>,
    tally_type: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let config = read_config()?;
    let client = reqwest::blocking::Client::new();

    let elections = match election_ids {
        Some(el) => el,
        None => GetElections::get_by_election_event(&election_event_id)?,
    };

    let variables = create_tally_ceremony::Variables {
        election_event_id: election_event_id.to_string(),
        election_ids: elections,
        configuration: None,
        tally_type: Some(tally_type.to_string()),
    };

    let request_body = CreateTallyCeremony::build_query(variables);

    let response = client
        .post(&config.endpoint_url)
        .bearer_auth(config.auth_token)
        .json(&request_body)
        .send()?;

    if response.status().is_success() {
        let response_body: Response<create_tally_ceremony::ResponseData> = response.json()?;
        if let Some(data) = response_body.data {
            if let Some(e) = data.create_tally_ceremony {
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
