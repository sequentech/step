// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Messaging routes: the internal sending API for Keycloak, the provider
//! callbacks relayed by the webhook component, and the administrators'
//! account and event actions. No route here is public: providers reach
//! harvest only through the `messaging-webhooks` component.

use crate::services::authorization::authorize;
use crate::services::dependencies::HarvestServices;
use crate::services::otp_capacity::{Admitted, Arrival};
use crate::services::service_auth::VerifiedServiceClaims;
use chrono::{DateTime, Utc};
use deadpool_postgres::Client as DbClient;
use messaging::relay::{
    self, CallbackEnvelope, CallbackMethod, CallbackProvider, RelayAuthError,
    RelayResponse, RelaySecretRefusal, ReplayGuard, ReplayRefusal,
    RELAY_SECRET_ENV, RELAY_SECRET_POLICY_ENV, RELAY_SIGNATURE_HEADER,
};
use rocket::data::{Data, ToByteUnit};
use rocket::http::{RawStr, Status};
use rocket::request::{FromRequest, Outcome, Request};
use rocket::serde::json::Json;
use rocket::State;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::services::keycloak::update_realm_attributes;
use sequent_core::types::messaging::NoticeRecipient;
use sequent_core::types::messaging::{
    AccountCheck, AccountLimits, AccountSender, CreateMessengerLinkRequest,
    CreateMessengerLinkResponse, CredentialName, EventMessagingConfig,
    MessageAttemptState, MessageChannel, MessageContent, MessagePurpose,
    MessageReason, MessengerLinkRequest, MessengerLinkStatus, ProviderApproval,
    ReadinessPolicy, SendMessageRequest, SendMessageResponse,
    REALM_ATTR_MESSAGING,
};
use sequent_core::types::permissions::Permissions;
use sequent_core::types::voter_channel::{voter_channel, VoterChannel};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::str::FromStr;
use tracing::{error, info, instrument, warn};
use uuid::Uuid;
use windmill::postgres::messaging::{
    account_endpoint, check_account_update, get_messaging_account,
    identity_changed, insert_messaging_account,
    update_messaging_account_reentering, AccountSettings, MessagingAccount,
};
use windmill::services::messaging::accounts::{
    callback_blocker_reason, callback_blockers_with, callback_readiness_with,
    candidate_check_needed, candidate_refusal, check_account, check_candidate,
    delete_account, entered_credentials, generate_report_credentials,
    messenger_page_change_impact, replace_checked_credentials,
    replace_credentials, store_candidate_check, CredentialsOutcome,
    DeleteOutcome, StoredCheck, ACCOUNT_IN_USE_REASON,
    CANDIDATE_REFUSED_REASON, CHECK_DROPPED_REASON,
};
use windmill::services::messaging::audit::post_attempt_outcomes_or_warn;
use windmill::services::messaging::config::{
    projections_using_account, save_event_messaging_config_as, SaveOutcome,
};
use windmill::services::messaging::dispatch::{
    account_for_delivery, deliver_within, notice_recipient, record_refusal,
    Delivery, Dispatcher, NoticeTarget, Place, ProviderTemplate, ReadinessGate,
    Recipient, Refusal, SendPolicy,
};
use windmill::services::messaging::links::{
    confirm_link, create_link, link_status, send_bound_code, BoundCode,
    LinkError,
};
use windmill::services::messaging::webhooks::{
    aws_events, meta_callback, meta_subscription, viber_events, WebhookOutcome,
};
use windmill::tasks::send_template::find_voter;

type RouteError = (Status, String);

/// The refusal of a request that failed inside harvest. The cause is only
/// logged.
pub const INTERNAL_ERROR: &str = "internal error";

/// The refusal of an account action on an account the tenant doesn't have
/// (deleted meanwhile, or another tenant's).
pub const UNKNOWN_ACCOUNT: &str = "unknown account";

fn internal(error: anyhow::Error) -> RouteError {
    error!("messaging route failed: {error:#}");
    (Status::InternalServerError, INTERNAL_ERROR.to_string())
}

fn unknown_account() -> RouteError {
    (Status::NotFound, UNKNOWN_ACCOUNT.to_string())
}

/// Logged when a relayed callback found no connection of its share in
/// time: answered 503, so the provider retries it (R7-03).
pub const CALLBACKS_BUSY: &str =
    "Callback refused for now: callbacks hold their whole database share";

fn bad_request(error: impl std::fmt::Display) -> RouteError {
    (Status::BadRequest, error.to_string())
}

async fn db_client(
    services: &State<HarvestServices>,
) -> Result<DbClient, RouteError> {
    services
        .databases
        .hasura()
        .await
        .get()
        .await
        .map_err(|error| internal(anyhow::anyhow!("{error}")))
}

async fn dispatcher() -> Result<&'static Dispatcher, RouteError> {
    Dispatcher::global().await.map_err(internal)
}

fn parse_uuid(value: &str) -> Result<Uuid, RouteError> {
    Uuid::parse_str(value).map_err(|_| bad_request("invalid identifier"))
}

fn parse_time(value: &str) -> Result<DateTime<Utc>, RouteError> {
    DateTime::parse_from_rfc3339(value)
        .map(|d| d.with_timezone(&Utc))
        .map_err(|_| bad_request("invalid time"))
}

