// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The Vitrine sidecar's internal listener (Vitrine's `docs/api.md`, sections
//! 2.2, 2.3 and 2.5): the admin routes, the `events` log's submissions and
//! receipts, and the voter routes of private events.

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;
use strum_macros::{Display, EnumString};

/// The media type of `POST add` bodies.
pub const SUBMISSION_TYPE: &str = "application/vnd.vitrine.submission";
/// The longest wait `POST add` and `GET wait` take, in milliseconds.
pub const MAX_WAIT_MS: u64 = 5000;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_ERROR_BODY: usize = 512;

/// The service whose environment configures the client.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display)]
pub enum Service {
    #[strum(serialize = "HARVEST")]
    Harvest,
    #[strum(serialize = "WINDMILL")]
    Windmill,
}

impl Service {
    pub fn internal_url_env(&self) -> String {
        format!("{self}_VITRINE_INTERNAL_URL")
    }

    pub fn admin_token_file_env(&self) -> String {
        format!("{self}_VITRINE_ADMIN_TOKEN_FILE")
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SidecarError {
    #[error("the Vitrine sidecar answered {status}: {body}")]
    Status { status: u16, body: String },
    #[error("the Vitrine sidecar can't be reached: {0}")]
    Transport(String),
    #[error("the Vitrine sidecar's answer isn't valid: {0}")]
    BadResponse(String),
}

impl SidecarError {
    pub fn status(&self) -> Option<u16> {
        match self {
            SidecarError::Status { status, .. } => Some(*status),
            _ => None,
        }
    }
}

/// How a log takes its entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Display, EnumString)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum Submission {
    /// From its queue table, which the ballot box writes.
    Queue,
    /// Through `POST add`, with the log's submission token.
    Http,
}

/// `POST /admin/logs`.
#[derive(Debug, Clone, Serialize)]
pub struct CreateLog<'a> {
    pub name: &'a str,
    pub origin: &'a str,
    pub database: &'a str,
    pub profile: &'a str,
    pub visibility: &'a str,
    pub submission: Submission,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_role: Option<&'a str>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreatedLog {
    pub id: i64,
    pub instance_id: String,
    pub vkey: String,
    pub log_time_vkey: String,
    pub url: String,
    pub token: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct InstalledPolicy {
    pub policy_id: String,
    pub ack: String,
}

/// What became of an entry submitted or waited for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Answer {
    /// The log sequenced it: its receipt, against its batch's checkpoint.
    Receipt(String),
    /// Still queued when the wait ended.
    Queued,
}

/// A voter route's answer, as the sidecar gave it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Forwarded {
    pub status: u16,
    pub body: String,
}

#[derive(Debug, Clone)]
pub struct SidecarClient {
    http: reqwest::Client,
    base: String,
    admin_token: Option<String>,
}

fn transport(e: reqwest::Error) -> SidecarError {
    SidecarError::Transport(e.without_url().to_string())
}

fn one_line(body: &str) -> String {
    let line = body.lines().next().unwrap_or_default();
    line.chars().take(MAX_ERROR_BODY).collect()
}

/// The token in a token file: its first line.
pub fn read_token_file(path: &str) -> Result<String> {
    let text = std::fs::read_to_string(path).with_context(|| format!("Error reading {path}"))?;
    let token = text.lines().next().unwrap_or_default().trim().to_string();
    if token.is_empty() {
        return Err(anyhow!("{path} holds no token"));
    }
    Ok(token)
}

impl SidecarClient {
    pub fn new(base: &str, admin_token: Option<String>) -> Result<Self> {
        let http = reqwest::Client::builder()
            .http2_prior_knowledge()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .context("Error building the Vitrine sidecar's HTTP client")?;
        Ok(SidecarClient {
            http,
            base: base.trim_end_matches('/').to_string(),
            admin_token,
        })
    }

