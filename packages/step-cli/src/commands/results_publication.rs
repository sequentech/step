// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::utils::read_config::read_config;
use clap::Args;
use colored::Colorize;
use sequent_core::ballot::{
    ResultsWebsiteAccess, ResultsWebsiteStatus, ResultsWebsiteVisibilityScope,
};
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use windmill::types::results_publication::{
    validate_access_visibility, ResultsPublicationStatus, ResultsRouteScope,
};

const CONFIGURE_RESULTS_WEBSITE_MUTATION: &str = r#"
mutation ConfigureResultsWebsite(
  $election_event_id: String!
  $status: ResultsWebsiteStatus!
  $access: ResultsWebsiteAccess!
  $visibility_scope: ResultsWebsiteVisibilityScope!
) {
  configureResultsWebsitePolicy(
    election_event_id: $election_event_id
    status: $status
    access: $access
    visibility_scope: $visibility_scope
  ) {
    election_event_id
    status
    access
    visibility_scope
  }
}
"#;

const PUBLISH_RESULTS_WEBSITE_MUTATION: &str = r#"
mutation PublishResultsWebsite(
  $election_event_id: String!
  $tally_session_id: String!
  $tally_session_execution_id: String!
  $results_event_id: String!
  $route_scope: ResultsRouteScope!
  $route_election_id: String
  $election_ids: [String!]!
  $contest_ids: [String!]!
  $access: ResultsWebsiteAccess!
  $visibility_scope: ResultsWebsiteVisibilityScope!
) {
  publishResultsWebsite(
    election_event_id: $election_event_id
    tally_session_id: $tally_session_id
    tally_session_execution_id: $tally_session_execution_id
    results_event_id: $results_event_id
    route_scope: $route_scope
    route_election_id: $route_election_id
    election_ids: $election_ids
    contest_ids: $contest_ids
    access: $access
    visibility_scope: $visibility_scope
  ) {
    publication_id
    task_execution_id
    publication_status
    error_msg
  }
}
"#;

const REVOKE_RESULTS_PUBLICATION_MUTATION: &str = r#"
mutation RevokeResultsPublication($election_event_id: String!, $publication_id: String!) {
  revokeResultsPublication(
    election_event_id: $election_event_id
    publication_id: $publication_id
  ) {
    publication_id
    publication_status
  }
}
"#;

#[derive(Args)]
#[command(
    about = "Publish configured tally results to the results website",
    long_about = None
)]
pub struct PublishResults {
    #[arg(long)]
    election_event_id: String,

    #[arg(long)]
    tally_session_id: String,

    #[arg(long)]
    tally_session_execution_id: String,

    #[arg(long)]
    results_event_id: String,

    /// Results route scope: event or election
    #[arg(long, default_value = "event")]
    route_scope: ResultsRouteScope,

    /// Required when route-scope is election
    #[arg(long)]
    route_election_id: Option<String>,

    /// Election id included in this publication. Repeat for multiple elections.
    #[arg(long = "election-id", required = true)]
    election_ids: Vec<String>,

    /// Contest id included in this publication. Repeat for multiple contests.
    #[arg(long = "contest-id", required = true)]
    contest_ids: Vec<String>,

    /// Access mode: public or authenticated
    #[arg(long, default_value = "public")]
    access: ResultsWebsiteAccess,

    /// Visibility scope: full_event or area_based
    #[arg(long, default_value = "full_event")]
    visibility_scope: ResultsWebsiteVisibilityScope,
}

#[derive(Args)]
#[command(
    about = "Configure results website policy for an election event",
    long_about = None
)]
pub struct ConfigureResultsWebsite {
    #[arg(long)]
    election_event_id: String,

    /// Results website status: enabled or disabled
    #[arg(long, default_value = "enabled")]
    status: ResultsWebsiteStatus,

    /// Access mode: public or authenticated
    #[arg(long, default_value = "public")]
    access: ResultsWebsiteAccess,

    /// Visibility scope: full_event or area_based
    #[arg(long, default_value = "full_event")]
    visibility_scope: ResultsWebsiteVisibilityScope,
}

#[derive(Args)]
#[command(about = "Revoke a results website publication", long_about = None)]
pub struct RevokeResultsPublication {
    #[arg(long)]
    election_event_id: String,

    #[arg(long)]
    publication_id: String,
}

