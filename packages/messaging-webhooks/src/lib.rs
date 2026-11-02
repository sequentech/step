// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The one public entrypoint for messaging provider callbacks.
//!
//! A thin relay: no database, no vault, no provider secrets and no
//! verification. Each callback is wrapped in a
//! [`messaging::relay::CallbackEnvelope`], signed with the relay secret and
//! posted to harvest's internal `POST /messaging/callbacks`; harvest's
//! answer goes back to the provider unchanged. Any other path is a 404 and
//! is never forwarded.
//!
//! Logs carry the provider, the method, the first characters of the key,
//! the status and the latency; never bodies, query values or header values.

#[macro_use]
extern crate rocket;

use anyhow::{Context, Result};
use chrono::Utc;
use messaging::relay::{
    self, CallbackEnvelope, CallbackMethod, CallbackProvider, RelayResponse, CALLBACKS_PATH,
    RELAY_SECRET_ENV, RELAY_SECRET_POLICY_ENV, RELAY_SIGNATURE_HEADER,
};
use rocket::data::{Data, ToByteUnit};
use rocket::http::{ContentType, Status};
use rocket::request::{FromRequest, Outcome, Request};
use rocket::response::{self, Responder, Response};
use rocket::{Build, Rocket, State};
use std::io::Cursor;
use std::time::{Duration, Instant};
use tracing::{info, warn};

/// Harvest's internal address, `host:port`, as Hasura's actions use it.
pub const HARVEST_DOMAIN_ENV: &str = "HARVEST_DOMAIN";

/// How long harvest has to answer a relayed callback.
pub const HARVEST_TIMEOUT: Duration = Duration::from_secs(10);

/// How many characters of a webhook key the logs show.
const LOGGED_KEY_CHARS: usize = 6;

/// What the component needs to run.
#[derive(Clone)]
pub struct Config {
    /// Harvest's `POST /messaging/callbacks`.
    pub callbacks_url: String,
    /// The shared relay secret ([`RELAY_SECRET_ENV`]).
    pub relay_secret: Vec<u8>,
    /// Larger provider bodies are answered 413 and not forwarded.
    pub max_body_bytes: u64,
    /// [`HARVEST_TIMEOUT`], shorter in tests.
    pub timeout: Duration,
    /// Callbacks relayed at the same time ([`relay::MAX_IN_FLIGHT_ENV`]);
    /// one more is answered 503 unread and the provider sends it again.
    pub max_in_flight: usize,
}

impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Config")
            .field("callbacks_url", &self.callbacks_url)
            .field("relay_secret", &"<redacted>")
            .field("max_body_bytes", &self.max_body_bytes)
            .field("timeout", &self.timeout)
            .field("max_in_flight", &self.max_in_flight)
            .finish()
    }
}

impl Config {
    /// From the process environment.
    ///
    /// # Errors
    /// See [`Config::from_vars`].
    pub fn from_env() -> Result<Self> {
        Self::from_vars(|name| std::env::var(name).ok())
    }

    /// From the variables `var` returns: [`HARVEST_DOMAIN_ENV`],
    /// [`RELAY_SECRET_ENV`] (checked by [`relay::relay_secret`] under
    /// [`RELAY_SECRET_POLICY_ENV`]), [`relay::MAX_BODY_BYTES_ENV`] and
    /// [`relay::MAX_IN_FLIGHT_ENV`].
    ///
    /// # Errors
    /// When the harvest address is missing, the relay secret is refused
    /// (unset, shorter than 32 bytes, or the development value without its
    /// policy), the body limit isn't a positive number of bytes, or the
    /// in-flight limit isn't a positive whole number.
    pub fn from_vars(var: impl Fn(&str) -> Option<String>) -> Result<Self> {
        let domain = var(HARVEST_DOMAIN_ENV)
            .map(|value| value.trim().trim_end_matches('/').to_string())
            .filter(|value| !value.is_empty())
            .with_context(|| format!("{HARVEST_DOMAIN_ENV} is not set"))?;
        let base = if domain.contains("://") {
            domain
        } else {
            format!("http://{domain}")
        };
        let relay_secret = relay::relay_secret(
            var(RELAY_SECRET_ENV).as_deref(),
            var(RELAY_SECRET_POLICY_ENV).as_deref(),
        )?;
        let max_body_bytes = relay::max_body_bytes(var(relay::MAX_BODY_BYTES_ENV).as_deref())?;
        let max_in_flight = relay::positive_count(
            relay::MAX_IN_FLIGHT_ENV,
            var(relay::MAX_IN_FLIGHT_ENV).as_deref(),
            relay::DEFAULT_MAX_IN_FLIGHT,
        )?;
        Ok(Self {
            callbacks_url: format!("{base}{CALLBACKS_PATH}"),
            relay_secret,
            max_body_bytes,
            timeout: HARVEST_TIMEOUT,
            max_in_flight,
        })
    }
}

