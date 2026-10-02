// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Messaging routes: the internal sending API for Keycloak, provider
//! webhooks, and the administrators' account and event actions.

use crate::services::authorization::authorize;
use crate::services::dependencies::HarvestServices;
use chrono::{DateTime, Utc};
use deadpool_postgres::Client as DbClient;
use rocket::data::{Data, ToByteUnit};
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome, Request};
use rocket::serde::json::Json;
use rocket::State;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::services::keycloak::update_realm_attributes;
use sequent_core::types::messaging::{
    AccountCheck, AccountLimits, AccountSender, CreateMessengerLinkRequest,
    CreateMessengerLinkResponse, CredentialName, EventMessagingConfig,
    MessageAttemptState, MessageContent, MessagePurpose, MessengerLinkRequest,
    MessengerLinkStatus, ProviderApproval, SendMessageRequest,
    SendMessageResponse, REALM_ATTR_MESSAGING,
};
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::str::FromStr;
use tracing::{error, instrument, warn};
use uuid::Uuid;
use windmill::postgres::messaging::{
    get_messaging_account, insert_messaging_account, update_messaging_account,
    AccountSettings,
};
use windmill::services::messaging::accounts::{
    check_account, delete_account, replace_credentials,
};
use windmill::services::messaging::config::{
    save_event_messaging_config, SaveOutcome,
};
use windmill::services::messaging::dispatch::{
    deliver, Delivery, Dispatcher, FallbackPolicy, Recipient,
};
use windmill::services::messaging::links::{
    confirm_link, create_link, link_status, LinkError,
};
use windmill::services::messaging::webhooks::{
    aws_events, meta_events, meta_subscription, viber_events, WebhookOutcome,
};

/// Webhook bodies are small; anything larger is refused unread.
const WEBHOOK_BODY_LIMIT_KIB: u64 = 512;

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

/// Keycloak's service account, for the tenant named in the request.
fn authorize_service(
    claims: &JwtClaims,
    tenant_id: &str,
) -> Result<(), RouteError> {
    authorize(
        claims,
        true,
        Some(tenant_id.to_string()),
        vec![Permissions::SERVICE_ACCOUNT],
    )
}

/// Sends one message for Keycloak: codes, and enrollment and credential
/// notices. Codes are sent at once, never queued behind bulk sends.
#[instrument(skip_all)]
#[post("/messages/send", format = "json", data = "<body>")]
pub async fn send_message(
    claims: JwtClaims,
    body: Json<SendMessageRequest>,
    services: &State<HarvestServices>,
) -> Result<Json<SendMessageResponse>, RouteError> {
    let request = body.into_inner();
    authorize_service(&claims, &request.tenant_id)?;
    if request.logical_key.trim().is_empty() {
        return Err(bad_request("missing logical_key"));
    }
    let expires_at =
        request.expires_at.as_deref().map(parse_time).transpose()?;
    if request.purpose == MessagePurpose::OTP && expires_at.is_none() {
        return Err(bad_request("codes need expires_at"));
    }
    let mut client = db_client(services).await?;
    let result = deliver(
        &mut client,
        dispatcher().await?,
        &Delivery {
            tenant_id: parse_uuid(&request.tenant_id)?,
            election_event_id: request
                .election_event_id
                .as_deref()
                .map(parse_uuid)
                .transpose()?,
            purpose: request.purpose,
            first_channel: request.channel,
            fallback: FallbackPolicy::NONE,
            recipient: Recipient {
                voter_id: request.voter_id.clone(),
                destinations: BTreeMap::from([(
                    request.channel,
                    request.destination.clone(),
                )]),
                verified: vec![request.channel],
                election_ids: vec![],
            },
            contents: BTreeMap::from([(
                request.channel,
                request.content.clone(),
            )]),
            language: request.language.clone(),
            template_alias: None,
            logical_key: format!("keycloak:{}", request.logical_key),
            expires_at,
            account_id: None,
            provider_template: None,
        },
    )
    .await
    .map_err(internal)?;
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
    }
}

#[instrument(skip_all)]
#[post("/messages/link", format = "json", data = "<body>")]
pub async fn create_messenger_link(
    claims: JwtClaims,
    body: Json<CreateMessengerLinkRequest>,
    services: &State<HarvestServices>,
) -> Result<Json<CreateMessengerLinkResponse>, RouteError> {
    let request = body.into_inner();
    authorize_service(&claims, &request.tenant_id)?;
    let mut client = db_client(services).await?;
    create_link(&mut client, &request)
        .await
        .map_err(|error| bad_request(error))?
        .map(Json)
        .map_err(link_error)
}

