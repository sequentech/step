// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The trustee's calls to the platform: which boards are its own, and the
//! report of a halt on one of them.

use std::time::Duration;

use anyhow::{bail, Context as _, Result};
use protocol_board::{TrusteeBoard, TrusteeBoardsResponse, TrusteeReport};
use serde::de::DeserializeOwned;

use crate::login::{Credentials, TokenSource};

/// How long a request to the platform may take before it counts as failed.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// harvest, called as the trustee's user.
pub(crate) struct PlatformClient {
    http: reqwest::Client,
    boards_url: String,
    report_url: String,
    tokens: TokenSource,
}

impl PlatformClient {
    pub(crate) fn new(
        harvest_url: &str,
        credentials: Credentials,
    ) -> Result<PlatformClient> {
        let http = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .context("building the platform's HTTP client")?;
        let harvest_url = harvest_url.trim_end_matches('/');
        Ok(PlatformClient {
            tokens: TokenSource::new(http.clone(), credentials),
            http,
            boards_url: format!("{harvest_url}/trustee/boards"),
            report_url: format!("{harvest_url}/trustee/boards/report"),
        })
    }

    /// The boards that need this trustee's work, oldest first.
    pub(crate) async fn list(&mut self) -> Result<Vec<TrusteeBoard>> {
        let token = self.tokens.access_token().await?;
        let answer: TrusteeBoardsResponse =
            call(self.http.get(&self.boards_url).bearer_auth(token))
                .await
                .with_context(|| format!("GET {}", self.boards_url))?;
        Ok(answer.boards)
    }

    /// Report a halt.
    pub(crate) async fn report(
        &mut self,
        report: &TrusteeReport,
    ) -> Result<()> {
        let token = self.tokens.access_token().await?;
        send(
            self.http
                .post(&self.report_url)
                .bearer_auth(token)
                .json(report),
        )
        .await
        .with_context(|| {
            format!("POST {} for board {}", self.report_url, report.board)
        })?;
        Ok(())
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