/// Managed state: the configuration, one HTTP client to harvest and the
/// places for callbacks in flight.
struct Relay {
    config: Config,
    http: reqwest::Client,
    in_flight: std::sync::Arc<tokio::sync::Semaphore>,
}

/// Why a callback was answered 503: the provider retries it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Unavailable {
    /// No connection to harvest.
    Unreachable,
    /// Harvest didn't answer within the timeout.
    TimedOut,
    /// Harvest answered, but not HTTP 200 (for example, it refused the relay
    /// signature).
    Refused(u16),
    /// Harvest's HTTP 200 wasn't a usable [`RelayResponse`].
    BadAnswer,
    /// [`Config::max_in_flight`] callbacks are already being relayed; this
    /// one isn't read or forwarded.
    Busy,
}

impl std::fmt::Display for Unavailable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unreachable => write!(f, "Harvest unreachable"),
            Self::TimedOut => write!(f, "Harvest timed out"),
            Self::Refused(status) => write!(f, "Harvest refused the callback (HTTP {status})"),
            Self::BadAnswer => write!(f, "Harvest's answer is not a relay response"),
            Self::Busy => write!(f, "Too many callbacks in flight"),
        }
    }
}

/// The answer to the provider.
#[derive(Debug, PartialEq, Eq)]
pub struct Reply {
    status: Status,
    content_type: ContentType,
    body: String,
}

impl Reply {
    /// A status with an empty body.
    fn empty(status: Status) -> Self {
        Self {
            status,
            content_type: ContentType::Plain,
            body: String::new(),
        }
    }

    /// Harvest's answer, as is. A status outside 100–599 isn't one.
    fn relayed(answer: RelayResponse) -> Result<Self, Unavailable> {
        if !(100..=599).contains(&answer.status) {
            return Err(Unavailable::BadAnswer);
        }
        Ok(Self {
            status: Status::new(answer.status),
            content_type: ContentType::parse_flexible(&answer.content_type)
                .unwrap_or(ContentType::Plain),
            body: answer.body,
        })
    }
}

impl<'r> Responder<'r, 'static> for Reply {
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'static> {
        Response::build()
            .status(self.status)
            .header(self.content_type)
            .sized_body(self.body.len(), Cursor::new(self.body))
            .ok()
    }
}

/// The request's raw query and its headers. [`CallbackEnvelope::new`]
/// keeps only the forwarded ones.
struct Incoming {
    query: String,
    headers: Vec<(String, String)>,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Incoming {
    type Error = std::convert::Infallible;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        Outcome::Success(Incoming {
            query: request
                .uri()
                .query()
                .map(|query| query.as_str().to_string())
                .unwrap_or_default(),
            headers: request
                .headers()
                .iter()
                .map(|header| {
                    (
                        header.name().as_str().to_string(),
                        header.value().to_string(),
                    )
                })
                .collect(),
        })
    }
}

impl Relay {
    /// Posts the signed envelope to harvest and returns its answer.
    async fn forward(&self, envelope: &CallbackEnvelope) -> Result<Reply, Unavailable> {
        let payload = serde_json::to_vec(envelope).map_err(|_| Unavailable::BadAnswer)?;
        let signature = relay::sign(&self.config.relay_secret, Utc::now().timestamp(), &payload);
        let response = self
            .http
            .post(&self.config.callbacks_url)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .header(RELAY_SIGNATURE_HEADER, signature)
            .body(payload)
            .send()
            .await
            .map_err(|error| {
                if error.is_timeout() {
                    Unavailable::TimedOut
                } else {
                    Unavailable::Unreachable
                }
            })?;
        if response.status() != reqwest::StatusCode::OK {
            return Err(Unavailable::Refused(response.status().as_u16()));
        }
        let answer = response.json::<RelayResponse>().await.map_err(|error| {
            if error.is_timeout() {
                Unavailable::TimedOut
            } else {
                Unavailable::BadAnswer
            }
        })?;
        Reply::relayed(answer)
    }
}

/// The body, or the status that refuses it.
async fn read_body(data: Data<'_>, limit: u64) -> Result<Vec<u8>, Status> {
    let body = data
        .open(limit.bytes())
        .into_bytes()
        .await
        .map_err(|_| Status::BadRequest)?;
    if !body.is_complete() {
        return Err(Status::PayloadTooLarge);
    }
    Ok(body.into_inner())
}

