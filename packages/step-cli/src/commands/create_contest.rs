// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::{types::hasura_types::*, utils::read_config::read_config};
use clap::Args;
use colored::Colorize;
use graphql_client::{GraphQLQuery, Response};
use sequent_core::{ballot::ContestPresentation, types::ceremonies::CountingAlgType};
use std::collections::HashMap;

#[derive(Args)]
#[command(about = "Create a new contest", long_about = None)]
pub struct CreateContest {
    /// Name of the contest
    #[arg(long)]
    name: String,

    /// Description of the contest
    #[arg(long, default_value = "")]
    description: String,

    /// Election event id - the election event to be associated with
    #[arg(long)]
    election_event_id: String,

    /// Election id - the election to be associated with
    #[arg(long)]
    election_id: String,

    /// Counting algorithm (optional)
    #[arg(long, default_value_t = CountingAlgType::PluralityAtLarge)]
    counting_algorithm: CountingAlgType,
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/insert_contest.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct InsertContest;

impl CreateContest {
    pub fn run(&self) {
        match create_contest(
            &self.name,
            &self.description,
            &self.election_event_id,
            &self.election_id,
            self.counting_algorithm,
        ) {
            Ok(id) => {
                println!(
                    "{} {}",
                    "Success! Contest created successfully! ID:".green(),
                    id.cyan()
                );
            }
            Err(err) => {
                eprintln!("Error! Failed to create contest: {}", err)
            }
        }
    }
}

fn create_contest(
    name: &str,
    description: &str,
    election_event_id: &str,
    election_id: &str,
    counting_algorithm: CountingAlgType,
) -> Result<String, Box<dyn std::error::Error>> {
    let config = read_config()?;
    let client = reqwest::blocking::Client::new();

    let mut presentation = ContestPresentation::default();

    presentation.i18n = Some(HashMap::from([(
        "en".to_string(),
        HashMap::from([("name".to_string(), Some(name.to_string()))]),
    )]));

    let variables = insert_contest::Variables {
        description: Some(description.to_string()),
        election_event_id: election_event_id.to_string(),
        election_id: election_id.to_string(),
        tenant_id: config.tenant_id.clone(),

        counting_algorithm: Some(counting_algorithm.to_string()),
        presentation: Some(serde_json::to_value(presentation)?),
        max_votes: Some(1),
        min_votes: Some(0),
        winning_candidates_num: Some(1),
        is_acclaimed: Some(false),
        is_active: Some(true),
        is_encrypted: Some(true),
    };

    let request_body = InsertContest::build_query(variables);

    let response = client
        .post(&config.endpoint_url)
        .bearer_auth(config.auth_token)
        .json(&request_body)
        .send()?;

    if response.status().is_success() {
        let response_body: Response<insert_contest::ResponseData> = response.json()?;
        if let Some(data) = response_body.data {
            if let Some(e) = data.insert_sequent_backend_contest {
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
