// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Thin wrapper around POSTing a `graphql_client` query to Hasura and
//! parsing the response. Each call site still owns interpreting
//! `Response::data`/`Response::errors` — `insert_cast_vote`'s callers need
//! the raw `errors` to classify outcomes by `extensions.code`; everything
//! else just wants data-or-bail, which `data_or_bail` gives them.

use anyhow::{Context, Result};
use graphql_client::{GraphQLQuery, Response};

#[derive(Clone)]
pub struct HasuraClient {
    http: reqwest::Client,
    endpoint_url: String,
    bearer_token: String,
}

impl HasuraClient {
    pub fn new(
        http: reqwest::Client,
        endpoint_url: impl Into<String>,
        bearer_token: impl Into<String>,
    ) -> Self {
        Self {
            http,
            endpoint_url: endpoint_url.into(),
            bearer_token: bearer_token.into(),
        }
    }

    /// Sends the query and returns the raw `Response`, un-interpreted.
    pub async fn send<Q: GraphQLQuery>(
        &self,
        variables: Q::Variables,
    ) -> Result<Response<Q::ResponseData>> {
        let body = Q::build_query(variables);
        let response = self
            .http
            .post(&self.endpoint_url)
            .bearer_auth(&self.bearer_token)
            .json(&body)
            .send()
            .await
            .context("failed to send GraphQL request")?;

        let status = response.status();
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            anyhow::bail!("HTTP {status} from {}: {text}", self.endpoint_url);
        }

        response
            .json()
            .await
            .with_context(|| format!("failed to parse GraphQL response (HTTP {status})"))
    }

    /// Sends the query and returns `data`, or an error built from
    /// `errors` (or a generic message if the response had neither).
    pub async fn data_or_bail<Q: GraphQLQuery>(
        &self,
        variables: Q::Variables,
    ) -> Result<Q::ResponseData> {
        let response = self.send::<Q>(variables).await?;
        data_or_bail(response)
    }
}

pub fn data_or_bail<T>(response: Response<T>) -> Result<T> {
    if let Some(data) = response.data {
        Ok(data)
    } else if let Some(errors) = response.errors {
        anyhow::bail!("{}", format_errors(&errors))
    } else {
        anyhow::bail!("GraphQL response had neither data nor errors")
    }
}

pub fn format_errors(errors: &[graphql_client::Error]) -> String {
    errors
        .iter()
        .map(|error| error.message.clone())
        .collect::<Vec<_>>()
        .join(", ")
}

/// The `extensions.code` of the first GraphQL error, if any — this is how
/// Hasura-wrapped Harvest errors are classified, since Hasura always
/// responds HTTP 200 regardless of the underlying action's status code.
pub fn first_error_code(errors: &[graphql_client::Error]) -> Option<&str> {
    errors.first()?.extensions.as_ref()?.get("code")?.as_str()
}