    /// The client for a service: `<SERVICE>_VITRINE_INTERNAL_URL` and
    /// `<SERVICE>_VITRINE_ADMIN_TOKEN_FILE`.
    pub fn from_env(service: Service) -> Result<Self> {
        let url_env = service.internal_url_env();
        let base = std::env::var(&url_env)
            .ok()
            .filter(|url| !url.is_empty())
            .ok_or_else(|| anyhow!("{url_env} isn't set"))?;
        let token = std::env::var(service.admin_token_file_env())
            .ok()
            .filter(|path| !path.is_empty())
            .map(|path| read_token_file(&path))
            .transpose()?;
        Self::new(&base, token)
    }

    fn url(&self, path: &str) -> String {
        format!("{}/{}", self.base, path.trim_start_matches('/'))
    }

    fn admin(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match &self.admin_token {
            Some(token) => request.bearer_auth(token),
            None => request,
        }
    }

    async fn send(&self, request: reqwest::RequestBuilder) -> Result<(u16, String), SidecarError> {
        let response = request.send().await.map_err(transport)?;
        let status = response.status().as_u16();
        let body = response.text().await.map_err(transport)?;
        Ok((status, body))
    }

    async fn expect(
        &self,
        request: reqwest::RequestBuilder,
        ok: &[u16],
    ) -> Result<(u16, String), SidecarError> {
        let (status, body) = self.send(request).await?;
        if ok.contains(&status) {
            Ok((status, body))
        } else {
            Err(SidecarError::Status {
                status,
                body: one_line(&body),
            })
        }
    }

    pub async fn create_log(&self, log: &CreateLog<'_>) -> Result<CreatedLog, SidecarError> {
        let request = self.admin(self.http.post(self.url("admin/logs")).json(log));
        let (_, body) = self.expect(request, &[201]).await?;
        serde_json::from_str(&body).map_err(|e| SidecarError::BadResponse(e.to_string()))
    }

    /// `GET /admin/logs`: every log of the sidecar.
    pub async fn logs(&self) -> Result<Vec<Value>, SidecarError> {
        let request = self.admin(self.http.get(self.url("admin/logs")));
        let (_, body) = self.expect(request, &[200]).await?;
        let listing: Value =
            serde_json::from_str(&body).map_err(|e| SidecarError::BadResponse(e.to_string()))?;
        match listing.get("logs") {
            Some(Value::Array(logs)) => Ok(logs.clone()),
            _ => Err(SidecarError::BadResponse("no logs list".into())),
        }
    }

    /// `GET <log>/status`.
    pub async fn status(&self, name: &str) -> Result<Value, SidecarError> {
        let (_, body) = self
            .expect(self.http.get(self.url(&format!("{name}/status"))), &[200])
            .await?;
        serde_json::from_str(&body).map_err(|e| SidecarError::BadResponse(e.to_string()))
    }

    pub async fn put_policy(
        &self,
        name: &str,
        policy: &str,
    ) -> Result<InstalledPolicy, SidecarError> {
        let request = self.admin(
            self.http
                .put(self.url(&format!("admin/logs/{name}/policy")))
                .header(reqwest::header::CONTENT_TYPE, "text/plain; charset=utf-8")
                .body(policy.to_string()),
        );
        let (_, body) = self.expect(request, &[200]).await?;
        serde_json::from_str(&body).map_err(|e| SidecarError::BadResponse(e.to_string()))
    }

    /// The log's witness enrolment note, or `None` while the sidecar has none
    /// to give (no enrolment key, or no policy yet).
    pub async fn enrolment(&self, name: &str) -> Result<Option<String>, SidecarError> {
        let request = self.admin(
            self.http
                .get(self.url(&format!("admin/logs/{name}/enrolment"))),
        );
        match self.expect(request, &[200, 404, 409]).await? {
            (200, note) => Ok(Some(note)),
            _ => Ok(None),
        }
    }