#[instrument(skip_all)]
#[post("/messages/link/status", format = "json", data = "<body>")]
pub async fn messenger_link_status(
    claims: JwtClaims,
    body: Json<MessengerLinkRequest>,
    services: &State<HarvestServices>,
) -> Result<Json<MessengerLinkStatus>, RouteError> {
    let request = body.into_inner();
    authorize_service(&claims, &request.tenant_id)?;
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
    claims: JwtClaims,
    body: Json<MessengerLinkRequest>,
    services: &State<HarvestServices>,
) -> Result<Json<MessengerLinkStatus>, RouteError> {
    let request = body.into_inner();
    authorize_service(&claims, &request.tenant_id)?;
    let mut client = db_client(services).await?;
    confirm_link(&mut client, &request)
        .await
        .map_err(internal)?
        .map(Json)
        .map_err(link_error)
}

/// `X-Hub-Signature-256` of a Meta webhook.
pub struct HubSignature(Option<String>);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for HubSignature {
    type Error = ();

    async fn from_request(
        request: &'r Request<'_>,
    ) -> Outcome<Self, Self::Error> {
        Outcome::Success(HubSignature(
            request
                .headers()
                .get_one("X-Hub-Signature-256")
                .map(str::to_string),
        ))
    }
}

/// Meta's `hub.mode`, `hub.verify_token` and `hub.challenge`: Rocket reads
/// the dotted names as fields of `hub`.
#[derive(FromForm)]
pub struct MetaSubscription {
    mode: Option<String>,
    verify_token: Option<String>,
    challenge: Option<String>,
}

fn webhook_status(outcome: WebhookOutcome) -> Status {
    match outcome {
        WebhookOutcome::Accepted => Status::Ok,
        WebhookOutcome::Unauthorized => Status::Unauthorized,
        WebhookOutcome::NotFound => Status::NotFound,
    }
}

async fn read_body(data: Data<'_>) -> Result<Vec<u8>, Status> {
    let body = data
        .open(WEBHOOK_BODY_LIMIT_KIB.kibibytes())
        .into_bytes()
        .await
        .map_err(|_| Status::BadRequest)?;
    if !body.is_complete() {
        return Err(Status::PayloadTooLarge);
    }
    Ok(body.into_inner())
}

/// Meta's subscription handshake for WhatsApp and Messenger accounts.
#[instrument(skip_all)]
#[get("/webhooks/meta/<key>?<hub>")]
pub async fn meta_webhook_subscription(
    key: &str,
    hub: MetaSubscription,
    services: &State<HarvestServices>,
) -> Result<String, Status> {
    let mut client = db_client(services).await.map_err(|(status, _)| status)?;
    meta_subscription(
        &mut client,
        key,
        hub.mode.as_deref(),
        hub.verify_token.as_deref(),
        hub.challenge.as_deref(),
    )
    .await
    .map_err(|error| internal(error).0)?
    .ok_or(Status::Forbidden)
}

#[instrument(skip_all)]
#[post("/webhooks/meta/<key>", data = "<data>")]
pub async fn meta_webhook(
    key: &str,
    signature: HubSignature,
    data: Data<'_>,
    services: &State<HarvestServices>,
) -> Status {
    let body = match read_body(data).await {
        Ok(body) => body,
        Err(status) => return status,
    };
    let Ok(mut client) = db_client(services).await else {
        return Status::ServiceUnavailable;
    };
    let Ok(dispatcher) = dispatcher().await else {
        return Status::ServiceUnavailable;
    };
    match meta_events(
        &mut client,
        dispatcher,
        key,
        signature.0.as_deref(),
        &body,
    )
    .await
    {
        Ok(outcome) => webhook_status(outcome),
        Err(error) => {
            warn!("Meta webhook failed: {error:#}");
            Status::InternalServerError
        }
    }
}

