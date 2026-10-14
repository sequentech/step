// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The trustee's calls to the platform: which boards are its own, and the
//! report of a halt on one of them.

use std::time::Duration;

use anyhow::{bail, Context as _, Result};
use protocol_board::{
    TrusteeActionResult, TrusteeBoard, TrusteeBoardsResponse,
    TrusteeEventUpdates, TrusteeReport,
};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::login::{Credentials, TokenSource};

/// How long a request to the platform may take before it counts as failed.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

const TRUSTEE_ROLE: &str = "trustee-ceremony";
const EVENT_UPDATES: &str =
    "query TrusteeEventUpdates { trustee_event_updates }";
const ACTION_RESULT: &str = "mutation SubmitTrusteeActionResult($result: jsonb!) { submit_trustee_action_result(result: $result) { accepted } }";

const LIST_BOARDS: &str =
    "query ListTrusteeBoards { list_trustee_boards { boards { name kind parent } } }";
const REPORT_BOARD: &str =
    "mutation ReportTrusteeBoard($report: jsonb!) { report_trustee_board(report: $report) { accepted } }";

#[derive(Deserialize)]
struct GraphqlResponse<T> {
    data: Option<T>,
    #[serde(default)]
    errors: Vec<GraphqlError>,
}

#[derive(Deserialize)]
struct GraphqlError {
    message: String,
}

#[derive(Deserialize)]
struct BoardListing {
    list_trustee_boards: TrusteeBoardsResponse,
}

#[derive(Deserialize)]
struct ReportResult {
    report_trustee_board: Option<ReportAcknowledgement>,
}

#[derive(Deserialize)]
struct ReportAcknowledgement {
    accepted: bool,
}

#[derive(Deserialize)]
struct EventUpdatesResult {
    trustee_event_updates: TrusteeEventUpdates,
}

#[derive(Deserialize)]
struct ActionResultAcknowledgement {
    submit_trustee_action_result: Option<ReportAcknowledgement>,
}

/// The same public Hasura actions browser trustees call, using HTTP and JSON.
pub(crate) struct PlatformClient {
    http: reqwest::Client,
    graphql_url: String,
    tokens: TokenSource,
}

impl PlatformClient {
    pub(crate) fn new(
        graphql_url: &str,
        credentials: Credentials,
    ) -> Result<PlatformClient> {
        let http = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .context("building the platform's HTTP client")?;
        Ok(PlatformClient {
            tokens: TokenSource::new(http.clone(), credentials),
            http,
            graphql_url: graphql_url.to_owned(),
        })
    }

    /// The boards that need this trustee's work, oldest first.
    pub(crate) async fn list(&mut self) -> Result<Vec<TrusteeBoard>> {
        let answer: BoardListing = self.query(LIST_BOARDS, json!({})).await?;
        Ok(answer.list_trustee_boards.boards)
    }

    /// One discovery request carries board assignments and pending actions.
    /// During a rolling upgrade only an absent new field falls back to the
    /// previous discovery endpoint; authentication and data errors still fail.
    pub(crate) async fn updates(&mut self) -> Result<TrusteeEventUpdates> {
        match self
            .query::<EventUpdatesResult>(EVENT_UPDATES, json!({}))
            .await
        {
            Ok(answer) => Ok(answer.trustee_event_updates),
            Err(error) if missing_event_updates_field(&error) => {
                Ok(TrusteeEventUpdates {
                    boards: self.list().await?,
                    actions: Vec::new(),
                })
            }
            Err(error) => Err(error),
        }
    }

    pub(crate) async fn action_result(
        &mut self,
        result: &TrusteeActionResult,
    ) -> Result<()> {
        let answer: ActionResultAcknowledgement =
            self.query(ACTION_RESULT, json!({"result": result})).await?;
        if !answer
            .submit_trustee_action_result
            .is_some_and(|ack| ack.accepted)
        {
            bail!("The platform did not acknowledge the trustee action result");
        }
        Ok(())
    }

    /// Report a halt; only an explicit acknowledgement settles it.
    pub(crate) async fn report(
        &mut self,
        report: &TrusteeReport,
    ) -> Result<()> {
        let answer: ReportResult = self
            .query(REPORT_BOARD, json!({"report": report}))
            .await
            .with_context(|| format!("reporting board {}", report.board))?;
        if !answer
            .report_trustee_board
            .is_some_and(|acknowledgement| acknowledgement.accepted)
        {
            bail!("the platform did not acknowledge the halted board");
        }
        Ok(())
    }

    /// Fixed operations sent through the existing HTTP client. GraphQL errors
    /// may arrive with HTTP 200 and partial data; neither counts as success.
    async fn query<T: DeserializeOwned>(
        &mut self,
        query: &str,
        variables: Value,
    ) -> Result<T> {
        let token = self.tokens.access_token().await?;
        let answer: GraphqlResponse<T> = call(
            self.http
                .post(&self.graphql_url)
                .bearer_auth(token)
                .header("x-hasura-role", TRUSTEE_ROLE)
                .json(&json!({"query": query, "variables": variables})),
        )
        .await
        .with_context(|| format!("POST {}", self.graphql_url))?;
        if !answer.errors.is_empty() {
            bail!(
                "trustee API: {}",
                answer
                    .errors
                    .iter()
                    .map(|error| error.message.as_str())
                    .collect::<Vec<_>>()
                    .join("; ")
            );
        }
        answer.data.context("the trustee API returned no data")
    }
}

/// Send a request and read its JSON answer.
pub(crate) async fn call<T: DeserializeOwned>(
    request: reqwest::RequestBuilder,
) -> Result<T> {
    send(request)
        .await?
        .json()
        .await
        .context("reading the answer")
}

/// Send a request. An answer that is not a success is an error with its
/// status and its body.
async fn send(request: reqwest::RequestBuilder) -> Result<reqwest::Response> {
    let response = request.send().await?;
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }
    match response.text().await {
        Ok(body) => bail!("HTTP {status}: {body}"),
        Err(err) => {
            bail!("HTTP {status}, with a body that cannot be read: {err}")
        }
    }
}

#[cfg(test)]
#[path = "platform_tests.rs"]
pub(crate) mod tests;

fn missing_event_updates_field(error: &anyhow::Error) -> bool {
    let message = error.to_string();
    message.contains("Cannot query field \"trustee_event_updates\"")
        || message.contains(
            "field 'trustee_event_updates' not found in type: 'query_root'",
        )
}