#[derive(Debug, Deserialize)]
pub struct PublishResultsWebsitePayload {
    publication_id: String,
    task_execution_id: String,
    publication_status: ResultsPublicationStatus,
    error_msg: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RevokeResultsPublicationPayload {
    publication_id: String,
    publication_status: ResultsPublicationStatus,
}

#[derive(Debug, Deserialize)]
pub struct ConfigureResultsWebsitePayload {
    election_event_id: String,
    status: ResultsWebsiteStatus,
    access: ResultsWebsiteAccess,
    visibility_scope: ResultsWebsiteVisibilityScope,
}

#[derive(Serialize)]
struct GraphqlRequest<V> {
    query: &'static str,
    variables: V,
}

#[derive(Debug, Deserialize)]
struct GraphqlResponse<T> {
    data: Option<T>,
    errors: Option<Vec<GraphqlError>>,
}

#[derive(Debug, Deserialize)]
struct GraphqlError {
    message: String,
}

#[derive(Serialize)]
struct ConfigureResultsWebsiteVariables {
    election_event_id: String,
    status: ResultsWebsiteStatus,
    access: ResultsWebsiteAccess,
    visibility_scope: ResultsWebsiteVisibilityScope,
}

#[derive(Deserialize)]
struct ConfigureResultsWebsiteData {
    #[serde(rename = "configureResultsWebsitePolicy")]
    configure_results_website_policy: Option<ConfigureResultsWebsitePayload>,
}

#[derive(Serialize)]
struct PublishResultsVariables {
    election_event_id: String,
    tally_session_id: String,
    tally_session_execution_id: String,
    results_event_id: String,
    route_scope: ResultsRouteScope,
    route_election_id: Option<String>,
    election_ids: Vec<String>,
    contest_ids: Vec<String>,
    access: ResultsWebsiteAccess,
    visibility_scope: ResultsWebsiteVisibilityScope,
}

#[derive(Deserialize)]
struct PublishResultsData {
    #[serde(rename = "publishResultsWebsite")]
    publish_results_website: Option<PublishResultsWebsitePayload>,
}

#[derive(Serialize)]
struct RevokeResultsVariables {
    election_event_id: String,
    publication_id: String,
}

#[derive(Deserialize)]
struct RevokeResultsData {
    #[serde(rename = "revokeResultsPublication")]
    revoke_results_publication: Option<RevokeResultsPublicationPayload>,
}

impl PublishResults {
    pub fn run(&self) {
        match publish_results(self) {
            Ok(result) => {
                println!(
                    "{} {}",
                    "Success! Results publication started. Publication ID:".green(),
                    result.publication_id.cyan()
                );
                println!(
                    "{} {}",
                    "Task execution ID:".green(),
                    result.task_execution_id.cyan()
                );
                println!(
                    "{} {}",
                    "Publication status:".green(),
                    result.publication_status.to_string().cyan()
                );
                if let Some(error_msg) = result.error_msg {
                    eprintln!("{} {}", "Warning:".yellow(), error_msg);
                }
            }
            Err(err) => {
                eprintln!("Error! Failed to publish results: {}", err)
            }
        }
    }
}

impl ConfigureResultsWebsite {
    pub fn run(&self) {
        match configure_results_website(self) {
            Ok(result) => {
                println!(
                    "{} {}",
                    "Success! Configured results website for election event:".green(),
                    result.election_event_id.cyan()
                );
                println!("{} {}", "Status:".green(), result.status.to_string().cyan());
                println!("{} {}", "Access:".green(), result.access.to_string().cyan());
                println!(
                    "{} {}",
                    "Visibility scope:".green(),
                    result.visibility_scope.to_string().cyan()
                );
            }
            Err(err) => {
                eprintln!("Error! Failed to configure results website: {}", err)
            }
        }
    }
}

impl RevokeResultsPublication {
    pub fn run(&self) {
        match revoke_results_publication(&self.election_event_id, &self.publication_id) {
            Ok(result) => {
                println!(
                    "{} {}",
                    "Success! Results publication revoked. Publication ID:".green(),
                    result.publication_id.cyan()
                );
                println!(
                    "{} {}",
                    "Publication status:".green(),
                    result.publication_status.to_string().cyan()
                );
            }
            Err(err) => {
                eprintln!("Error! Failed to revoke results publication: {}", err)
            }
        }
    }
}

pub fn configure_results_website(
    command: &ConfigureResultsWebsite,
) -> Result<ConfigureResultsWebsitePayload, Box<dyn std::error::Error>> {
    validate_access_visibility(command.access, command.visibility_scope)?;

    let variables = ConfigureResultsWebsiteVariables {
        election_event_id: command.election_event_id.clone(),
        status: command.status,
        access: command.access,
        visibility_scope: command.visibility_scope,
    };
    let request_body = GraphqlRequest {
        query: CONFIGURE_RESULTS_WEBSITE_MUTATION,
        variables,
    };
    let response: GraphqlResponse<ConfigureResultsWebsiteData> = post_graphql(request_body)?;

    if let Some(data) = response.data {
        if let Some(payload) = data.configure_results_website_policy {
            return Ok(payload);
        }
    }

    Err(graphql_error_message(response.errors, "failed to configure results website").into())
}

pub fn publish_results(
    command: &PublishResults,
) -> Result<PublishResultsWebsitePayload, Box<dyn std::error::Error>> {
    validate_publish_config(command)?;

    let route_election_id = if command.route_scope == ResultsRouteScope::Election {
        command.route_election_id.clone()
    } else {
        None
    };
    let variables = PublishResultsVariables {
        election_event_id: command.election_event_id.clone(),
        tally_session_id: command.tally_session_id.clone(),
        tally_session_execution_id: command.tally_session_execution_id.clone(),
        results_event_id: command.results_event_id.clone(),
        route_scope: command.route_scope,
        route_election_id,
        election_ids: command.election_ids.clone(),
        contest_ids: command.contest_ids.clone(),
        access: command.access,
        visibility_scope: command.visibility_scope,
    };
    let request_body = GraphqlRequest {
        query: PUBLISH_RESULTS_WEBSITE_MUTATION,
        variables,
    };
    let response: GraphqlResponse<PublishResultsData> = post_graphql(request_body)?;

    if let Some(data) = response.data {
        if let Some(payload) = data.publish_results_website {
            return Ok(payload);
        }
    }

    Err(graphql_error_message(response.errors, "failed to publish results").into())
}

pub fn revoke_results_publication(
    election_event_id: &str,
    publication_id: &str,
) -> Result<RevokeResultsPublicationPayload, Box<dyn std::error::Error>> {
    let variables = RevokeResultsVariables {
        election_event_id: election_event_id.to_string(),
        publication_id: publication_id.to_string(),
    };
    let request_body = GraphqlRequest {
        query: REVOKE_RESULTS_PUBLICATION_MUTATION,
        variables,
    };
    let response: GraphqlResponse<RevokeResultsData> = post_graphql(request_body)?;

    if let Some(data) = response.data {
        if let Some(payload) = data.revoke_results_publication {
            return Ok(payload);
        }
    }

    Err(graphql_error_message(response.errors, "failed to revoke results publication").into())
}

fn post_graphql<V: Serialize, T: for<'de> Deserialize<'de>>(
    request_body: GraphqlRequest<V>,
) -> Result<GraphqlResponse<T>, Box<dyn std::error::Error>> {
    let hasura_role = Permissions::PUBLISH_RESULTS_WRITE.to_string();
    post_graphql_with_role(request_body, &hasura_role)
}

fn post_graphql_with_role<V: Serialize, T: for<'de> Deserialize<'de>>(
    request_body: GraphqlRequest<V>,
    hasura_role: &str,
) -> Result<GraphqlResponse<T>, Box<dyn std::error::Error>> {
    let config = read_config()?;
    let client = reqwest::blocking::Client::new();
    let response = client
        .post(&config.endpoint_url)
        .bearer_auth(config.auth_token)
        .header("x-hasura-role", hasura_role)
        .json(&request_body)
        .send()?;

    if response.status().is_success() {
        Ok(response.json()?)
    } else {
        let status = response.status();
        let error_message = response.text()?;
        Err(format!("HTTP Status: {}\nError Message: {}", status, error_message).into())
    }
}

fn validate_publish_config(command: &PublishResults) -> Result<(), Box<dyn std::error::Error>> {
    validate_access_visibility(command.access, command.visibility_scope)?;

    match (command.route_scope, command.route_election_id.as_deref()) {
        (ResultsRouteScope::Event, Some(_)) => {
            return Err("route-election-id cannot be used when route-scope is event".into());
        }
        (ResultsRouteScope::Election, None) => {
            return Err("route-election-id is required when route-scope is election".into());
        }
        (ResultsRouteScope::Election, Some(route_election_id))
            if command.election_ids.len() != 1
                || command.election_ids.first().map(String::as_str) != Some(route_election_id) =>
        {
            return Err("an election route must publish exactly its route election".into());
        }
        _ => {}
    }

    Ok(())
}

fn graphql_error_message(errors: Option<Vec<GraphqlError>>, fallback: &str) -> String {
    errors
        .map(|items| {
            items
                .into_iter()
                .map(|error| error.message)
                .collect::<Vec<String>>()
                .join(", ")
        })
        .filter(|message| !message.is_empty())
        .unwrap_or_else(|| fallback.to_string())
}
