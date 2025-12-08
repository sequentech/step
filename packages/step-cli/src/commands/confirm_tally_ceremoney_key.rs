// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::{
    types::hasura_types::*,
    utils::{read_config::read_config, trustees::store_private_key::get_private_key_content},
};
use clap::Args;
use graphql_client::{GraphQLQuery, Response};

#[derive(Args)]
#[command(about = "Confirm trustee key for tally ceremony", long_about = None)]
pub struct ConfirmKeyForTally {
    /// Election event id - the election event to start the key ceremony for
    #[arg(long)]
    election_event_id: String,

    /// Tally Cremony Id - the tally ceremony to be associated with
    #[arg(long)]
    tally_id: String,
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/restore_private_key.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct RestorePrivateKey;

impl ConfirmKeyForTally {
    pub fn run(&self) {
        match confirm_key(&self.election_event_id, &self.tally_id) {
            Ok(is_valid) => {
                if is_valid {
                    println!("Success! Successfully confirmed key");
                } else {
                    eprintln!("Error! Failed to confirm key")
                }
            }
            Err(err) => {
                eprintln!("Failed to confirm key: {}", err)
            }
        }
    }
}

pub fn confirm_key(
    election_event_id: &str,
    tally_id: &str,
) -> Result<bool, Box<dyn std::error::Error>> {
    let config = read_config()?;
    let client = reqwest::blocking::Client::new();

    let key = get_private_key_content(&election_event_id, &config.username)?;

    let variables = restore_private_key::Variables {
        election_event_id: election_event_id.to_string(),
        tally_session_id: tally_id.to_string(),
        private_key_base64: key,
    };

    let request_body = RestorePrivateKey::build_query(variables);

    let response = client
        .post(&config.endpoint_url)
        .bearer_auth(config.auth_token)
        .json(&request_body)
        .send()?;

    if response.status().is_success() {
        let response_body: Response<restore_private_key::ResponseData> = response.json()?;
        if let Some(data) = response_body.data {
            if let Some(e) = data.restore_private_key {
                Ok(e.is_valid)
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
