// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::{types::hasura_types::*, utils::read_config::read_config};
use clap::Args;
use edit_user::EditUsersInput;
use graphql_client::{GraphQLQuery, Response};
use serde_json::{Map, Value};

#[derive(Args)]
#[command(about = "Edit a voter", long_about = None)]
pub struct UpdateVoter {
    /// Election event id - the election event to be associated with
    #[arg(long)]
    election_event_id: String,

    /// User Id
    #[arg(long)]
    user_id: String,

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

    /// User Password
    #[arg(long, default_value = "")]
    password: String,

    /// Area id - area associated to user
    #[arg(long, default_value = "")]
    area_id: String,

    /// mobile - user mobile_number
    #[arg(long, default_value = "")]
    mobile: String,
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/edit_user.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct EditUser;

impl UpdateVoter {
    pub fn run(&self) {
        match edit_voter(
            &self.election_event_id,
            &self.user_id,
            &self.first_name,
            &self.last_name,
            &self.username,
            &self.email,
            &self.password,
            &self.area_id,
            &self.mobile,
        ) {
            Ok(id) => {
                println!("Success! Voter updated successfully! ID: {}", id);
            }
            Err(err) => {
                eprintln!("Error! Failed to update voter: {}", err)
            }
        }
    }
}

pub fn edit_voter(
    election_event_id: &str,
    user_id: &str,
    first_name: &str,
    last_name: &str,
    username: &str,
    email: &str,
    password: &str,
    area_id: &str,
    mobile: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let config = read_config()?;
    let client = reqwest::blocking::Client::new();

    let mut attributes = Map::new();

    if !area_id.is_empty() {
        attributes.insert(
            "area-id".to_string(),
            Value::Array(vec![Value::String(area_id.to_string())]),
        );
    }

    if !mobile.is_empty() {
        attributes.insert(
            "sequent.read-only.mobile-number".to_string(),
            Value::Array(vec![Value::String(mobile.to_string())]),
        );
    }

    let attributes_value = if attributes.is_empty() {
        None
    } else {
        Some(Value::Object(attributes))
    };

    let variables = edit_user::Variables {
        body: EditUsersInput {
            tenant_id: config.tenant_id.clone(),
            user_id: user_id.to_string(),
            password: if password.is_empty() {
                None
            } else {
                Some(password.to_string())
            },
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
            attributes: attributes_value,
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
            enabled: Some(true),
            groups: None,
            election_event_id: Some(election_event_id.to_string()),
        },
    };

    let request_body = EditUser::build_query(variables);

    let response = client
        .post(&config.endpoint_url)
        .bearer_auth(config.auth_token)
        .json(&request_body)
        .send()?;

    if response.status().is_success() {
        let response_body: Response<edit_user::ResponseData> = response.json()?;
        if let Some(data) = response_body.data {
            let user_data = data.edit_user;
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
