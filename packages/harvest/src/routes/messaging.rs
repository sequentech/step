// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Messaging routes: the internal sending API for Keycloak, the provider
//! callbacks relayed by the webhook component, and the administrators'
//! account and event actions. No route here is public: providers reach
//! harvest only through the `messaging-webhooks` component.

use crate::services::authorization::authorize;
use crate::services::dependencies::HarvestServices;
use crate::services::service_auth::VerifiedServiceClaims;
use chrono::{DateTime, Utc};
use deadpool_postgres::Client as DbClient;
use messaging::relay::{
    self, CallbackEnvelope, CallbackMethod, CallbackProvider, RelayAuthError,
    RelayResponse, RELAY_SECRET_ENV, RELAY_SIGNATURE_HEADER,
};
use rocket::data::{Data, ToByteUnit};
use rocket::http::{RawStr, Status};
use rocket::request::{FromRequest, Outcome, Request};
use rocket::serde::json::Json;
use rocket::State;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::services::keycloak::update_realm_attributes;
use sequent_core::types::messaging::{
    AccountCheck, AccountLimits, AccountSender, CreateMessengerLinkRequest,
    CreateMessengerLinkResponse, CredentialName, EventMessagingConfig,
    MessageAttemptState, MessageChannel, MessageContent, MessagePurpose,
    MessengerLinkRequest, MessengerLinkStatus, ProviderApproval,
    ReadinessPolicy, SendMessageRequest, SendMessageResponse,
    REALM_ATTR_MESSAGING,
};
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::str::FromStr;
use tracing::{error, info, instrument, warn};
use uuid::Uuid;
use windmill::postgres::messaging::{
    get_messaging_account, insert_messaging_account,
    update_messaging_account_reentering, AccountSettings, MessagingAccount,
};
use windmill::services::messaging::accounts::{
    callback_readiness, check_account, delete_account,
    generate_report_credentials, replace_credentials,
};
use windmill::services::messaging::audit::post_attempt_outcomes_or_warn;
use windmill::services::messaging::config::{
    save_event_messaging_config, SaveOutcome,
};
use windmill::services::messaging::dispatch::{
    deliver, Delivery, Dispatcher, ProviderTemplate, Recipient,
};
use windmill::services::messaging::links::{
    confirm_link, create_link, link_status, LinkError,
};
use windmill::services::messaging::webhooks::{
    aws_events, meta_events, meta_subscription, viber_events, WebhookOutcome,
};

type RouteError = (Status, String);

fn internal(error: anyhow::Error) -> RouteError {
    error!("messaging route failed: {error:#}");
    (Status::InternalServerError, "internal error".to_string())
}

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
/// notices. Codes are sent at once, never queued behind bulk sends.
#[instrument(skip_all)]
#[post("/messages/send", format = "json", data = "<body>")]
pub async fn send_message(
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
    let mut client = db_client(services).await?;
    let election_event_id = claims
        .require_event(&client, request.election_event_id.as_deref())
        .await?;
    let result = deliver(
        &mut client,
        dispatcher().await?,
        &Delivery {
            tenant_id,
            election_event_id,
            purpose: request.purpose,
            recipient: Recipient {
                voter_id: request.voter_id.clone(),
                channel: request.channel,
                destination: request.destination.clone(),
                // The voter's Posts: windmill refuses a channel they don't
                // offer ("Channel not offered for notices"/"…for codes").
                election_ids: request.election_ids.clone(),
            },
            content: Some(request.content.clone()),
            language: request.language.clone(),
            template_alias: None,
            logical_key: format!("keycloak:{}", request.logical_key),
            expires_at,
            account_id: None,
            template_key: request.template_key.clone(),
            provider_template: None,
        },
    )
    .await
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
        state: result.state,
        reason: result.reason,
    }))
}

