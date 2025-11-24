// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::{
    types::hasura_types::*,
    utils::{read_config::read_config, trustees::get::GetTrustees},
};
use clap::Args;
use graphql_client::{GraphQLQuery, Response};

#[derive(Args)]
#[command(about = "Start Key Ceremony", long_about = None)]
pub struct StartKeyCeremony {
    /// Election event id - the election event to start the key ceremony for
    #[arg(long)]
    election_event_id: String,

    /// Threshold - the minimum number of trustees required to tally
    #[arg(long, default_value_t = 2)]
    threshold: i64,
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/create_keys_ceremony.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct CreateKeysCeremony;

impl StartKeyCeremony {
    pub fn run(&self) {
        match start_ceremony(&self.election_event_id, self.threshold) {
            Ok(id) => {
                println!("Success! Successfully started key ceremony. ID: {}", id);
            }
            Err(err) => {
                eprintln!("Error! Failed to start key ceremony: {}", err)
            }
        }
    }
}

pub fn start_ceremony(
    election_event_id: &str,
    threshold: i64,
) -> Result<String, Box<dyn std::error::Error>> {
    let config = read_config()?;
    let client = reqwest::blocking::Client::new();

    let trustees = GetTrustees::get_names()?;
    let variables = create_keys_ceremony::Variables {
        election_event_id: election_event_id.to_string(),
        threshold,
        trustee_names: Some(trustees),
    };

    let request_body = CreateKeysCeremony::build_query(variables);

    let response = client
        .post(&config.endpoint_url)
        .bearer_auth(config.auth_token)
        .json(&request_body)
        .send()?;

    if response.status().is_success() {
        let response_body: Response<create_keys_ceremony::ResponseData> = response.json()?;
        if let Some(data) = response_body.data {
            if let Some(e) = data.create_keys_ceremony {
                Ok(e.keys_ceremony_id)
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
