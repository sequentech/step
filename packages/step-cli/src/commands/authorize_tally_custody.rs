// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::{
    types::hasura_types::*,
    utils::{
        read_config::read_config,
        trustees::{store_custody_request::get_custody_request_content, trustee_graphql_request},
    },
};
use clap::Args;
use colored::Colorize;
use graphql_client::{GraphQLQuery, Response};

#[derive(Args)]
#[command(about = "Authorize trustee custody for a tally ceremony", long_about = None)]
pub struct AuthorizeTallyCustodyCommand {
    /// Election event id - the election event to start the key ceremony for
    #[arg(long)]
    election_event_id: String,

    /// Tally Cremony Id - the tally ceremony to be associated with
    #[arg(long)]
    tally_id: String,

    /// Native trustee keys file.
    #[arg(long)]
    trustee_config: std::path::PathBuf,
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/authorize_tally_custody.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct AuthorizeTallyCustody;

impl AuthorizeTallyCustodyCommand {
    pub fn run(&self) {
        match confirm_key_with_keys(
            &self.election_event_id,
            &self.tally_id,
            &self.trustee_config,
        ) {
            Ok(is_valid) => {
                if is_valid {
                    println!("{}", "Success! Successfully confirmed key".green());
                } else {
                    eprintln!("{}", "Error! Failed to confirm key".red())
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
    let keys_file = std::env::var_os("TRUSTEE_CONFIG_PATH")
        .ok_or("TRUSTEE_CONFIG_PATH must name your native trustee keys file")?;
    confirm_key_with_keys(
        election_event_id,
        tally_id,
        std::path::Path::new(&keys_file),
    )
}

fn confirm_key_with_keys(
    election_event_id: &str,
    tally_id: &str,
    keys_file: &std::path::Path,
) -> Result<bool, Box<dyn std::error::Error>> {
    let config = read_config()?;
    let client = reqwest::blocking::Client::new();

    let request = get_custody_request_content(election_event_id, &config.username)?;
    let mut custody: protocol_board::CustodyRequest = serde_json::from_str(&request)?;
    if custody.tenant_id != config.tenant_id
        || custody.election_event_id != election_event_id
        || custody.trustee_name != config.username
        || custody.tally_session_id.is_some()
    {
        return Err("custody request does not match this trustee and election event".into());
    }
    custody.tally_session_id = Some(tally_id.to_owned());
    let secrets = protocol_board::TrusteeSecrets::parse(&std::fs::read_to_string(keys_file)?)?;
    let key = custody.attest(&secrets)?;

    let variables = authorize_tally_custody::Variables {
        election_event_id: election_event_id.to_string(),
        tally_session_id: tally_id.to_string(),
        custody_proof_json: key,
    };

    let request_body = AuthorizeTallyCustody::build_query(variables);

    let response = trustee_graphql_request(&client, &config.endpoint_url, &config.auth_token)
        .json(&request_body)
        .send()?;

    if response.status().is_success() {
        let response_body: Response<authorize_tally_custody::ResponseData> = response.json()?;
        if let Some(data) = response_body.data {
            if let Some(e) = data.authorize_tally_custody {
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
