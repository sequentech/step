// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::{
    types::hasura_types::*,
    utils::{read_config::read_config, upload_file::GetUploadUrl},
};
use clap::Args;
use colored::Colorize;
use graphql_client::{GraphQLQuery, Response};
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::Read,
    thread::sleep,
    time::{Duration, Instant},
};

#[derive(Args)]
#[command(
    about = "Bulk-import voters into an election event from a CSV/TSV file",
    long_about = None
)]
pub struct ImportVoters {
    /// Election event id - the election event to import the voters into
    #[arg(long)]
    election_event_id: String,

    /// Path of the voters file - .csv or .tsv, same column shape `generate-voters` produces
    #[arg(long)]
    file_path: String,

    #[arg(long, default_value_t = false)]
    is_local: bool,
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/import_users.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct ImportUsers;

impl ImportVoters {
    pub fn run(&self) {
        match import_voters(&self.election_event_id, &self.file_path, self.is_local) {
            Ok(()) => {
                println!("{}", "Success! Voters imported successfully!".green());
            }
            Err(err) => {
                eprintln!("Error! Failed to import voters: {}", err)
            }
        }
    }
}

fn sha256_file(path: &str) -> Result<String, Box<dyn std::error::Error>> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 8192];

    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    Ok(hex::encode(hasher.finalize()))
}

pub fn import_voters(
    election_event_id: &str,
    file_path: &str,
    is_local: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = read_config()?;
    let client = reqwest::blocking::Client::new();

    let sha256 = sha256_file(file_path)?;
    let document_id = GetUploadUrl::upload_for_election_event(
        file_path.to_string(),
        is_local,
        Some(election_event_id.to_string()),
    )?;

    let variables = import_users::Variables {
        tenant_id: config.tenant_id.clone(),
        document_id,
        election_event_id: Some(election_event_id.to_string()),
        sha256: Some(sha256),
    };

    let request_body = ImportUsers::build_query(variables);

    let response = client
        .post(&config.endpoint_url)
        .bearer_auth(config.auth_token)
        .json(&request_body)
        .send()?;

    let response_body: Response<import_users::ResponseData> =
        response.json().map_err(|e| format!("{:?}", e))?;

    let task_execution_id = match (response_body.data, response_body.errors) {
        (Some(data), _) => {
            let output = data.import_users.ok_or("failed starting import task")?;
            output.task_execution.id
        }
        (None, Some(errors)) => {
            let messages = errors
                .into_iter()
                .map(|e| e.message)
                .collect::<Vec<_>>()
                .join(", ");
            return Err(messages.into());
        }
        _ => return Err("Unknown error: empty data and no GraphQL errors".into()),
    };

    let start_time = Instant::now();
    let timeout = Duration::from_secs(300);
    let polling_interval = Duration::from_secs(3);

    loop {
        match crate::utils::tasks::get_task_status(&task_execution_id) {
            Ok(status) if status == "SUCCESS" => return Ok(()),
            Ok(status) if status == "FAILED" => return Err("Import voters task failed".into()),
            Ok(_) => {
                if Instant::now().duration_since(start_time) >= timeout {
                    return Err("Timeout while waiting for import voters task to complete".into());
                }
                sleep(polling_interval);
            }
            Err(e) => return Err(format!("Error checking task status: {}", e).into()),
        }
    }
}
