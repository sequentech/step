// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::utils::read_config::read_config;
use clap::Args;
use colored::Colorize;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

const GET_ELECTION_EVENT_PRESENTATION_QUERY: &str = r#"
query GetElectionEventPresentation($election_event_id: uuid!) {
  sequent_backend_election_event(
    where: { id: { _eq: $election_event_id } }
    limit: 1
  ) {
    id
    presentation
  }
}
"#;

const CONFIGURE_RESULTS_WEBSITE_MUTATION: &str = r#"
mutation ConfigureResultsWebsite($election_event_id: uuid!, $presentation: jsonb!) {
  update_sequent_backend_election_event(
    where: { id: { _eq: $election_event_id } }
    _set: { presentation: $presentation }
  ) {
    affected_rows
    returning {
      id
      presentation
    }
  }
}
"#;

const PUBLISH_RESULTS_WEBSITE_MUTATION: &str = r#"
mutation PublishResultsWebsite(
  $election_event_id: String!
  $tally_session_id: String!
  $tally_session_execution_id: String!
  $results_event_id: String!
  $route_scope: String!
  $route_election_id: String
  $election_ids: [String!]!
  $contest_ids: [String!]!
  $access: String!
  $visibility_scope: String!
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

const REFRESH_RESULTS_PUBLICATION_INDEX_MUTATION: &str = r#"
mutation RefreshResultsPublicationIndex($election_event_id: String!) {
  refreshResultsPublicationIndex(election_event_id: $election_event_id) {
    election_event_id
    results_enabled
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
    route_scope: String,

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
    access: String,

    /// Visibility scope: full_event or area_based
    #[arg(long, default_value = "full_event")]
    visibility_scope: String,
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
    status: String,

    /// Access mode: public or authenticated
    #[arg(long, default_value = "public")]
    access: String,

    /// Visibility scope: full_event or area_based
    #[arg(long, default_value = "full_event")]
    visibility_scope: String,
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
    publication_status: String,
    error_msg: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RevokeResultsPublicationPayload {
    publication_id: String,
    publication_status: String,
}

#[derive(Debug, Deserialize)]
pub struct ConfigureResultsWebsitePayload {
    election_event_id: String,
    status: String,
    access: String,
    visibility_scope: String,
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
struct ElectionEventPresentationVariables {
    election_event_id: String,
}

#[derive(Debug, Deserialize)]
struct ElectionEventPresentationRow {
    id: String,
    presentation: Option<Value>,
}

#[derive(Deserialize)]
struct GetElectionEventPresentationData {
    sequent_backend_election_event: Vec<ElectionEventPresentationRow>,
}

#[derive(Serialize)]
struct ConfigureResultsWebsiteVariables {
    election_event_id: String,
    presentation: Value,
}

#[derive(Deserialize)]
struct UpdateElectionEventResult {
    affected_rows: i64,
    returning: Vec<ElectionEventPresentationRow>,
}

#[derive(Deserialize)]
struct ConfigureResultsWebsiteData {
    update_sequent_backend_election_event: Option<UpdateElectionEventResult>,
}

#[derive(Serialize)]
struct PublishResultsVariables {
    election_event_id: String,
    tally_session_id: String,
    tally_session_execution_id: String,
    results_event_id: String,
    route_scope: String,
    route_election_id: Option<String>,
    election_ids: Vec<String>,
    contest_ids: Vec<String>,
    access: String,
    visibility_scope: String,
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

#[derive(Serialize)]
struct RefreshResultsPublicationIndexVariables {
    election_event_id: String,
}

#[derive(Deserialize)]
struct RefreshResultsPublicationIndexData {
    #[serde(rename = "refreshResultsPublicationIndex")]
    refresh_results_publication_index: Option<Value>,
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
                    result.publication_status.cyan()
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
                println!("{} {}", "Status:".green(), result.status.cyan());
                println!("{} {}", "Access:".green(), result.access.cyan());
                println!(
                    "{} {}",
                    "Visibility scope:".green(),
                    result.visibility_scope.cyan()
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
                    result.publication_status.cyan()
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
    validate_results_website_policy(&command.status, &command.access, &command.visibility_scope)?;

    let existing_presentation = get_election_event_presentation(&command.election_event_id)?;
    let mut presentation = match existing_presentation {
        Some(Value::Null) | None => json!({}),
        Some(value) => value,
    };
    let presentation_object =
        match presentation.as_object_mut() {
            Some(object) => object,
            None => return Err(
                "election event presentation must be a JSON object to configure results website"
                    .into(),
            ),
        };
    presentation_object.insert(
        "results_website".to_string(),
        json!({
            "status": command.status,
            "access": command.access,
            "visibility_scope": command.visibility_scope,
        }),
    );

    let variables = ConfigureResultsWebsiteVariables {
        election_event_id: command.election_event_id.clone(),
        presentation,
    };
    let request_body = GraphqlRequest {
        query: CONFIGURE_RESULTS_WEBSITE_MUTATION,
        variables,
    };
    let response: GraphqlResponse<ConfigureResultsWebsiteData> =
        post_graphql_with_role(request_body, "election-event-write")?;

    if let Some(data) = response.data {
        if let Some(update_result) = data.update_sequent_backend_election_event {
            if update_result.affected_rows == 0 {
                return Err("election event not found or not writable".into());
            }
            let updated_row = update_result
                .returning
                .first()
                .ok_or("results website policy update returned no row")?;
            let policy = updated_row
                .presentation
                .as_ref()
                .and_then(|presentation| presentation.get("results_website"))
                .and_then(Value::as_object)
                .ok_or("results website policy was not stored")?;
            refresh_results_publication_index(&command.election_event_id)?;

            return Ok(ConfigureResultsWebsitePayload {
                election_event_id: updated_row.id.clone(),
                status: policy_string(policy, "status")?,
                access: policy_string(policy, "access")?,
                visibility_scope: policy_string(policy, "visibility_scope")?,
            });
        }
    }

    Err(graphql_error_message(response.errors, "failed to configure results website").into())
}

pub fn publish_results(
    command: &PublishResults,
) -> Result<PublishResultsWebsitePayload, Box<dyn std::error::Error>> {
    validate_publish_config(command)?;

    let route_election_id = if command.route_scope == "election" {
        command.route_election_id.clone()
    } else {
        None
    };
    let variables = PublishResultsVariables {
        election_event_id: command.election_event_id.clone(),
        tally_session_id: command.tally_session_id.clone(),
        tally_session_execution_id: command.tally_session_execution_id.clone(),
        results_event_id: command.results_event_id.clone(),
        route_scope: command.route_scope.clone(),
        route_election_id,
        election_ids: command.election_ids.clone(),
        contest_ids: command.contest_ids.clone(),
        access: command.access.clone(),
        visibility_scope: command.visibility_scope.clone(),
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

fn refresh_results_publication_index(
    election_event_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let variables = RefreshResultsPublicationIndexVariables {
        election_event_id: election_event_id.to_string(),
    };
    let request_body = GraphqlRequest {
        query: REFRESH_RESULTS_PUBLICATION_INDEX_MUTATION,
        variables,
    };
    let response: GraphqlResponse<RefreshResultsPublicationIndexData> = post_graphql(request_body)?;

    if let Some(data) = response.data {
        if data.refresh_results_publication_index.is_some() {
            return Ok(());
        }
    }

    Err(graphql_error_message(
        response.errors,
        "failed to refresh results publication index",
    )
    .into())
}

fn post_graphql<V: Serialize, T: for<'de> Deserialize<'de>>(
    request_body: GraphqlRequest<V>,
) -> Result<GraphqlResponse<T>, Box<dyn std::error::Error>> {
    post_graphql_with_role(request_body, "publish-results-write")
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

fn get_election_event_presentation(
    election_event_id: &str,
) -> Result<Option<Value>, Box<dyn std::error::Error>> {
    let variables = ElectionEventPresentationVariables {
        election_event_id: election_event_id.to_string(),
    };
    let request_body = GraphqlRequest {
        query: GET_ELECTION_EVENT_PRESENTATION_QUERY,
        variables,
    };
    let response: GraphqlResponse<GetElectionEventPresentationData> =
        post_graphql_with_role(request_body, "election-event-write")?;

    if let Some(data) = response.data {
        return data
            .sequent_backend_election_event
            .into_iter()
            .next()
            .map(|row| row.presentation)
            .ok_or_else(|| "election event not found or not readable".into());
    }

    Err(graphql_error_message(response.errors, "failed to read election event").into())
}

fn validate_publish_config(command: &PublishResults) -> Result<(), Box<dyn std::error::Error>> {
    validate_value("route-scope", &command.route_scope, &["event", "election"])?;
    validate_results_website_policy("enabled", &command.access, &command.visibility_scope)?;

    if command.route_scope == "election" && command.route_election_id.is_none() {
        return Err("route-election-id is required when route-scope is election".into());
    }

    Ok(())
}

fn validate_results_website_policy(
    status: &str,
    access: &str,
    visibility_scope: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_value("status", status, &["enabled", "disabled"])?;
    validate_value("access", access, &["public", "authenticated"])?;
    validate_value(
        "visibility-scope",
        visibility_scope,
        &["full_event", "area_based"],
    )?;

    if access == "public" && visibility_scope != "full_event" {
        return Err("public results must use full_event visibility".into());
    }

    Ok(())
}

fn validate_value(
    name: &str,
    value: &str,
    accepted_values: &[&str],
) -> Result<(), Box<dyn std::error::Error>> {
    if accepted_values.contains(&value) {
        Ok(())
    } else {
        Err(format!(
            "invalid {name}: {value}. Expected one of: {}",
            accepted_values.join(", ")
        )
        .into())
    }
}

fn policy_string(
    policy: &Map<String, Value>,
    key: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    policy
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| format!("results website policy is missing {key}").into())
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