fn link_error(error: LinkError) -> RouteError {
    match error {
        LinkError::MessengerNotEnabled => (
            Status::UnprocessableEntity,
            "Messenger is not enabled for codes".to_string(),
        ),
        LinkError::NotFound => (Status::NotFound, "unknown link".to_string()),
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
    create_link(&mut client, &request)
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
}

/// Whether harvest can check relay signatures.
#[derive(Clone)]
enum RelaySecret {
    /// `MESSAGING_RELAY_SECRET` is unset: every relayed callback is refused
    /// with 503, so the component makes the provider retry.
    NotConfigured,
    /// The secret shared with the component.
    Configured(Vec<u8>),
}

/// Why a relayed callback was refused because harvest isn't set up for it.
const RELAY_NOT_CONFIGURED: &str =
    "Callbacks refused: MESSAGING_RELAY_SECRET is not set";

impl MessagingRelay {
    /// `secret` (none or blank: not configured) and the component's body
    /// limit.
    pub fn new(secret: Option<&[u8]>, max_body_bytes: u64) -> Self {
        Self {
            secret: match secret.filter(|secret| !secret.is_empty()) {
                Some(secret) => RelaySecret::Configured(secret.to_vec()),
                None => RelaySecret::NotConfigured,
            },
            envelope_limit: relay::envelope_limit(max_body_bytes),
        }
    }

    /// From `MESSAGING_RELAY_SECRET` and
    /// `MESSAGING_WEBHOOKS_MAX_BODY_BYTES`.
    pub fn from_env() -> Self {
        let secret = std::env::var(RELAY_SECRET_ENV)
            .ok()
            .filter(|secret| !secret.trim().is_empty());
        if secret.is_none() {
            warn!("{RELAY_NOT_CONFIGURED}");
        }
        let max_body_bytes = relay::max_body_bytes(
            std::env::var(relay::MAX_BODY_BYTES_ENV).ok().as_deref(),
        )
        .unwrap_or_else(|error| {
            error!("{error:#}; using {} bytes", relay::DEFAULT_MAX_BODY_BYTES);
            relay::DEFAULT_MAX_BODY_BYTES
        });
        Self::new(secret.as_deref().map(str::as_bytes), max_body_bytes)
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
async fn provider_callback(
    envelope: &CallbackEnvelope,
    services: &State<HarvestServices>,
) -> Result<RelayResponse, Status> {
    let key = envelope.key.as_str();
    if !relay::is_webhook_key(key) {
        return Ok(provider_answer(Status::NotFound));
    }
    let body = envelope.body().map_err(|_| Status::BadRequest)?;
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
            let dispatcher =
                dispatcher().await.map_err(|_| Status::ServiceUnavailable)?;
            meta_events(
                &mut client,
                dispatcher,
                key,
                envelope.header("x-hub-signature-256"),
                &body,
            )
            .await
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
    let RelaySecret::Configured(secret) = &relay_config.secret else {
        error!("{RELAY_NOT_CONFIGURED}");
        return Err(Status::ServiceUnavailable);
    };
    let bytes = read_body(data, relay_config.envelope_limit).await?;
    if let Err(reason) = relay::verify(
        secret,
        Some(&signature.0),
        &bytes,
        Utc::now().timestamp(),
    ) {
        warn!(%reason, "relayed callback refused");
        return Err(Status::Unauthorized);
    }
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
    fn of(account: &MessagingAccount) -> Self {
        let readiness = callback_readiness(account);
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
) -> Result<Json<MessagingAccountOutput>, RouteError> {
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
        readiness: input.readiness.unwrap_or_default(),
        is_default: input.is_default.unwrap_or(false),
    };
    let mut client = db_client(services).await?;
    let tx = client.transaction().await.map_err(|e| internal(e.into()))?;
    let reentered: BTreeSet<CredentialName> = values.keys().copied().collect();
    let account = match input.id.as_deref().map(parse_uuid).transpose()? {
        Some(id) => update_messaging_account_reentering(
            &tx, &tenant_id, &id, &settings, &reentered,
        )
        .await
        .map_err(bad_request)?
        .ok_or((Status::NotFound, "unknown account".to_string()))?,
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
    tx.commit().await.map_err(|e| internal(e.into()))?;
    Ok(Json(MessagingAccountOutput::of(&account)))
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
) -> Result<Json<MessagingAccountOutput>, RouteError> {
    let tenant_id =
        authorize_admin(&claims, Permissions::MESSAGING_ACCOUNT_WRITE)?;
    let id = parse_uuid(&body.id)?;
    let mut client = db_client(services).await?;
    let tx = client.transaction().await.map_err(|e| internal(e.into()))?;
    if !delete_account(&tx, &tenant_id, &id)
        .await
        .map_err(internal)?
    {
        return Err((Status::NotFound, "unknown account".to_string()));
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
) -> Result<Json<CallbackUrlOutput>, RouteError> {
    let tenant_id =
        authorize_admin(&claims, Permissions::MESSAGING_ACCOUNT_WRITE)?;
    let id = parse_uuid(&body.id)?;
    let mut client = db_client(services).await?;
    let tx = client.transaction().await.map_err(|e| internal(e.into()))?;
    let account = get_messaging_account(&tx, &tenant_id, &id)
        .await
        .map_err(internal)?
        .ok_or((Status::NotFound, "unknown account".to_string()))?;
    tx.commit().await.map_err(|e| internal(e.into()))?;
    let readiness = callback_readiness(&account);
    Ok(Json(CallbackUrlOutput {
        callback_url: readiness.url().map(str::to_string),
        reason: readiness.reason().map(str::to_string),
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
) -> Result<Json<ReplaceCredentialsOutput>, RouteError> {
    let tenant_id =
        authorize_admin(&claims, Permissions::MESSAGING_ACCOUNT_WRITE)?;
    let input = body.into_inner();
    let id = parse_uuid(&input.id)?;
    let values = parse_credentials(input.credentials)?;
    let mut client = db_client(services).await?;
    let tx = client.transaction().await.map_err(|e| internal(e.into()))?;
    let replaced = replace_credentials(
        &tx,
        &tenant_id,
        &id,
        &values,
        input.generate_verify_token.unwrap_or(false),
    )
    .await
    .map_err(bad_request)?
    .ok_or((Status::NotFound, "unknown account".to_string()))?;
    let mut names: Vec<String> =
        replaced.replaced.iter().map(|n| n.to_string()).collect();
    let report = match input.generate_report_credentials.unwrap_or(false) {
        true => {
            let generated = generate_report_credentials(&tx, &tenant_id, &id)
                .await
                .map_err(bad_request)?
                .ok_or((Status::NotFound, "unknown account".to_string()))?;
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
) -> Result<Json<CheckOutput>, RouteError> {
    let tenant_id =
        authorize_admin(&claims, Permissions::MESSAGING_ACCOUNT_WRITE)?;
    let id = parse_uuid(&body.id)?;
    let mut client = db_client(services).await?;
    let tx = client.transaction().await.map_err(|e| internal(e.into()))?;
    let status =
        check_account(&tx, &tenant_id, &id, &dispatcher().await?.endpoints)
            .await
            .map_err(internal)?
            .ok_or((Status::NotFound, "unknown account".to_string()))?;
    let readiness = get_messaging_account(&tx, &tenant_id, &id)
        .await
        .map_err(internal)?
        .map(|account| callback_readiness(&account));
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
) -> Result<Json<TestAccountOutput>, RouteError> {
    let tenant_id =
        authorize_admin(&claims, Permissions::MESSAGING_ACCOUNT_WRITE)?;
    let input = body.into_inner();
    let id = parse_uuid(&input.id)?;
    let mut client = db_client(services).await?;
    let tx = client.transaction().await.map_err(|e| internal(e.into()))?;
    let account = get_messaging_account(&tx, &tenant_id, &id)
        .await
        .map_err(internal)?
        .ok_or((Status::NotFound, "unknown account".to_string()))?;
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
    let result = deliver(
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
    let projection = match save_event_messaging_config(
        &tx,
        &tenant_id,
        &event_id,
        &input.config,
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
    Ok(Json(EventConfigOutput {
        errors: Value::Array(vec![]),
    }))
}

#[cfg(test)]
#[path = "../../tests/support/messaging_routes.rs"]
mod route_tests;