    pub async fn freeze(&self, name: &str) -> Result<(), SidecarError> {
        let request = self.admin(
            self.http
                .post(self.url(&format!("admin/logs/{name}/freeze"))),
        );
        self.expect(request, &[200]).await.map(|_| ())
    }

    fn answer(status: u16, body: String) -> Result<Answer, SidecarError> {
        match status {
            200 => Ok(Answer::Receipt(body)),
            202 => Ok(Answer::Queued),
            status => Err(SidecarError::Status {
                status,
                body: one_line(&body),
            }),
        }
    }

    /// `POST <log>/add?timeout=<ms>` with the log's submission token.
    pub async fn add(
        &self,
        name: &str,
        token: &str,
        entry: &[u8],
        timeout_ms: u64,
    ) -> Result<Answer, SidecarError> {
        let mut body = format!("entry {}\n", entry.len()).into_bytes();
        body.extend_from_slice(entry);
        body.push(b'\n');
        let request = self
            .http
            .post(self.url(&format!(
                "{name}/add?timeout={}",
                timeout_ms.min(MAX_WAIT_MS)
            )))
            .bearer_auth(token)
            .header(reqwest::header::CONTENT_TYPE, SUBMISSION_TYPE)
            .body(body);
        let (status, body) = self.send(request).await?;
        Self::answer(status, body)
    }

    /// `GET <log>/wait/<leaf hash>?timeout=<ms>`.
    pub async fn wait(
        &self,
        name: &str,
        leaf_hex: &str,
        timeout_ms: u64,
    ) -> Result<Answer, SidecarError> {
        let request = self.http.get(self.url(&format!(
            "{name}/wait/{leaf_hex}?timeout={}",
            timeout_ms.min(MAX_WAIT_MS)
        )));
        let (status, body) = self.send(request).await?;
        Self::answer(status, body)
    }

    /// `GET <log>/receipt/<leaf hash>`: the receipt against the checkpoint
    /// with the earliest quorum time that contains the entry, or `None`
    /// while no checkpoint that meets the quorum does.
    pub async fn receipt(
        &self,
        name: &str,
        leaf_hex: &str,
    ) -> Result<Option<String>, SidecarError> {
        let request = self
            .http
            .get(self.url(&format!("{name}/receipt/{leaf_hex}")));
        match self.expect(request, &[200, 404]).await? {
            (200, receipt) => Ok(Some(receipt)),
            _ => Ok(None),
        }
    }

    /// A voter route of a private log, forwarded after the application's
    /// login: `POST <log>/voter/<route>[?hub=<hub>]`.
    pub async fn voter(
        &self,
        name: &str,
        route: &str,
        body: &str,
        hub: Option<&str>,
    ) -> Result<Forwarded, SidecarError> {
        let mut request = self
            .http
            .post(self.url(&format!("{name}/voter/{route}")))
            .header(reqwest::header::CONTENT_TYPE, "text/plain; charset=utf-8")
            .body(body.to_string());
        if let Some(hub) = hub {
            request = request.query(&[("hub", hub)]);
        }
        let (status, body) = self.send(request).await?;
        Ok(Forwarded { status, body })
    }
}

/// Posts a log's enrolment note to a development witness's admin listener,
/// as the witness's operator does in production.
pub async fn post_enrolment(
    witness_admin_url: &str,
    admin_token: &str,
    note: &str,
) -> Result<(), SidecarError> {
    let http = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()
        .map_err(transport)?;
    let response = http
        .post(format!(
            "{}/admin/logs",
            witness_admin_url.trim_end_matches('/')
        ))
        .bearer_auth(admin_token)
        .header(reqwest::header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(note.to_string())
        .send()
        .await
        .map_err(transport)?;
    let status = response.status().as_u16();
    let body = response.text().await.map_err(transport)?;
    match status {
        200 | 201 => Ok(()),
        status => Err(SidecarError::Status {
            status,
            body: one_line(&body),
        }),
    }
}
