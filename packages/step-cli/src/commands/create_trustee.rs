// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::types::hasura_types::*;
use crate::utils::read_config::read_config;
use clap::Args;
use colored::Colorize;
use graphql_client::{GraphQLQuery, Response};

#[derive(Args)]
#[command(about = "Register a native trustee's two public keys in the current tenant", long_about = None)]
pub struct CreateTrustee {
    /// The trustee's name (matches the runner's TRUSTEE_NAME).
    #[arg(long)]
    name: String,

    /// Public key - the trustee's public key, base64-encoded
    #[arg(long)]
    public_key: String,
    /// Canonical base64 share encryption public key from trustee public-keys.
    #[arg(long)]
    share_encryption_public_key: String,
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/create_trustee.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct CreateTrusteeMutation;

impl CreateTrustee {
    /// Execute the command, preserving failures for shell automation.
    pub fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        match create_trustee(
            &self.name,
            &self.public_key,
            &self.share_encryption_public_key,
        ) {
            Ok(id) => {
                println!(
                    "{} {}",
                    "Success! Registered trustee. ID:".green(),
                    id.cyan()
                );
            }
            Err(err) => return Err(err),
        }
        Ok(())
    }
}

fn create_trustee(
    name: &str,
    public_key: &str,
    share_encryption_public_key: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    protocol_board::Committee::new(&[protocol_board::RawTrusteeRecord {
        name: name.to_owned(),
        signing_public_key: Some(public_key.to_owned()),
        share_encryption_public_key: Some(share_encryption_public_key.to_owned()),
    }])?;
    let config = read_config()?;
    let client = reqwest::blocking::Client::new();

    let variables = create_trustee_mutation::Variables {
        name: name.to_string(),
        public_key: public_key.to_string(),
        share_encryption_public_key: share_encryption_public_key.to_string(),
        tenant_id: config.tenant_id.clone(),
    };

    let request_body = CreateTrusteeMutation::build_query(variables);

    let response = client
        .post(&config.endpoint_url)
        .bearer_auth(config.auth_token)
        .json(&request_body)
        .send()?;

    if response.status().is_success() {
        let response_body: Response<create_trustee_mutation::ResponseData> = response.json()?;
        if let Some(data) = response_body.data {
            let Some(output) = data.insert_sequent_backend_trustee_one else {
                return Err(Box::from("failed registering trustee"));
            };
            Ok(output.id)
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