/// Sends one message for Keycloak: codes, and enrollment and credential
/// notices. Every one is interactive (`otp_capacity`): a place of its
/// account's, taken before any database connection is held for the send,
/// then a token of the whole bucket, within the budgets counted from the
/// request's arrival; never queued behind bulk sends.
#[instrument(skip_all)]
#[post("/messages/send", format = "json", data = "<body>")]
pub async fn send_message(
    // First: the budgets count from before the token check and the body.
    arrival: Arrival,
    claims: VerifiedServiceClaims,
    body: Json<SendMessageRequest>,
    services: &State<HarvestServices>,
) -> Result<Json<SendMessageResponse>, RouteError> {
    let request = body.into_inner();
    let tenant_id = claims.require_tenant(&request.tenant_id)?;
    if request.logical_key.trim().is_empty() {
        return Err(bad_request("missing logical_key"));
    }
    let expires_at =
        request.expires_at.as_deref().map(parse_time).transpose()?;
    if request.purpose == MessagePurpose::OTP && expires_at.is_none() {
        return Err(bad_request("codes need expires_at"));
    }
    let election_event_id = {
        let client = db_client(services).await?;
        claims
            .require_event(&client, request.election_event_id.as_deref())
            .await?
    };
    let logical_key = format!("keycloak:{}", request.logical_key);
    let destination = request
        .destination
        .clone()
        .filter(|destination| !destination.trim().is_empty());
    // Codes go where Keycloak is verifying them; notices go to the voter's
    // one channel, which harvest reads from the voter's record.
    let recipient = match request.purpose {
        MessagePurpose::OTP => match (request.channel, destination) {
            (Some(channel), Some(destination)) => Ok(NoticeTarget {
                channel,
                destination,
                // Messenger codes go through a link, bound to the
                // conversation of the account's own Page.
                messenger_page: None,
            }),
            _ => return Err(bad_request("codes need channel and destination")),
        },
        MessagePurpose::NOTICE => {
            let voter = match (request.recipient, request.voter_id.as_deref()) {
                (NoticeRecipient::VOTER_CHANNEL, Some(voter_id)) => {
                    voter_record(
                        services,
                        &request.tenant_id,
                        election_event_id.map(|id| id.to_string()).as_deref(),
                        voter_id,
                    )
                    .await?
                }
                _ => None,
            };
            notice_recipient(
                request.recipient,
                request.channel,
                destination.as_deref(),
                voter.as_ref(),
            )
        }
    };
    let result = match recipient {
        Ok(target) => {
            let delivery = Delivery {
                tenant_id,
                election_event_id,
                purpose: request.purpose,
                recipient: Recipient {
                    voter_id: request.voter_id.clone(),
                    channel: target.channel,
                    destination: target.destination,
                    // The voter's Posts: windmill refuses a channel they
                    // don't offer ("Channel not offered for notices"/"…for
                    // codes").
                    election_ids: request.election_ids.clone(),
                    // The voter's Page: a send through another Page's
                    // account is refused (R7-02).
                    messenger_page: target.messenger_page,
                },
                content: Some(request.content.clone()),
                language: request.language.clone(),
                template_alias: None,
                logical_key,
                expires_at,
                account_id: None,
                template_key: request.template_key.clone(),
                provider_template: None,
            };
            // The account, read on a connection given back before waiting
            // for one of its places.
            let account = {
                let mut client = db_client(services).await?;
                account_for_delivery(&mut client, &delivery)
                    .await
                    .map_err(internal)?
            };
            let admitted = match &account {
                Some(account) => {
                    services
                        .otp_capacity
                        .admit(account.id, &account.limits, arrival)
                        .await
                }
                // The environment's email and SMS have no account limits.
                None => Admitted {
                    place: Place::NOT_REQUIRED,
                    permit: None,
                },
            };
            let mut client = db_client(services).await?;
            let result = deliver_within(
                &mut client,
                dispatcher().await?,
                &delivery,
                services.otp_capacity.policy(arrival, admitted.place),
            )
            .await;
            // The place is kept until the send is recorded.
            drop(admitted);
            result
        }
        // One FAILED row with the reason: nothing goes anywhere else.
        Err((channel, reason)) => {
            info!(%reason, "notice refused before sending");
            let mut client = db_client(services).await?;
            record_refusal(
                &mut client,
                &Refusal {
                    tenant_id,
                    election_event_id,
                    voter_id: request.voter_id.clone(),
                    channel,
                    destination: request.destination.clone(),
                    purpose: request.purpose,
                    template_alias: None,
                    language: request.language.clone(),
                    logical_key,
                    reason,
                },
            )
            .await
        }
    }
    .map_err(internal)?;
    // The attempt's electoral log entry, off the code's response path.
    if !result.recorded.is_empty() {
        let databases = services.databases.clone();
        let recorded = result.recorded.clone();
        tokio::spawn(async move {
            let Ok(mut client) = databases.hasura().await.get().await else {
                return;
            };
            let Ok(tx) = client.transaction().await else {
                return;
            };
            post_attempt_outcomes_or_warn(&tx, &recorded, None, None).await;
            let _ = tx.commit().await;
        });
    }
    Ok(Json(SendMessageResponse {
        message_id: result
            .message_id
            .map(|id| id.to_string())
            .unwrap_or_default(),
        // The ledger's state and reason, also for a send past its capacity
        // budget (FAILED, `CAPACITY_BUSY`) or refused by its account's
        // readiness: Keycloak shows Resend after its timer and records the
        // reason.
        state: result.state,
        reason: result.reason,
    }))
}

/// The voter's one channel from their record in Keycloak, or `None` when
/// the voter isn't found or has no channel.
async fn voter_record(
    services: &State<HarvestServices>,
    tenant_id: &str,
    election_event_id: Option<&str>,
    voter_id: &str,
) -> Result<Option<VoterChannel>, RouteError> {
    let mut hasura = db_client(services).await?;
    let mut keycloak = services
        .databases
        .keycloak()
        .await
        .get()
        .await
        .map_err(|error| internal(anyhow::anyhow!("{error}")))?;
    let hasura_tx =
        hasura.transaction().await.map_err(|e| internal(e.into()))?;
    let keycloak_tx = keycloak
        .transaction()
        .await
        .map_err(|e| internal(e.into()))?;
    let user = find_voter(
        &hasura_tx,
        &keycloak_tx,
        tenant_id,
        election_event_id,
        voter_id,
    )
    .await
    .map_err(internal)?;
    keycloak_tx.commit().await.map_err(|e| internal(e.into()))?;
    hasura_tx.commit().await.map_err(|e| internal(e.into()))?;
    let found = user.and_then(|user| {
        voter_channel(user.email.as_deref(), user.attributes.as_ref())
    });
    if found.is_none() {
        info!("the voter has no recorded channel: the request's is used");
    }
    Ok(found)
}

fn link_error(error: LinkError) -> RouteError {
    match error {
        LinkError::MessengerNotEnabled => (
            Status::UnprocessableEntity,
            "Messenger is not enabled for codes".to_string(),
        ),
        LinkError::NotFound => (Status::NotFound, "unknown link".to_string()),
        // Keycloak may then start the sign-in reconnect (R7-10).
        LinkError::MessengerPageChanged => (
            Status::Conflict,
            MessageReason::MESSENGER_PAGE_CHANGED.to_string(),
        ),
        LinkError::CallbackNotConfigured => (
            Status::UnprocessableEntity,
            windmill::services::messaging::accounts::CALLBACK_NOT_CONFIGURED
                .to_string(),
        ),
    }
}

#[instrument(skip_all)]
#[post("/messages/link", format = "json", data = "<body>")]
pub async fn create_messenger_link(
    claims: VerifiedServiceClaims,
    body: Json<CreateMessengerLinkRequest>,
    services: &State<HarvestServices>,
) -> Result<Json<CreateMessengerLinkResponse>, RouteError> {
    let request = body.into_inner();
    claims.require_tenant(&request.tenant_id)?;
    let mut client = db_client(services).await?;
    claims
        .require_event(&client, request.election_event_id.as_deref())
        .await?;
    create_link(
        &mut client,
        &request,
        services.messaging_webhooks_url.as_deref(),
    )
    .await
    .map_err(|error| bad_request(error))?
    .map(Json)
    .map_err(link_error)
}

#[instrument(skip_all)]
#[post("/messages/link/status", format = "json", data = "<body>")]
pub async fn messenger_link_status(
    claims: VerifiedServiceClaims,
    body: Json<MessengerLinkRequest>,
    services: &State<HarvestServices>,
) -> Result<Json<MessengerLinkStatus>, RouteError> {
    let request = body.into_inner();
    claims.require_tenant(&request.tenant_id)?;
    let mut client = db_client(services).await?;
    link_status(&mut client, &request)
        .await
        .map_err(internal)?
        .map(Json)
        .map_err(link_error)
}

