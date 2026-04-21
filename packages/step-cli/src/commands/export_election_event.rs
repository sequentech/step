// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::types::hasura_types::uuid;
use crate::utils::read_config::read_config;
use clap::Args;
use colored::Colorize;
use graphql_client::{GraphQLQuery, Response};
use sequent_core::types::permissions::Permissions;
use serde_json::Value;
use std::thread::sleep;
use std::time::{Duration, Instant};
use tracing::{error, info};
#[derive(Args, Debug)]
#[command(about = "Export an election event", long_about = None)]
/// Export an election event command arguments
#[allow(clippy::struct_excessive_bools)]
pub struct ExportElectionEventCommand {
    /// Election event id
    #[arg(long)]
    election_event_id: String,

    /// Include voters
    #[arg(long, default_value_t = false)]
    include_voters: bool,

    /// Include activity logs
    #[arg(long, default_value_t = false)]
    activity_logs: bool,

    /// Include bulletin board
    #[arg(long, default_value_t = false)]
    bulletin_board: bool,

    /// Include publications
    #[arg(long, default_value_t = false)]
    publications: bool,

    /// Include S3 files
    #[arg(long, default_value_t = false)]
    s3_files: bool,

    /// Include scheduled events
    #[arg(long, default_value_t = false)]
    scheduled_events: bool,

    /// Include reports
    #[arg(long, default_value_t = false)]
    reports: bool,

    /// Include applications
    #[arg(long, default_value_t = false)]
    applications: bool,

    /// Include tally
    #[arg(long, default_value_t = false)]
    tally: bool,

    /// Encrypt the exported package
    #[arg(long, default_value_t = false)]
    encrypted: bool,

    /// Output directory
    #[arg(long, default_value = "./data")]
    output_dir: String,
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/export_election_event.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
/// Export election event query
pub struct ExportElectionEvent;

impl ExportElectionEventCommand {
    /// Execute the export election event command
    pub fn run(&self) {
        match export_election_event(
            &self.election_event_id,
            &self.output_dir,
            self.include_voters,
            self.activity_logs,
            self.bulletin_board,
            self.publications,
            self.s3_files,
            self.scheduled_events,
            self.reports,
            self.applications,
            self.tally,
            self.encrypted,
        ) {
            Ok(()) => {
                info!(
                    "{}",
                    "Success! Election event exported successfully!".green()
                );
            }
            Err(err) => {
                error!("Error! Failed to export election event: {err}");
            }
        }
    }
}

/// Export election event
#[allow(clippy::too_many_arguments, clippy::fn_params_excessive_bools)]
pub fn export_election_event(
    election_event_id: &str,
    output_dir: &str,
    include_voters: bool,
    activity_logs: bool,
    bulletin_board: bool,
    publications: bool,
    s3_files: bool,
    scheduled_events: bool,
    reports: bool,
    applications: bool,
    tally: bool,
    encrypted: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = read_config()?;
    let client = reqwest::blocking::Client::new();

    if tally && !bulletin_board {
        return Err("Error: bulletin_board must be exported when tally is exported".into());
    }

    let is_encrypted = bulletin_board || reports || applications || tally || encrypted;

    let variables = export_election_event::Variables {
        election_event_id: Some(election_event_id.to_string()),
        export_configurations: Some(export_election_event::ExportOptions {
            password: None,
            is_encrypted: Some(is_encrypted),
            include_voters: Some(include_voters),
            activity_logs: Some(activity_logs),
            bulletin_board: Some(bulletin_board),
            publications: Some(publications),
            s3_files: Some(s3_files),
            scheduled_events: Some(scheduled_events),
            reports: Some(reports),
            applications: Some(applications),
            tally: Some(tally),
        }),
    };

    let request_body = ExportElectionEvent::build_query(variables);

    let response = client
        .post(&config.endpoint_url)
        .bearer_auth(config.auth_token)
        .header(
            "x-hasura-role",
            Permissions::ELECTION_EVENT_READ.to_string(),
        )
        .json(&request_body)
        .send()?;

    let response_body: graphql_client::Response<export_election_event::ResponseData> =
        response.json().map_err(|e| format!("{e:?}"))?;

    match (response_body.data, response_body.errors) {
        (Some(data), _) => {
            let export_data = data
                .export_election_event
                .ok_or("No export data returned")?;

            let document_id = export_data.document_id;
            let task_execution = export_data.task_execution;

            // Wait for task to complete using polling
            let start_time = Instant::now();
            let timeout = Duration::from_secs(60); // 60 second timeout
            let polling_interval = Duration::from_secs(3); // Poll every 3 seconds

            loop {
                match crate::utils::tasks::get_task_status(&task_execution.id) {
                    Ok(status) => {
                        if status == "SUCCESS" {
                            break;
                        } else if status == "FAILED" {
                            return Err("Export task failed".into());
                        }
                        // Task is still running, check timeout
                        if Instant::now().duration_since(start_time) >= timeout {
                            return Err("Timeout while waiting for export task to complete".into());
                        }
                        sleep(polling_interval);
                    }
                    Err(e) => {
                        return Err(format!("Error checking task status: {e}").into());
                    }
                }
            }

            // Now download the document
            let document = crate::utils::tally::download_document::fetch_document(
                election_event_id,
                &document_id,
            )?;
            let extension = if is_encrypted { "ezip" } else { "zip" };
            let output_path = format!("{output_dir}/election_event_export.{extension}");
            crate::utils::tally::download_document::download_file(&document.url, &output_path)?;

            if let Some(password) = export_data.password {
                info!("🔑  Export password: {password}");
            }

            Ok(())
        }
        (None, Some(errors)) => {
            let messages = errors
                .into_iter()
                .map(|e| e.message)
                .collect::<Vec<_>>()
                .join(", ");
            Err(messages.into())
        }
        _ => Err("Unknown error: empty data and no GraphQL errors".into()),
    }
}
