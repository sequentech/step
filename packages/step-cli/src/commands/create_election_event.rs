// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::types::hasura_types::*;
use crate::utils::read_config::read_config;
use clap::Args;
use graphql_client::{GraphQLQuery, Response};
use serde_json::Value;

#[derive(Args, Debug)]
#[command(about = "Create a new election event", long_about = None)]
pub struct CreateElectionEventCLI {
    /// Name of the election event
    #[arg(long)]
    name: String,

    /// Description of the election event
    #[arg(long, default_value = "")]
    description: String,

    /// Encryption protocol (currently hardcoded to RSA256)
    #[arg(long, default_value = "RSA256")]
    encryption_protocol: String,

    /// Whether the event is archived
    #[arg(long, default_value_t = false)]
    is_archived: bool,
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/insert_election_event.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct CreateElectionEvent;

impl CreateElectionEventCLI {
    pub fn run(&self) {
        match create_election_event(
            &self.name,
            &self.description,
            &self.encryption_protocol,
            self.is_archived,
        ) {
            Ok(id) => {
                println!(
                    "Success! Election event created successfully! ID: {}",
                    id.unwrap_or_else(|| "None".to_string())
                );
            }
            Err(err) => {
                eprintln!("Error! Failed to create election event: {}", err)
            }
        }
    }
}

fn create_election_event(
    name: &str,
    description: &str,
    encryption_protocol: &str,
    is_archived: bool,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let config = read_config()?;
    let client = reqwest::blocking::Client::new();

    let variables = create_election_event::Variables {
        election_event: create_election_event::CreateElectionEventInput {
            tenant_id: config.tenant_id.clone(),
            name: name.to_string(),
            description: Some(description.to_string()),
            encryption_protocol: Some(encryption_protocol.to_string()),
            is_archived: Some(is_archived),

            id: None,
            presentation: None,
            created_at: None,
            updated_at: None,
            labels: None,
            annotations: None,
            bulletin_board_reference: None,
            voting_channels: None,
            dates: None,
            status: None,
            user_boards: None,
            is_audit: None,
            audit_election_event_id: None,
            public_key: None,
        },
    };

    let request_body = CreateElectionEvent::build_query(variables);

    let response = client
        .post(&config.endpoint_url)
        .bearer_auth(config.auth_token)
        .json(&request_body)
        .send()?;

    if response.status().is_success() {
        let response_body: Response<create_election_event::ResponseData> = response.json()?;
        if let Some(data) = response_body.data {
            if let Some(event) = data.insert_election_event {
                Ok(event.id)
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