#[instrument(skip_all)]
#[post("/messages/link/confirm", format = "json", data = "<body>")]
pub async fn confirm_messenger_link(
    claims: VerifiedServiceClaims,
    body: Json<MessengerLinkRequest>,
    services: &State<HarvestServices>,
) -> Result<Json<MessengerLinkStatus>, RouteError> {
    let request = body.into_inner();
    claims.require_tenant(&request.tenant_id)?;
    let mut client = db_client(services).await?;
    confirm_link(&mut client, &request)
        .await
        .map_err(internal)?
        .map(Json)
        .map_err(link_error)
}

/// How harvest takes provider callbacks relayed by the webhook component
/// (`POST /messaging/callbacks`). Read once, into [`HarvestServices`].
#[derive(Clone)]
pub struct MessagingRelay {
    secret: RelaySecret,
    /// The largest envelope read: [`relay::envelope_limit`] of the
    /// component's body limit.
    envelope_limit: u64,
    /// Relay signatures already taken while fresh: a captured relayed
    /// request isn't taken twice.
    replays: std::sync::Arc<ReplayGuard>,
    /// Callbacks' own share of the database pool that codes use too
    /// (R7-03, [`relay::CALLBACK_CONNECTIONS_ENV`]).
    callbacks: std::sync::Arc<tokio::sync::Semaphore>,
    /// How long a callback waits for its share before it is answered 503.
    callback_wait: std::time::Duration,
}

/// Whether harvest can check relay signatures.
#[derive(Clone)]
enum RelaySecret {
    /// `MESSAGING_RELAY_SECRET` is unset or refused: every relayed callback
    /// is answered 503, so the component makes the provider retry.
    NotConfigured(RelaySecretRefusal),
    /// The secret shared with the component.
    Configured(Vec<u8>),
}

impl MessagingRelay {
    /// `secret`, already checked (none or empty: not configured), and the
    /// component's body limit.
    pub fn new(secret: Option<&[u8]>, max_body_bytes: u64) -> Self {
        Self {
            secret: match secret.filter(|secret| !secret.is_empty()) {
                Some(secret) => RelaySecret::Configured(secret.to_vec()),
                None => RelaySecret::NotConfigured(RelaySecretRefusal::NOT_SET),
            },
            envelope_limit: relay::envelope_limit(max_body_bytes),
            replays: Default::default(),
            callbacks: std::sync::Arc::new(tokio::sync::Semaphore::new(
                relay::DEFAULT_CALLBACK_CONNECTIONS,
            )),
            callback_wait: std::time::Duration::from_millis(
                relay::CALLBACK_CONNECTION_WAIT_MS,
            ),
        }
    }

    /// Callbacks hold at most `connections` database connections at once,
    /// each waiting at most `wait` for one.
    pub fn with_callback_share(
        mut self,
        connections: usize,
        wait: std::time::Duration,
    ) -> Self {
        self.callbacks =
            std::sync::Arc::new(tokio::sync::Semaphore::new(connections));
        self.callback_wait = wait;
        self
    }

    /// From the variables `var` returns: `MESSAGING_RELAY_SECRET`, checked
    /// by [`relay::relay_secret`] under `MESSAGING_RELAY_SECRET_POLICY`
    /// (the same rule as the component's), and
    /// `MESSAGING_WEBHOOKS_MAX_BODY_BYTES`.
    pub fn from_vars(var: impl Fn(&str) -> Option<String>) -> Self {
        let max_body_bytes =
            relay::max_body_bytes(var(relay::MAX_BODY_BYTES_ENV).as_deref())
                .unwrap_or_else(|error| {
                    error!(
                        "{error:#}; using {} bytes",
                        relay::DEFAULT_MAX_BODY_BYTES
                    );
                    relay::DEFAULT_MAX_BODY_BYTES
                });
        let connections = relay::positive_count(
            relay::CALLBACK_CONNECTIONS_ENV,
            var(relay::CALLBACK_CONNECTIONS_ENV).as_deref(),
            relay::DEFAULT_CALLBACK_CONNECTIONS,
        )
        .unwrap_or_else(|error| {
            error!("{error:#}; using {}", relay::DEFAULT_CALLBACK_CONNECTIONS);
            relay::DEFAULT_CALLBACK_CONNECTIONS
        });
        let mut relay = Self::new(None, max_body_bytes).with_callback_share(
            connections,
            std::time::Duration::from_millis(
                relay::CALLBACK_CONNECTION_WAIT_MS,
            ),
        );
        match relay::relay_secret(
            var(RELAY_SECRET_ENV).as_deref(),
            var(RELAY_SECRET_POLICY_ENV).as_deref(),
        ) {
            Ok(secret) => relay.secret = RelaySecret::Configured(secret),
            Err(refusal) => {
                warn!("Callbacks refused: {refusal}");
                relay.secret = RelaySecret::NotConfigured(refusal);
            }
        }
        relay
    }

    /// From the process environment ([`MessagingRelay::from_vars`]).
    pub fn from_env() -> Self {
        Self::from_vars(|name| std::env::var(name).ok())
    }
}

/// A relay signature header that is present, well formed and fresh. Checked
/// before the body is read; the MAC is checked once it is.
pub struct RelaySignature(String);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for RelaySignature {
    type Error = RelayAuthError;

    async fn from_request(
        request: &'r Request<'_>,
    ) -> Outcome<Self, Self::Error> {
        let header = request.headers().get_one(RELAY_SIGNATURE_HEADER);
        match relay::check_header(header, Utc::now().timestamp()) {
            Ok(()) => Outcome::Success(RelaySignature(
                header.unwrap_or_default().to_string(),
            )),
            Err(reason) => {
                warn!(%reason, "relayed callback refused");
                Outcome::Error((Status::Unauthorized, reason))
            }
        }
    }
}

fn webhook_status(outcome: WebhookOutcome) -> Status {
    match outcome {
        WebhookOutcome::Accepted => Status::Ok,
        WebhookOutcome::Unauthorized => Status::Unauthorized,
        WebhookOutcome::NotFound => Status::NotFound,
    }
}

/// What the provider gets: the status, with no body.
fn provider_answer(status: Status) -> RelayResponse {
    RelayResponse::text(status.code, "")
}

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

/// Meta's `hub.mode`, `hub.verify_token` and `hub.challenge`, from the raw
/// query of the handshake.
#[derive(Default, Debug, PartialEq, Eq)]
struct MetaHub {
    mode: Option<String>,
    verify_token: Option<String>,
    challenge: Option<String>,
}

impl MetaHub {
    fn parse(query: &str) -> Self {
        let mut hub = Self::default();
        for pair in query.split('&').filter(|pair| !pair.is_empty()) {
            let (name, value) = pair.split_once('=').unwrap_or((pair, ""));
            let value = RawStr::new(value).url_decode_lossy().into_owned();
            match RawStr::new(name).url_decode_lossy().as_ref() {
                "hub.mode" => hub.mode = Some(value),
                "hub.verify_token" => hub.verify_token = Some(value),
                "hub.challenge" => hub.challenge = Some(value),
                _ => {}
            }
        }
        hub
    }
}

