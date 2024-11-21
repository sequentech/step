// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::{types::hasura_types::*, utils::read_config::read_config};
use graphql_client::{GraphQLQuery, Response};

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/get_trustees.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]

pub struct GetTrustees;

impl GetTrustees {
    pub fn get_names() -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let config = read_config()?;
        let client = reqwest::blocking::Client::new();

        let variables = get_trustees::Variables {
            tenant_id: config.tenant_id.to_string(),
        };

        let request_body = GetTrustees::build_query(variables);

        let response = client
            .post(&config.endpoint_url)
            .bearer_auth(config.auth_token)
            .json(&request_body)
            .send()?;

        if response.status().is_success() {
            let response_body: Response<get_trustees::ResponseData> = response.json()?;
            if let Some(data) = response_body.data {
                let names: Vec<String> = data
                    .sequent_backend_trustee
                    .iter()
                    .filter_map(|trustee| trustee.name.clone())
                    .collect();
                Ok(names)
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
}
