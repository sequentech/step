// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::{types::hasura_types::*, utils::read_config::read_config};
use clap::Args;
use graphql_client::{GraphQLQuery, Response};

#[derive(Args)]
#[command(about = "Create a new area", long_about = None)]
pub struct CreateArea {
    /// Name of the area
    #[arg(long)]
    name: String,

    /// Description of the area
    #[arg(long, default_value = "")]
    description: String,

    /// Election event id - the election event to be associated with
    #[arg(long)]
    election_event_id: String,
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/insert_area.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct InsertArea;

impl CreateArea {
    pub fn run(&self) {
        match create_area(&self.name, &self.description, &self.election_event_id) {
            Ok(id) => {
                println!("Success! Area created successfully! ID: {}", id);
            }
            Err(err) => {
                eprintln!("Error! Failed to create Area: {}", err)
            }
        }
    }
}

fn create_area(
    name: &str,
    description: &str,
    election_event_id: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let config = read_config()?;
    let client = reqwest::blocking::Client::new();

    let variables = insert_area::Variables {
        name: name.to_string(),
        description: Some(description.to_string()),
        election_event_id: election_event_id.to_string(),
        tenant_id: config.tenant_id.clone(),

        parent_id: None,
    };

    let request_body = InsertArea::build_query(variables);

    let response = client
        .post(&config.endpoint_url)
        .bearer_auth(config.auth_token)
        .json(&request_body)
        .send()?;

    if response.status().is_success() {
        let response_body: Response<insert_area::ResponseData> = response.json()?;
        if let Some(data) = response_body.data {
            if let Some(e) = data.insert_sequent_backend_area {
                Ok(e.returning[0].id.clone())
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