/// Runs the provider's own verification and parsing on the original bytes.
/// The account comes from the envelope's key only, never from the payload.
/// `Err` is a failure the provider should retry.
/// Sends a code a Get Started released, after the callback was answered:
/// spawned, with its own place, bucket and budget counted from now. Its
/// outcome is in the ledger (and the electoral log); if harvest stops
/// first, the link stays bound without CODE_SENT and the voter taps Get
/// Started again.
fn send_code_after_answer(services: &State<HarvestServices>, code: BoundCode) {
    let databases = services.databases.clone();
    let capacity = services.otp_capacity.clone();
    tokio::spawn(async move {
        let arrival = Arrival(tokio::time::Instant::now());
        let admitted = capacity
            .admit(code.account.id, &code.account.limits, arrival)
            .await;
        let sent = async {
            let mut client = databases
                .hasura()
                .await
                .get()
                .await
                .map_err(|error| anyhow::anyhow!("{error}"))?;
            let dispatcher = Dispatcher::global().await?;
            send_bound_code(
                &mut client,
                dispatcher,
                &code,
                capacity.policy(arrival, admitted.place),
            )
            .await
        }
        .await;
        drop(admitted);
        if let Err(error) = sent {
            warn!(link = %code.link_id, "could not send a Messenger code: {error:#}");
        }
    });
}

async fn provider_callback(
    envelope: &CallbackEnvelope,
    services: &State<HarvestServices>,
) -> Result<RelayResponse, Status> {
    let key = envelope.key.as_str();
    if !relay::is_webhook_key(key) {
        return Ok(provider_answer(Status::NotFound));
    }
    let body = envelope.body().map_err(|_| Status::BadRequest)?;
    // Callbacks' own share of the pool, taken before a connection (R7-03):
    // public requests can't take the connections codes need. Held until
    // the answer; the codes a Get Started releases are sent afterwards
    // with places of their own.
    let relay_config = &services.messaging_relay;
    let _share = match tokio::time::timeout(
        relay_config.callback_wait,
        relay_config.callbacks.clone().acquire_owned(),
    )
    .await
    {
        Ok(Ok(permit)) => permit,
        _ => {
            warn!(provider = ?envelope.provider, "{CALLBACKS_BUSY}");
            return Err(Status::ServiceUnavailable);
        }
    };
    let mut client = db_client(services)
        .await
        .map_err(|_| Status::ServiceUnavailable)?;
    let provider = envelope.provider;
    let outcome = match (provider, envelope.method) {
        (CallbackProvider::META, CallbackMethod::GET) => {
            let hub = MetaHub::parse(&envelope.query);
            return match meta_subscription(
                &mut client,
                key,
                hub.mode.as_deref(),
                hub.verify_token.as_deref(),
                hub.challenge.as_deref(),
            )
            .await
            {
                Ok(Some(challenge)) => Ok(RelayResponse::text(200, challenge)),
                Ok(None) => Ok(provider_answer(Status::Forbidden)),
                Err(error) => {
                    warn!("Meta subscription failed: {error:#}");
                    Err(Status::InternalServerError)
                }
            };
        }
        (CallbackProvider::META, CallbackMethod::POST) => {
            // Answered once the events are stored and the links bound; the
            // codes those links release are sent after the answer.
            match meta_callback(
                &mut client,
                key,
                envelope.header("x-hub-signature-256"),
                &body,
            )
            .await
            {
                Ok((outcome, codes)) => {
                    drop(client);
                    for code in codes {
                        send_code_after_answer(services, code);
                    }
                    Ok(outcome)
                }
                Err(error) => Err(error),
            }
        }
        (CallbackProvider::VIBER, CallbackMethod::POST) => {
            let dispatcher =
                dispatcher().await.map_err(|_| Status::ServiceUnavailable)?;
            viber_events(
                &mut client,
                dispatcher,
                key,
                envelope.header("authorization"),
                &body,
            )
            .await
        }
        (CallbackProvider::AWS, CallbackMethod::POST) => {
            let dispatcher =
                dispatcher().await.map_err(|_| Status::ServiceUnavailable)?;
            aws_events(&mut client, dispatcher, key, &body).await
        }
        (_, CallbackMethod::GET) => {
            return Ok(provider_answer(Status::MethodNotAllowed))
        }
    };
    match outcome {
        Ok(outcome) => Ok(provider_answer(webhook_status(outcome))),
        Err(error) => {
            warn!("{provider} callback failed: {error:#}");
            Err(Status::InternalServerError)
        }
    }
}

/// Provider callbacks relayed by the webhook component, the only public
/// entrypoint for them. Authenticated by the relay signature only. HTTP 200
/// carries the provider's answer; any other status makes the component
/// answer 503, so the provider retries.
#[instrument(skip_all)]
#[post("/messaging/callbacks", data = "<data>")]
pub async fn messaging_callbacks(
    signature: RelaySignature,
    data: Data<'_>,
    services: &State<HarvestServices>,
) -> Result<Json<RelayResponse>, Status> {
    let relay_config = &services.messaging_relay;
    let secret = match &relay_config.secret {
        RelaySecret::Configured(secret) => secret,
        RelaySecret::NotConfigured(refusal) => {
            error!("Callbacks refused: {refusal}");
            return Err(Status::ServiceUnavailable);
        }
    };
    let bytes = read_body(data, relay_config.envelope_limit).await?;
    let now = Utc::now().timestamp();
    let verified = match relay::verify_signature(
        secret,
        Some(&signature.0),
        &bytes,
        now,
    ) {
        Ok(verified) => verified,
        Err(reason) => {
            warn!(%reason, "relayed callback refused");
            return Err(Status::Unauthorized);
        }
    };
    let envelope: CallbackEnvelope =
        serde_json::from_slice(&bytes).map_err(|_| {
            warn!("relayed callback refused: not a callback envelope");
            Status::BadRequest
        })?;
    if envelope.version != relay::ENVELOPE_VERSION {
        warn!(
            version = envelope.version,
            "relayed callback refused: unknown envelope version"
        );
        return Err(Status::BadRequest);
    }
    // The component signs what it just received.
    if let Err(reason) = envelope.check_received_at(verified.timestamp) {
        warn!(%reason, "relayed callback refused");
        return Err(Status::Unauthorized);
    }
    // Taken once: a captured relayed request can't be sent again while its
    // signature is fresh.
    match relay_config.replays.admit(&verified, now) {
        Ok(()) => {}
        Err(ReplayRefusal::Replayed) => {
            warn!("relayed callback refused: already taken");
            return Err(Status::Unauthorized);
        }
        Err(ReplayRefusal::Busy) => {
            warn!(
                "relayed callback refused for now: too many fresh signatures"
            );
            return Err(Status::ServiceUnavailable);
        }
    }
    let answer = provider_callback(&envelope, services).await?;
    info!(
        provider = %envelope.provider,
        method = %envelope.method,
        key = envelope.key.get(..6).unwrap_or_default(),
        status = answer.status,
        "provider callback answered"
    );
    Ok(Json(answer))
}

