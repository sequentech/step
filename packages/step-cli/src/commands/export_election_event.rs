// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::types::hasura_types::*;
use crate::utils::read_config::read_config;
use clap::Args;
use graphql_client::{GraphQLQuery, Response};
use serde_json::Value;

#[derive(Args, Debug)]
#[command(about = "Export an election event", long_about = None)]
pub struct ExportElectionEventCommand {
    /// ID of the election event to export
    #[arg(long)]
    election_event_id: String,

    /// Whether to include voters in the export
    #[arg(long, default_value_t = false)]
    include_voters: bool,

    /// Whether to include activity logs in the export
    #[arg(long, default_value_t = false)]
    activity_logs: bool,

    /// Whether to include bulletin board in the export
    #[arg(long, default_value_t = false)]
    bulletin_board: bool,

    /// Whether to include publications in the export
    #[arg(long, default_value_t = false)]
    publications: bool,

    /// Whether to include S3 files in the export
    #[arg(long, default_value_t = false)]
    s3_files: bool,

    /// Whether to include scheduled events in the export
    #[arg(long, default_value_t = false)]
    scheduled_events: bool,

    /// Whether to include reports in the export
    #[arg(long, default_value_t = false)]
    reports: bool,

    /// Whether to include applications in the export
    #[arg(long, default_value_t = false)]
    applications: bool,

    /// Whether to include tally in the export
    #[arg(long, default_value_t = false)]
    tally: bool,

    /// Output directory for downloaded files
    #[arg(long, default_value = "output")]
    output_dir: String,
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/export_election_event.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct ExportElectionEvent;

impl ExportElectionEventCommand {
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
        ) {
            Ok(_) => {
                println!("Success! Election event exported successfully!");
            }
            Err(err) => {
                eprintln!("Error! Failed to export election event: {}", err)
            }
        }
    }
}

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
) -> Result<(), Box<dyn std::error::Error>> {
    let config = read_config()?;
    let client = reqwest::blocking::Client::new();
    
    let is_encrypted = bulletin_board || reports || applications;

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
        .json(&request_body)
        .send()?;

    let response_body: graphql_client::Response<export_election_event::ResponseData> = response.json()?;

    println!("response_body: {:?}", response_body);
    Ok(())

    // match (response_body.data, response_body.errors) {
    //     (Some(data), _) => {
    //         let export_data = data
    //             .export_election_event
    //             .ok_or("No export data returned")?;

    //         let document_id = export_data.document_id;
          
    //         let document = crate::utils::tally::download_document::fetch_document(
    //             election_event_id,
    //             &document_id,
    //         )?;

    //         let output_path = format!("{}/election_event_export.zip", output_dir);
    //         crate::utils::tally::download_document::download_file(&document.url, &output_path)?;

    //         if let Some(password) = export_data.password {
    //             println!("🔑  Export password: {password}");
    //         }

    //         Ok(())
    //     }
    //     (None, Some(errors)) => {
    //         let messages = errors
    //             .into_iter()
    //             .map(|e| e.message)
    //             .collect::<Vec<_>>()
    //             .join(", ");
    //         Err(messages.into())
    //     }
    //     _ => Err("Unknown error: empty data and no GraphQL errors".into()),
    // }
} 