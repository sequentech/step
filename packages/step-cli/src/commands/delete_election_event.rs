// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::types::hasura_types::*;
use crate::utils::read_config::read_config;
use clap::Args;
use colored::Colorize;
use graphql_client::{GraphQLQuery, Response};
use std::{
    thread::sleep,
    time::{Duration, Instant},
};

#[derive(Args, Debug)]
#[command(about = "Delete an election event", long_about = None)]
pub struct DeleteElectionEventCLI {
    /// ID of the election event to delete
    #[arg(long)]
    election_event_id: String,
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/delete_election_event.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct DeleteElectionEvent;

impl DeleteElectionEventCLI {
    pub fn run(&self) {
        match delete_election_event(&self.election_event_id) {
            Ok(id) => {
                println!(
                    "{} {}",
                    "Success! Election event deleted successfully! ID:".green(),
                    id.cyan()
                );
            }
            Err(err) => {
                eprintln!("Error! Failed to delete election event: {}", err)
            }
        }
    }
}

fn wait_for_task(task_execution_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let start_time = Instant::now();
    let timeout = Duration::from_secs(300);
    let polling_interval = Duration::from_secs(3);

    loop {
        match crate::utils::tasks::get_task_status(task_execution_id) {
            Ok(status) if status == "SUCCESS" => return Ok(()),
            Ok(status) if status == "FAILED" => {
                return Err("Delete election event task failed".into())
            }
            Ok(_) => {
                if Instant::now().duration_since(start_time) >= timeout {
                    return Err(
                        "Timeout while waiting for delete election event task to complete".into(),
                    );
                }
                sleep(polling_interval);
            }
            Err(e) => return Err(format!("Error checking task status: {}", e).into()),
        }
    }
}

fn delete_election_event(election_event_id: &str) -> Result<String, Box<dyn std::error::Error>> {
    let config = read_config()?;
    let client = reqwest::blocking::Client::new();

    let variables = delete_election_event::Variables {
        election_event_id: election_event_id.to_string(),
    };

    let request_body = DeleteElectionEvent::build_query(variables);
    let response = client
        .post(&config.endpoint_url)
        .bearer_auth(config.auth_token)
        .json(&request_body)
        .send()?;

    if response.status().is_success() {
        let response_body: Response<delete_election_event::ResponseData> = response.json()?;
        if let Some(data) = response_body.data {
            if let Some(e) = data.delete_election_event {
                if let Some(err) = e.error_msg {
                    Err(Box::from(err))
                } else if let Some(id) = e.id {
                    // The mutation only enqueues the deletion; Hasura/Postgres
                    // rows, the Keycloak realm, ImmuDB entries, and document
                    // storage are torn down asynchronously by the matching
                    // celery task. Wait for it so the command doesn't report
                    // success before cleanup has actually finished.
                    if let Some(task_execution) = e.task_execution {
                        wait_for_task(&task_execution.id)?;
                    }
                    Ok(id)
                } else {
                    Err(Box::from("failed deleting election event"))
                }
            } else {
                Err(Box::from("failed deleting election event"))
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