/// A refusal of the account actions, as `{"message": reason}`: Hasura
/// passes the reason on to the Admin Portal, which shows it.
#[derive(Debug, PartialEq, Eq)]
pub struct JsonRefusal(Status, String);

impl From<RouteError> for JsonRefusal {
    fn from((status, reason): RouteError) -> Self {
        Self(status, reason)
    }
}

impl<'r> rocket::response::Responder<'r, 'static> for JsonRefusal {
    fn respond_to(
        self,
        request: &'r Request<'_>,
    ) -> rocket::response::Result<'static> {
        (self.0, Json(serde_json::json!({"message": self.1})))
            .respond_to(request)
    }
}

// No `Debug`: it may carry credentials.
#[derive(Deserialize)]
pub struct UpsertMessagingAccountInput {
    id: Option<String>,
    channel: Option<MessageChannel>,
    name: String,
    sender: AccountSender,
    limits: Option<AccountLimits>,
    provider_approval: Option<ProviderApproval>,
    /// Parse-only: every account is checked by its provider.
    readiness: Option<ReadinessPolicy>,
    is_default: Option<bool>,
    /// Credentials entered with the change. Needed when the address the
    /// credentials are sent to changes (`ENDPOINT_CREDENTIALS_REASON`).
    credentials: Option<HashMap<String, String>>,
}

#[derive(Serialize, Debug)]
pub struct MessagingAccountOutput {
    id: String,
    /// Where the provider sends callbacks, when it sends any and the
    /// webhook component's address is configured.
    #[serde(skip_serializing_if = "Option::is_none")]
    callback_url: Option<String>,
    /// Why there is no callback URL for a provider that needs one.
    #[serde(skip_serializing_if = "Option::is_none")]
    callback_reason: Option<String>,
}

impl MessagingAccountOutput {
    fn of(account: &MessagingAccount, component_url: Option<&str>) -> Self {
        let readiness = callback_readiness_with(
            component_url,
            account.provider,
            &account.webhook_key,
        );
        Self {
            id: account.id.to_string(),
            callback_url: readiness.url().map(str::to_string),
            callback_reason: readiness.reason().map(str::to_string),
        }
    }
}

fn authorize_admin(
    claims: &JwtClaims,
    permission: Permissions,
) -> Result<Uuid, RouteError> {
    authorize(
        claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![permission],
    )?;
    parse_uuid(&claims.hasura_claims.tenant_id)
}

#[instrument(skip_all)]
#[post("/messaging/accounts/upsert", format = "json", data = "<body>")]
pub async fn upsert_messaging_account(
    claims: JwtClaims,
    body: Json<UpsertMessagingAccountInput>,
    services: &State<HarvestServices>,
) -> Result<Json<MessagingAccountOutput>, JsonRefusal> {
    let tenant_id =
        authorize_admin(&claims, Permissions::MESSAGING_ACCOUNT_WRITE)?;
    let input = body.into_inner();
    // Providers with a channel of their own fix it; configurable ones take
    // the channel the administrator chose.
    let channel = input
        .sender
        .provider()
        .channel()
        .or(input.channel)
        .ok_or_else(|| bad_request("this provider needs a channel"))?;
    let values = parse_credentials(input.credentials.unwrap_or_default())?;
    let settings = AccountSettings {
        channel,
        name: input.name,
        sender: input.sender,
        limits: input.limits.unwrap_or_default(),
        provider_approval: input.provider_approval.unwrap_or_default(),
        // Parse-only: readiness is the server's (an identity change leaves
        // the account unchecked until a check of its new identity runs).
        readiness: ReadinessPolicy::PROVIDER_CHECK,
        is_default: input.is_default.unwrap_or(false),
    };
    let mut client = db_client(services).await?;
    let tx = client.transaction().await.map_err(|e| internal(e.into()))?;
    let reentered: BTreeSet<CredentialName> = values.keys().copied().collect();
    let id = input.id.as_deref().map(parse_uuid).transpose()?;
    // Credentials typed to replace stored ones are checked first, with the
    // settings saved with them; nothing is written unless the check
    // connects, so the stored ones stay in use (BC-F11).
    let mut candidate_check = None;
    let mut identity_change = false;
    let mut public_change = false;
    if let Some(id) = id {
        let current = get_messaging_account(&tx, &tenant_id, &id)
            .await
            .map_err(internal)?
            .ok_or_else(unknown_account)?;
        identity_change = identity_changed(&current.sender, &settings.sender);
        public_change = current.sender != settings.sender;
        // Settings and values the account can't take are refused for what
        // they are, before any provider is called (R5-04): an address
        // that isn't https, a provider or channel change, a new address
        // without its credentials.
        check_account_update(&current, &settings, &reentered)
            .map_err(bad_request)?;
        entered_credentials(&current, &values).map_err(bad_request)?;
        // A candidate check against a new address answers only "didn't
        // connect": what an address answered isn't shown (R5-04).
        let new_address = account_endpoint(&current.sender)
            != account_endpoint(&settings.sender);
        if candidate_check_needed(&current, &values) {
            let check = check_candidate(
                &tx,
                &current,
                &settings.sender,
                &values,
                &dispatcher().await?.endpoints,
            )
            .await
            .map_err(internal)?;
            if let Some(reason) = candidate_refusal(&check, current.provider) {
                warn!(account = %id, "account not saved: {reason}");
                return Err(bad_request(if new_address {
                    CANDIDATE_REFUSED_REASON.to_string()
                } else {
                    reason
                })
                .into());
            }
            candidate_check = Some(check);
        }
    }
    let account = match id {
        Some(id) => update_messaging_account_reentering(
            &tx, &tenant_id, &id, &settings, &reentered,
        )
        .await
        .map_err(bad_request)?
        .ok_or_else(unknown_account)?,
        None => insert_messaging_account(
            &tx,
            &tenant_id,
            &settings,
            &messaging::link::new_reference(),
        )
        .await
        .map_err(bad_request)?,
    };
    // Stored in the same transaction as the address they go with.
    if !values.is_empty() {
        replace_credentials(&tx, &tenant_id, &account.id, &values, false)
            .await
            .map_err(bad_request)?;
    }
    // The candidate's check describes the account as saved: it is its
    // status, and readiness comes from it again.
    if let Some(check) = &candidate_check {
        store_candidate_check(&tx, &account, check.clone())
            .await
            .map_err(internal)?;
    }
    tx.commit().await.map_err(|e| internal(e.into()))?;
    // After the answer, not inside the Hasura action (R5-18): an identity
    // change left the account unchecked, and it is checked in the
    // background, in its own transaction; a check that can't run leaves it
    // unchecked (shown with its reason) for the daily pass to retry
    // (AP-B3). The events whose settings send through the account publish
    // its new Page or label (R5-10).
    let recheck = identity_change && candidate_check.is_none();
    if recheck || public_change {
        let databases = services.databases.clone();
        let account_id = account.id;
        tokio::spawn(async move {
            let mut client = match databases.hasura().await.get().await {
                Ok(client) => client,
                Err(error) => {
                    warn!(
                        account = %account_id,
                        "account change not followed up: {error}"
                    );
                    return;
                }
            };
            if recheck {
                recheck_after_identity_change(
                    &mut client,
                    &tenant_id,
                    &account_id,
                )
                .await;
            }
            if public_change {
                republish_projections(&mut client, &tenant_id, &account_id)
                    .await;
            }
        });
    }
    Ok(Json(MessagingAccountOutput::of(
        &account,
        services.messaging_webhooks_url.as_deref(),
    )))
}

