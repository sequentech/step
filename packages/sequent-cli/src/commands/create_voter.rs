// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::{types::hasura_types::*, utils::read_config::read_config};
use clap::Args;
use create_user::KeycloakUser2;
use graphql_client::{GraphQLQuery, Response};

#[derive(Args)]
#[command(about = "Create a new voter", long_about = None)]
pub struct CreateVoter {
    /// Election event id - the election event to be associated with
    #[arg(long)]
    election_event_id: String,

    /// User first name
    #[arg(long, default_value = "")]
    first_name: String,

    /// User last name
    #[arg(long, default_value = "")]
    last_name: String,

    /// User username
    #[arg(long, default_value = "")]
    username: String,

    /// User Email
    #[arg(long, default_value = "")]
    email: String,
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/create_user.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct CreateUser;

impl CreateVoter {
    pub fn run(&self) {
        match create_voter(
            &self.election_event_id,
            &self.first_name,
            &self.last_name,
            &self.username,
            &self.email,
        ) {
            Ok(id) => {
                println!("Success! Voter created successfully! ID: {}", id);
            }
            Err(err) => {
                eprintln!("Error! Failed to create voter: {}", err)
            }
        }
    }
}

pub fn create_voter(
    election_event_id: &str,
    first_name: &str,
    last_name: &str,
    username: &str,
    email: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let config = read_config()?;
    let client = reqwest::blocking::Client::new();

    let variables = create_user::Variables {
        tenant_id: config.tenant_id.clone(),
        election_event_id: Some(election_event_id.to_string()),
        user: KeycloakUser2 {
            first_name: if first_name.is_empty() {
                None
            } else {
                Some(first_name.to_string())
            },
            last_name: if last_name.is_empty() {
                None
            } else {
                Some(last_name.to_string())
            },
            attributes: None,
            email: if email.is_empty() {
                None
            } else {
                Some(email.to_string())
            },
            username: if username.is_empty() {
                None
            } else {
                Some(username.to_string())
            },
            email_verified: None,
            enabled: Some(true),
            groups: None,
            id: None,
        },
    };

    let request_body = CreateUser::build_query(variables);

    let response = client
        .post(&config.endpoint_url)
        .bearer_auth(config.auth_token)
        .json(&request_body)
        .send()?;

    if response.status().is_success() {
        let response_body: Response<create_user::ResponseData> = response.json()?;
        if let Some(data) = response_body.data {
            let user_data = data.create_user;
            if let Some(id) = user_data.id {
                Ok(id)
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