/// Checks the key, reads the body, relays and logs one callback.
async fn relay_callback(
    relay: &Relay,
    provider: CallbackProvider,
    method: CallbackMethod,
    key: &str,
    incoming: Incoming,
    data: Option<Data<'_>>,
) -> Reply {
    let started = Instant::now();
    if !relay::is_webhook_key(key) {
        info!(%provider, %method, status = 404, "callback refused: not a webhook key");
        return Reply::empty(Status::NotFound);
    }
    let key_prefix = key.get(..LOGGED_KEY_CHARS).unwrap_or_default();
    // A place before the body is read (R7-03): a burst can't hold more
    // than `max_in_flight` harvest requests at once.
    let Ok(_place) = relay.in_flight.clone().try_acquire_owned() else {
        warn!(%provider, %method, key = key_prefix, status = 503,
            reason = %Unavailable::Busy, "callback not relayed; the provider will retry");
        return Reply::empty(Status::ServiceUnavailable);
    };
    let body = match data {
        Some(data) => match read_body(data, relay.config.max_body_bytes).await {
            Ok(body) => body,
            Err(status) => {
                info!(%provider, %method, key = key_prefix, status = status.code,
                    "callback refused: body over the limit or unreadable");
                return Reply::empty(status);
            }
        },
        None => Vec::new(),
    };
    let envelope = CallbackEnvelope::new(
        provider,
        key,
        method,
        &incoming.query,
        incoming
            .headers
            .iter()
            .map(|(name, value)| (name.as_str(), value.as_str())),
        &body,
        Utc::now(),
    );
    let reply = match relay.forward(&envelope).await {
        Ok(reply) => reply,
        Err(reason) => {
            warn!(%provider, %method, key = key_prefix, status = 503,
                latency_ms = started.elapsed().as_millis() as u64, %reason,
                "callback not relayed; the provider will retry");
            return Reply::empty(Status::ServiceUnavailable);
        }
    };
    info!(%provider, %method, key = key_prefix, status = reply.status.code,
        latency_ms = started.elapsed().as_millis() as u64, "callback relayed");
    reply
}

/// Meta's subscription handshake (`hub.mode`, `hub.verify_token`,
/// `hub.challenge`, in the raw query).
#[get("/webhooks/meta/<key>")]
async fn meta_subscription(key: &str, incoming: Incoming, relay: &State<Relay>) -> Reply {
    relay_callback(
        relay,
        CallbackProvider::META,
        CallbackMethod::GET,
        key,
        incoming,
        None,
    )
    .await
}

/// WhatsApp statuses, Messenger delivery and Get Started.
#[post("/webhooks/meta/<key>", data = "<data>")]
async fn meta_callback(
    key: &str,
    incoming: Incoming,
    data: Data<'_>,
    relay: &State<Relay>,
) -> Reply {
    relay_callback(
        relay,
        CallbackProvider::META,
        CallbackMethod::POST,
        key,
        incoming,
        Some(data),
    )
    .await
}

/// Infobip Viber delivery reports.
#[post("/webhooks/viber/<key>", data = "<data>")]
async fn viber_callback(
    key: &str,
    incoming: Incoming,
    data: Data<'_>,
    relay: &State<Relay>,
) -> Reply {
    relay_callback(
        relay,
        CallbackProvider::VIBER,
        CallbackMethod::POST,
        key,
        incoming,
        Some(data),
    )
    .await
}

/// SNS-wrapped SES events and SNS subscription confirmation. SNS posts
/// `text/plain`, so no content type is required.
#[post("/webhooks/aws/<key>", data = "<data>")]
async fn aws_callback(
    key: &str,
    incoming: Incoming,
    data: Data<'_>,
    relay: &State<Relay>,
) -> Reply {
    relay_callback(
        relay,
        CallbackProvider::AWS,
        CallbackMethod::POST,
        key,
        incoming,
        Some(data),
    )
    .await
}

/// Rocket answers `HEAD` with the matching `GET` route; Meta's handshake is
/// `GET` only, so every `HEAD` on a callback path is a 404 and is never
/// forwarded.
#[head("/webhooks/<_..>")]
fn no_head() -> Reply {
    Reply::empty(Status::NotFound)
}

/// Liveness. Never forwards.
#[get("/healthz")]
fn healthz() -> Reply {
    Reply::empty(Status::Ok)
}

/// Every other answer (unknown paths, methods): the status only, with no
/// server details.
#[catch(default)]
fn bare_status(status: Status, _: &Request<'_>) -> Reply {
    Reply::empty(status)
}

/// The component, ready to launch.
///
/// # Errors
/// When the HTTP client to harvest can't be built.
pub fn build(config: Config) -> Result<Rocket<Build>> {
    let http = reqwest::Client::builder()
        .timeout(config.timeout)
        .redirect(reqwest::redirect::Policy::none())
        .no_proxy()
        .build()
        .context("building the HTTP client to harvest")?;
    Ok(rocket::build()
        .mount(
            "/",
            routes![
                meta_subscription,
                meta_callback,
                viber_callback,
                aws_callback,
                healthz,
                no_head
            ],
        )
        .register("/", catchers![bare_status])
        .manage(Relay {
            in_flight: std::sync::Arc::new(tokio::sync::Semaphore::new(config.max_in_flight)),
            config,
            http,
        }))
}

pub mod logging;

#[cfg(test)]
mod tests;