/// Publishes again the public projection of every election event whose
/// settings send through the account, after its sender changed (R5-10):
/// the voters' pages name and link the account's current Page and label.
/// A realm that can't be updated is logged; the next save of the event's
/// settings publishes it.
async fn republish_projections(
    client: &mut DbClient,
    tenant_id: &Uuid,
    account_id: &Uuid,
) {
    let projections = async {
        let tx = client.transaction().await?;
        let projections =
            projections_using_account(&tx, tenant_id, account_id).await?;
        tx.commit().await?;
        anyhow::Ok(projections)
    }
    .await;
    let projections = match projections {
        Ok(projections) => projections,
        Err(error) => {
            warn!(
                account = %account_id,
                "messaging settings not published again: {error:#}"
            );
            return;
        }
    };
    for (event_id, projection) in projections {
        let published = async {
            update_realm_attributes(
                &tenant_id.to_string(),
                &event_id.to_string(),
                HashMap::from([(
                    REALM_ATTR_MESSAGING.to_string(),
                    serde_json::to_string(&projection)?,
                )]),
            )
            .await
        }
        .await;
        match published {
            Ok(()) => info!(
                account = %account_id,
                event = %event_id,
                "messaging settings published again after the account changed"
            ),
            Err(error) => warn!(
                account = %account_id,
                event = %event_id,
                "messaging settings not published again: {error:#}"
            ),
        }
    }
}

/// Runs the check of an account whose identity just changed. A check that
/// can't run is logged; the account stays unchecked.
async fn recheck_after_identity_change(
    client: &mut DbClient,
    tenant_id: &Uuid,
    account_id: &Uuid,
) {
    let result = async {
        let endpoints = &dispatcher()
            .await
            .map_err(|(_, e)| anyhow::anyhow!(e))?
            .endpoints;
        let tx = client.transaction().await?;
        let check =
            check_account(&tx, tenant_id, account_id, endpoints).await?;
        tx.commit().await?;
        anyhow::Ok(check)
    }
    .await;
    match result {
        Ok(Some(StoredCheck { check, stored })) => info!(
            account = %account_id,
            stored,
            connected = check.connected,
            reason = check.reason.as_deref().unwrap_or("-"),
            "account checked after its identity changed"
        ),
        Ok(None) => {}
        Err(error) => warn!(
            account = %account_id,
            "account left unchecked after its identity changed: {error:#}"
        ),
    }
}

fn parse_credentials(
    values: HashMap<String, String>,
) -> Result<BTreeMap<CredentialName, String>, RouteError> {
    values
        .into_iter()
        .map(|(name, value)| {
            CredentialName::from_str(&name)
                .map(|name| (name, value))
                .map_err(|_| bad_request(format!("unknown credential {name}")))
        })
        .collect()
}

#[derive(Deserialize, Debug)]
pub struct AccountIdInput {
    id: String,
}

#[instrument(skip_all)]
#[post("/messaging/accounts/delete", format = "json", data = "<body>")]
pub async fn delete_messaging_account(
    claims: JwtClaims,
    body: Json<AccountIdInput>,
    services: &State<HarvestServices>,
) -> Result<Json<MessagingAccountOutput>, JsonRefusal> {
    let tenant_id =
        authorize_admin(&claims, Permissions::MESSAGING_ACCOUNT_WRITE)?;
    let id = parse_uuid(&body.id)?;
    let mut client = db_client(services).await?;
    let tx = client.transaction().await.map_err(|e| internal(e.into()))?;
    match delete_account(&tx, &tenant_id, &id)
        .await
        .map_err(internal)?
    {
        DeleteOutcome::Deleted => {}
        DeleteOutcome::NotFound => return Err(unknown_account().into()),
        // Refused while an election event's settings send through it.
        DeleteOutcome::InUse(_) => {
            return Err(
                (Status::Conflict, ACCOUNT_IN_USE_REASON.to_string()).into()
            )
        }
    }
    tx.commit().await.map_err(|e| internal(e.into()))?;
    Ok(Json(MessagingAccountOutput {
        id: id.to_string(),
        callback_url: None,
        callback_reason: None,
    }))
}

#[derive(Serialize, Debug, PartialEq, Eq)]
pub struct CallbackUrlOutput {
    callback_url: Option<String>,
    reason: Option<String>,
}

/// The full callback URL an administrator enters at the provider, or why
/// there is none. Read when the account drawer opens: the webhook key itself
/// is never readable through Hasura.
#[instrument(skip_all)]
#[post("/messaging/accounts/callback-url", format = "json", data = "<body>")]
pub async fn messaging_account_callback_url(
    claims: JwtClaims,
    body: Json<AccountIdInput>,
    services: &State<HarvestServices>,
) -> Result<Json<CallbackUrlOutput>, JsonRefusal> {
    let tenant_id =
        authorize_admin(&claims, Permissions::MESSAGING_ACCOUNT_WRITE)?;
    let id = parse_uuid(&body.id)?;
    let mut client = db_client(services).await?;
    let tx = client.transaction().await.map_err(|e| internal(e.into()))?;
    let account = get_messaging_account(&tx, &tenant_id, &id)
        .await
        .map_err(internal)?
        .ok_or_else(unknown_account)?;
    tx.commit().await.map_err(|e| internal(e.into()))?;
    let readiness = callback_readiness_with(
        services.messaging_webhooks_url.as_deref(),
        account.provider,
        &account.webhook_key,
    );
    Ok(Json(CallbackUrlOutput {
        callback_url: readiness.url().map(str::to_string),
        reason: readiness.reason().map(str::to_string),
    }))
}

#[derive(Deserialize, Debug)]
pub struct ChangeImpactInput {
    account_id: String,
    /// The Page the account would move to; any other Page when absent.
    page_id: Option<String>,
}

#[derive(Serialize, Debug, PartialEq, Eq)]
pub struct ChangeImpactOutput {
    /// Voters whose `messenger-page` is the account's current Page, in the
    /// realms of the events that use the account: they connect again after
    /// a Page change.
    affected_voters: i64,
    /// How many election events use the account.
    events: usize,
}

