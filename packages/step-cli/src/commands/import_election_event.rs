// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::{
    types::hasura_types::*,
    utils::{read_config::read_config, upload_file::GetUploadUrl},
};
use clap::Args;
use graphql_client::{GraphQLQuery, Response};

#[derive(Args)]
#[command(about = "Import Election Event", long_about = None)]
pub struct ImportElectionEventFile {
    /// Path of Election Event file - .json file
    #[arg(long)]
    file_path: String,

    #[arg(long, default_value_t = false)]
    is_local: bool,
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/import_election_event.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct ImportElectionEvent;

impl ImportElectionEventFile {
    pub fn run(&self) {
        match import(&self.file_path, self.is_local) {
            Ok(id) => {
                println!("Success! Election event created successfully! ID: {}", id);
            }
            Err(err) => {
                eprintln!("Error! Failed to create election event: {}", err)
            }
        }
    }
}

pub fn import(file_path: &str, is_local: bool) -> Result<String, Box<dyn std::error::Error>> {
    let config = read_config()?;
    let client = reqwest::blocking::Client::new();
    let document_id = GetUploadUrl::upload(String::from(file_path), is_local)?;

    let variables = import_election_event::Variables {
        tenant_id: config.tenant_id.clone(),
        document_id,
        check_only: None,
    };

    let request_body = ImportElectionEvent::build_query(variables);
    let response = client
        .post(&config.endpoint_url)
        .bearer_auth(config.auth_token)
        .json(&request_body)
        .send()?;

    if response.status().is_success() {
        let response_body: Response<import_election_event::ResponseData> = response.json()?;
        if let Some(data) = response_body.data {
            if let Some(e) = data.import_election_event {
                if let Some(err) = e.error {
                    Err(Box::from(err))
                } else if let Some(id) = e.id {
                    Ok(id)
                } else {
                    Err(Box::from("failed generating id"))
                }
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
