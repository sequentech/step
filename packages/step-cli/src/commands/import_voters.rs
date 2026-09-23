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
use std::{
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
    /// Preserve import failures in the process exit status.
    pub fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        import_voters(&self.election_event_id, &self.file_path, self.is_local)?;
        println!("{}", "Success! Voters imported successfully!".green());
        Ok(())
    }
}

pub fn import_voters(
    election_event_id: &str,
    file_path: &str,
    is_local: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = read_config()?;
    let client = reqwest::blocking::Client::new();

    let document_id = GetUploadUrl::upload_for_election_event(
        file_path.to_string(),
        is_local,
        Some(election_event_id.to_string()),
    )?;

    let variables = import_users::Variables {
        tenant_id: config.tenant_id.clone(),
        document_id,
        election_event_id: Some(election_event_id.to_string()),
        sha256: None,
    };

    let request_body = ImportUsers::build_query(variables);

    let response = client
        .post(&config.endpoint_url)
        .bearer_auth(config.auth_token)
        .json(&request_body)
        .send()?
        .error_for_status()?;

    let response_body: Response<import_users::ResponseData> =
        response.json().map_err(|e| format!("{:?}", e))?;

    let task_execution_id = import_task_id(response_body)?;

    let start_time = Instant::now();
    let timeout = Duration::from_secs(300);
    let polling_interval = Duration::from_secs(3);

    loop {
        crate::utils::read_config::refresh_and_save_token()?;
        let status = crate::utils::tasks::get_task_status(&task_execution_id)?;
        if import_finished(&status, start_time.elapsed(), timeout)? {
            return Ok(());
        }
        sleep(polling_interval);
    }
}

/// Classify terminal task states before deciding whether another poll is useful.
fn import_finished(
    status: &str,
    elapsed: Duration,
    timeout: Duration,
) -> Result<bool, Box<dyn std::error::Error>> {
    match status {
        "SUCCESS" => Ok(true),
        "FAILED" => Err("Import voters task failed".into()),
        _ if elapsed >= timeout => Err("Timeout waiting for import voters".into()),
        _ => Ok(false),
    }
}

/// Preserve field-level GraphQL failures even when Hasura also returns partial data.
fn import_task_id(
    response: Response<import_users::ResponseData>,
) -> Result<String, Box<dyn std::error::Error>> {
    if let Some(errors) = response.errors.filter(|errors| !errors.is_empty()) {
        return Err(errors
            .into_iter()
            .map(|error| error.message)
            .collect::<Vec<_>>()
            .join(", ")
            .into());
    }
    response
        .data
        .and_then(|data| data.import_users)
        .map(|output| output.task_execution.id)
        .ok_or_else(|| "Import returned no task".into())
}

#[cfg(test)]
mod response_tests {
    use super::*;

    #[test]
    fn success_and_polling_terminal_states() {
        let response = serde_json::from_value(serde_json::json!({"data":{"import_users":{"task_execution":{"id":"task", "execution_status":"IN_PROGRESS"}}}})).unwrap();
        assert_eq!(import_task_id(response).unwrap(), "task");
        let deadline = Duration::from_secs(1);
        assert!(import_finished("SUCCESS", deadline, deadline).unwrap());
        assert!(import_finished("FAILED", Duration::ZERO, deadline).is_err());
        assert!(!import_finished("IN_PROGRESS", Duration::ZERO, deadline).unwrap());
        assert!(import_finished("IN_PROGRESS", deadline, deadline).is_err());
    }

    #[test]
    fn errors_survive_partial_or_absent_data() {
        for data in [
            serde_json::Value::Null,
            serde_json::json!({"import_users": null}),
        ] {
            let response = serde_json::from_value(
                serde_json::json!({"data": data, "errors": [{"message": "Import denied"}]}),
            )
            .unwrap();
            assert_eq!(
                import_task_id(response).unwrap_err().to_string(),
                "Import denied"
            );
        }
        assert!(
            import_task_id(serde_json::from_value(serde_json::json!({"data":null})).unwrap())
                .is_err()
        );
    }
}