/// What changing a Messenger account's Page does: how many voters must
/// connect again (BC-F13). The drawer warns with the count before saving.
#[instrument(skip_all)]
#[post("/messaging/accounts/change-impact", format = "json", data = "<body>")]
pub async fn messaging_account_change_impact(
    claims: JwtClaims,
    body: Json<ChangeImpactInput>,
    services: &State<HarvestServices>,
) -> Result<Json<ChangeImpactOutput>, JsonRefusal> {
    let tenant_id =
        authorize_admin(&claims, Permissions::MESSAGING_ACCOUNT_WRITE)?;
    let input = body.into_inner();
    let id = parse_uuid(&input.account_id)?;
    let mut client = db_client(services).await?;
    let tx = client.transaction().await.map_err(|e| internal(e.into()))?;
    let mut keycloak = services
        .databases
        .keycloak()
        .await
        .get()
        .await
        .map_err(|error| internal(anyhow::anyhow!("{error}")))?;
    let keycloak_tx = keycloak
        .transaction()
        .await
        .map_err(|e| internal(e.into()))?;
    let impact = messenger_page_change_impact(
        &tx,
        &keycloak_tx,
        &tenant_id,
        &id,
        input.page_id.as_deref(),
    )
    .await
    .map_err(internal)?
    .ok_or_else(unknown_account)?;
    keycloak_tx.commit().await.map_err(|e| internal(e.into()))?;
    tx.commit().await.map_err(|e| internal(e.into()))?;
    Ok(Json(ChangeImpactOutput {
        affected_voters: impact.affected_voters,
        events: impact.election_event_ids.len(),
    }))
}

#[derive(Serialize, Debug, PartialEq, Eq)]
pub struct AccountCallbackStatus {
    account_id: String,
    /// `CALLBACK_NOT_CONFIGURED`, `REPORT_CREDENTIALS_MISSING`: receipts are
    /// unavailable while any is set; Messenger isn't ready for codes without
    /// the callback address.
    blockers: Vec<String>,
    /// One reason per blocker, in the same order.
    reasons: Vec<String>,
}

#[derive(Serialize, Debug, PartialEq, Eq)]
pub struct CallbackStatusOutput {
    accounts: Vec<AccountCallbackStatus>,
}

/// For the event tab, Logs and Settings (`messaging-config-read` or
/// `messaging-account-read`): per account of the tenant, what stops its
/// callbacks from reaching Step. The callback URL itself, which holds the
/// webhook key, stays with `messaging-account-write`.
#[instrument(skip_all)]
#[post(
    "/messaging/accounts/callback-status",
    format = "json",
    data = "<_body>"
)]
pub async fn messaging_callback_status(
    claims: JwtClaims,
    _body: Json<Value>,
    services: &State<HarvestServices>,
) -> Result<Json<CallbackStatusOutput>, JsonRefusal> {
    let tenant_id =
        authorize_admin(&claims, Permissions::MESSAGING_CONFIG_READ).or_else(
            |_| authorize_admin(&claims, Permissions::MESSAGING_ACCOUNT_READ),
        )?;
    let mut client = db_client(services).await?;
    let tx = client.transaction().await.map_err(|e| internal(e.into()))?;
    let accounts =
        windmill::postgres::messaging::list_messaging_accounts(&tx, &tenant_id)
            .await
            .map_err(internal)?;
    tx.commit().await.map_err(|e| internal(e.into()))?;
    Ok(Json(CallbackStatusOutput {
        accounts: accounts
            .iter()
            .map(|account| {
                let blockers = callback_blockers_with(
                    services.messaging_webhooks_url.as_deref(),
                    account,
                );
                AccountCallbackStatus {
                    account_id: account.id.to_string(),
                    reasons: blockers
                        .iter()
                        .filter_map(|blocker| callback_blocker_reason(*blocker))
                        .map(str::to_string)
                        .collect(),
                    blockers: blockers
                        .iter()
                        .map(|blocker| blocker.to_string())
                        .collect(),
                }
            })
            .collect(),
    }))
}

#[derive(Deserialize)]
pub struct ReplaceCredentialsInput {
    id: String,
    credentials: HashMap<String, String>,
    generate_verify_token: Option<bool>,
    /// Viber (Infobip): new Basic auth for the report subscription.
    generate_report_credentials: Option<bool>,
}

#[derive(Serialize, Debug)]
pub struct ReplaceCredentialsOutput {
    replaced: Vec<String>,
    verify_token: Option<String>,
    /// Only when generated. Shown once.
    report_username: Option<String>,
    report_password: Option<String>,
}

#[instrument(skip_all)]
#[post("/messaging/accounts/credentials", format = "json", data = "<body>")]
pub async fn replace_messaging_account_credentials(
    claims: JwtClaims,
    body: Json<ReplaceCredentialsInput>,
    services: &State<HarvestServices>,
) -> Result<Json<ReplaceCredentialsOutput>, JsonRefusal> {
    let tenant_id =
        authorize_admin(&claims, Permissions::MESSAGING_ACCOUNT_WRITE)?;
    let input = body.into_inner();
    let id = parse_uuid(&input.id)?;
    let values = parse_credentials(input.credentials)?;
    let mut client = db_client(services).await?;
    let tx = client.transaction().await.map_err(|e| internal(e.into()))?;
    // Typed credentials that replace stored ones are saved only when the
    // check with them connects; otherwise the stored ones stay (BC-F11).
    let replaced = match replace_checked_credentials(
        &tx,
        &tenant_id,
        &id,
        &values,
        input.generate_verify_token.unwrap_or(false),
        &dispatcher().await?.endpoints,
    )
    .await
    .map_err(bad_request)?
    .ok_or_else(unknown_account)?
    {
        CredentialsOutcome::Replaced(replaced, _) => replaced,
        CredentialsOutcome::Refused(reason) => {
            return Err(bad_request(reason).into())
        }
    };
    let mut names: Vec<String> =
        replaced.replaced.iter().map(|n| n.to_string()).collect();
    let report = match input.generate_report_credentials.unwrap_or(false) {
        true => {
            let generated = generate_report_credentials(&tx, &tenant_id, &id)
                .await
                .map_err(bad_request)?
                .ok_or_else(unknown_account)?;
            names.extend([
                CredentialName::REPORT_USERNAME.to_string(),
                CredentialName::REPORT_PASSWORD.to_string(),
            ]);
            Some(generated)
        }
        false => None,
    };
    tx.commit().await.map_err(|e| internal(e.into()))?;
    let (report_username, report_password) = match report {
        Some(generated) => (Some(generated.username), Some(generated.password)),
        None => (None, None),
    };
    Ok(Json(ReplaceCredentialsOutput {
        replaced: names,
        verify_token: replaced.verify_token,
        report_username,
        report_password,
    }))
}

#[derive(Serialize, Debug)]
pub struct CheckOutput {
    status: AccountCheck,
    /// As in [`MessagingAccountOutput`].
    #[serde(skip_serializing_if = "Option::is_none")]
    callback_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    callback_reason: Option<String>,
}