#[instrument(skip_all)]
#[post("/webhooks/viber/<key>", data = "<data>")]
pub async fn viber_webhook(
    key: &str,
    data: Data<'_>,
    services: &State<HarvestServices>,
) -> Status {
    let body = match read_body(data).await {
        Ok(body) => body,
        Err(status) => return status,
    };
    let (Ok(mut client), Ok(dispatcher)) =
        (db_client(services).await, dispatcher().await)
    else {
        return Status::ServiceUnavailable;
    };
    match viber_events(&mut client, dispatcher, key, &body).await {
        Ok(outcome) => webhook_status(outcome),
        Err(error) => {
            warn!("Viber webhook failed: {error:#}");
            Status::InternalServerError
        }
    }
}

#[instrument(skip_all)]
#[post("/webhooks/aws/<key>", data = "<data>")]
pub async fn aws_webhook(
    key: &str,
    data: Data<'_>,
    services: &State<HarvestServices>,
) -> Status {
    let body = match read_body(data).await {
        Ok(body) => body,
        Err(status) => return status,
    };
    let (Ok(mut client), Ok(dispatcher)) =
        (db_client(services).await, dispatcher().await)
    else {
        return Status::ServiceUnavailable;
    };
    match aws_events(&mut client, dispatcher, key, &body).await {
        Ok(outcome) => webhook_status(outcome),
        Err(error) => {
            warn!("AWS webhook failed: {error:#}");
            Status::InternalServerError
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct UpsertMessagingAccountInput {
    id: Option<String>,
    name: String,
    sender: AccountSender,
    limits: Option<AccountLimits>,
    provider_approval: Option<ProviderApproval>,
    is_default: Option<bool>,
}

#[derive(Serialize, Debug)]
pub struct MessagingAccountOutput {
    id: String,
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
    let channel = input.sender.provider().channel().ok_or_else(|| {
        bad_request("development accounts cannot be created here")
    })?;
    let settings = AccountSettings {
        channel,
        name: input.name,
        sender: input.sender,
        limits: input.limits.unwrap_or_default(),
        provider_approval: input.provider_approval.unwrap_or_default(),
        is_default: input.is_default.unwrap_or(false),
    };
    let mut client = db_client(services).await?;
    let tx = client.transaction().await.map_err(|e| internal(e.into()))?;
    let account = match input.id.as_deref().map(parse_uuid).transpose()? {
        Some(id) => update_messaging_account(&tx, &tenant_id, &id, &settings)
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
    tx.commit().await.map_err(|e| internal(e.into()))?;
    Ok(Json(MessagingAccountOutput {
        id: account.id.to_string(),
    }))
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
    Ok(Json(MessagingAccountOutput { id: id.to_string() }))
}

#[derive(Deserialize)]
pub struct ReplaceCredentialsInput {
    id: String,
    credentials: HashMap<String, String>,
    generate_verify_token: Option<bool>,
}

#[derive(Serialize, Debug)]
pub struct ReplaceCredentialsOutput {
    replaced: Vec<String>,
    verify_token: Option<String>,
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
    let values = input
        .credentials
        .into_iter()
        .map(|(name, value)| {
            CredentialName::from_str(&name)
                .map(|name| (name, value))
                .map_err(|_| bad_request(format!("unknown credential {name}")))
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;
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
    tx.commit().await.map_err(|e| internal(e.into()))?;
    Ok(Json(ReplaceCredentialsOutput {
        replaced: replaced.replaced.iter().map(|n| n.to_string()).collect(),
        verify_token: replaced.verify_token,
    }))
}

#[derive(Serialize, Debug)]
pub struct CheckOutput {
    status: AccountCheck,
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
    tx.commit().await.map_err(|e| internal(e.into()))?;
    Ok(Json(CheckOutput { status }))
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
            first_channel: account.channel,
            fallback: FallbackPolicy::NONE,
            recipient: Recipient {
                voter_id: None,
                destinations: BTreeMap::from([(
                    account.channel,
                    input.destination,
                )]),
                verified: vec![account.channel],
                election_ids: vec![],
            },
            contents: BTreeMap::from([(
                account.channel,
                MessageContent {
                    subject: Some("Step test message".to_string()),
                    text,
                    html: None,
                    template_parameters: vec![],
                    code,
                },
            )]),
            language: input.language,
            template_alias: Some("account-test".to_string()),
            logical_key: format!("account-test:{id}:{}", Uuid::new_v4()),
            expires_at: Some(Utc::now() + chrono::Duration::minutes(5)),
            account_id: Some(id),
            provider_template: input.template,
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