#[instrument(skip_all)]
#[post("/messaging/accounts/check", format = "json", data = "<body>")]
pub async fn check_messaging_account(
    claims: JwtClaims,
    body: Json<AccountIdInput>,
    services: &State<HarvestServices>,
) -> Result<Json<CheckOutput>, JsonRefusal> {
    let tenant_id =
        authorize_admin(&claims, Permissions::MESSAGING_ACCOUNT_WRITE)?;
    let id = parse_uuid(&body.id)?;
    let mut client = db_client(services).await?;
    let tx = client.transaction().await.map_err(|e| internal(e.into()))?;
    let checked =
        check_account(&tx, &tenant_id, &id, &dispatcher().await?.endpoints)
            .await
            .map_err(internal)?
            .ok_or_else(unknown_account)?;
    // Saved meanwhile with another identity or credentials: the result
    // describes what the account no longer is, and wasn't stored.
    if !checked.stored {
        return Err((Status::Conflict, CHECK_DROPPED_REASON.to_string()).into());
    }
    let status = checked.check;
    let readiness = get_messaging_account(&tx, &tenant_id, &id)
        .await
        .map_err(internal)?
        .map(|account| {
            callback_readiness_with(
                services.messaging_webhooks_url.as_deref(),
                account.provider,
                &account.webhook_key,
            )
        });
    tx.commit().await.map_err(|e| internal(e.into()))?;
    Ok(Json(CheckOutput {
        status,
        callback_url: readiness
            .as_ref()
            .and_then(|readiness| readiness.url())
            .map(str::to_string),
        callback_reason: readiness
            .as_ref()
            .and_then(|readiness| readiness.reason())
            .map(str::to_string),
    }))
}

#[derive(Deserialize, Debug)]
pub struct TestAccountInput {
    id: String,
    purpose: MessagePurpose,
    destination: String,
    language: Option<String>,
    template: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct TestAccountOutput {
    message_id: Option<String>,
    state: MessageAttemptState,
    reason: Option<String>,
}

/// Sends a test message through one account, for its selected purpose and
/// destination. A test code is a fixed placeholder, never a real one.
#[instrument(skip_all)]
#[post("/messaging/accounts/test", format = "json", data = "<body>")]
pub async fn test_messaging_account(
    claims: JwtClaims,
    body: Json<TestAccountInput>,
    services: &State<HarvestServices>,
) -> Result<Json<TestAccountOutput>, JsonRefusal> {
    let tenant_id =
        authorize_admin(&claims, Permissions::MESSAGING_ACCOUNT_WRITE)?;
    let input = body.into_inner();
    let id = parse_uuid(&input.id)?;
    let mut client = db_client(services).await?;
    let tx = client.transaction().await.map_err(|e| internal(e.into()))?;
    let account = get_messaging_account(&tx, &tenant_id, &id)
        .await
        .map_err(internal)?
        .ok_or_else(unknown_account)?;
    tx.commit().await.map_err(|e| internal(e.into()))?;
    let template =
        input
            .template
            .filter(|name| !name.trim().is_empty())
            .map(|name| ProviderTemplate {
                name,
                language: input.language.clone(),
            });
    let code =
        (input.purpose == MessagePurpose::OTP).then(|| "000000".to_string());
    let text = match input.purpose {
        MessagePurpose::OTP => {
            "000000 is a test verification code from Step.".to_string()
        }
        MessagePurpose::NOTICE => {
            "This is a test message from Step.".to_string()
        }
    };
    let result = deliver_within(
        &mut client,
        dispatcher().await?,
        &Delivery {
            tenant_id,
            election_event_id: None,
            purpose: input.purpose,
            recipient: Recipient {
                voter_id: None,
                channel: account.channel,
                destination: input.destination,
                election_ids: vec![],
                // An administrator's test names a Page-scoped ID of the
                // account's own Page.
                messenger_page: None,
            },
            content: Some(MessageContent {
                subject: Some("Step test message".to_string()),
                text,
                html: None,
                template_parameters: vec![],
                code,
            }),
            language: input.language,
            template_alias: Some("account-test".to_string()),
            logical_key: format!("account-test:{id}:{}", Uuid::new_v4()),
            expires_at: Some(Utc::now() + chrono::Duration::minutes(5)),
            account_id: Some(id),
            template_key: None,
            provider_template: template,
        },
        // The test is how an account is tried before it is ready.
        SendPolicy {
            readiness: ReadinessGate::NOT_CHECKED_FOR_TEST_MESSAGE,
            ..SendPolicy::default()
        },
    )
    .await
    .map_err(internal)?;
    Ok(Json(TestAccountOutput {
        message_id: result.message_id.map(|id| id.to_string()),
        state: result.state,
        reason: result.reason,
    }))
}

#[derive(Deserialize, Debug)]
pub struct EventConfigInput {
    election_event_id: String,
    config: EventMessagingConfig,
}

#[derive(Serialize, Debug)]
pub struct EventConfigOutput {
    errors: Value,
}

/// Validates and saves the event's messaging configuration, then publishes
/// the channels voters may see to the event's realm.
#[instrument(skip_all)]
#[post("/messaging/event-config", format = "json", data = "<body>")]
pub async fn update_event_messaging_config(
    claims: JwtClaims,
    body: Json<EventConfigInput>,
    services: &State<HarvestServices>,
) -> Result<Json<EventConfigOutput>, RouteError> {
    let tenant_id =
        authorize_admin(&claims, Permissions::MESSAGING_CONFIG_WRITE)?;
    let input = body.into_inner();
    let event_id = parse_uuid(&input.election_event_id)?;
    let mut client = db_client(services).await?;
    let tx = client.transaction().await.map_err(|e| internal(e.into()))?;
    let projection = match save_event_messaging_config_as(
        &tx,
        &tenant_id,
        &event_id,
        &input.config,
        Some(&claims.hasura_claims.user_id),
    )
    .await
    .map_err(internal)?
    {
        SaveOutcome::Saved(projection) => projection,
        SaveOutcome::Invalid(errors) => {
            return Ok(Json(EventConfigOutput {
                errors: serde_json::to_value(errors)
                    .map_err(|e| internal(e.into()))?,
            }))
        }
        SaveOutcome::UnknownEvent => {
            return Err((
                Status::NotFound,
                "unknown election event".to_string(),
            ))
        }
    };
    update_realm_attributes(
        &tenant_id.to_string(),
        &event_id.to_string(),
        HashMap::from([(
            REALM_ATTR_MESSAGING.to_string(),
            serde_json::to_string(&projection)
                .map_err(|e| internal(e.into()))?,
        )]),
    )
    .await
    .map_err(internal)?;
    tx.commit().await.map_err(|e| internal(e.into()))?;
    // Who is offered what changed: the voter counts are recounted.
    crate::routes::messaging_insights::recount_after_save(
        services, &tenant_id, &event_id,
    )
    .await;
    Ok(Json(EventConfigOutput {
        errors: Value::Array(vec![]),
    }))
}

#[cfg(test)]
#[path = "../../tests/support/messaging_routes.rs"]
mod route_tests;
