// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Provider callbacks. The account comes from the URL's webhook key, never
//! from the payload, and each provider's own verification runs before
//! anything is read. Callbacks never authenticate a voter.

use super::accounts::runtime_account;
use super::config::get_event_messaging_config;
use super::dispatch::{deliver, Delivery, Dispatcher, FallbackPolicy, Recipient};
use super::keys::{tenant_key, TenantKey};
use super::links::bind_referral;
use crate::postgres::messaging::{
    deliver_messages_up_to, find_message_by_provider_id, find_message_by_reference,
    get_messaging_account_by_webhook_key, inbound_exists, insert_message, last_auto_reply_at,
    transition_message, MessagingAccount, NewMessage, StateChange, AUTO_REPLY_ALIAS,
};
use anyhow::Result;
use chrono::{Duration, Utc};
use deadpool_postgres::Client as DbClient;
use messaging::destination::Destination;
use messaging::webhooks::sns::{is_sns_url, is_trusted_certificate_url, ses_events, SnsEnvelope};
use messaging::webhooks::{
    http, infobip, meta, DeliveredUpTo, InboundMessage, StatusReport, WebhookEvent,
};
use sequent_core::types::messaging::{
    AccountSender, CredentialName, MessageAttemptState, MessageChannel, MessageContent,
    MessageDirection, MessagePurpose, MessagingProvider,
};
use std::collections::BTreeMap;
use tracing::{info, instrument, warn};
use uuid::Uuid;

#[derive(Debug, PartialEq, Eq)]
pub enum WebhookOutcome {
    Accepted,
    /// Signature, token or topic did not verify.
    Unauthorized,
    /// No account has this webhook key, or it is not for this provider.
    NotFound,
}

async fn account_for_key(
    client: &mut DbClient,
    webhook_key: &str,
    providers: &[MessagingProvider],
) -> Result<Option<MessagingAccount>> {
    let tx = client.transaction().await?;
    let account = get_messaging_account_by_webhook_key(&tx, webhook_key)
        .await?
        .filter(|account| providers.contains(&account.provider));
    tx.commit().await?;
    Ok(account)
}

async fn credential(
    client: &mut DbClient,
    account: &MessagingAccount,
    name: CredentialName,
) -> Result<Option<String>> {
    let tx = client.transaction().await?;
    let runtime = runtime_account(&tx, account).await?;
    tx.commit().await?;
    Ok(runtime.credential(name).ok().map(str::to_string))
}

const META_PROVIDERS: &[MessagingProvider] = &[
    MessagingProvider::WHATSAPP_CLOUD_API,
    MessagingProvider::MESSENGER_SEND_API,
];

/// Meta's subscription handshake. Returns the challenge to echo.
#[instrument(skip_all, err)]
pub async fn meta_subscription(
    client: &mut DbClient,
    webhook_key: &str,
    mode: Option<&str>,
    token: Option<&str>,
    challenge: Option<&str>,
) -> Result<Option<String>> {
    let Some(account) = account_for_key(client, webhook_key, META_PROVIDERS).await? else {
        return Ok(None);
    };
    let Some(expected) = credential(client, &account, CredentialName::VERIFY_TOKEN).await? else {
        return Ok(None);
    };
    Ok(meta::verify_subscription(mode, token, challenge, &expected))
}

#[instrument(skip_all, err)]
pub async fn meta_events(
    client: &mut DbClient,
    dispatcher: &Dispatcher,
    webhook_key: &str,
    signature: Option<&str>,
    body: &[u8],
) -> Result<WebhookOutcome> {
    let Some(account) = account_for_key(client, webhook_key, META_PROVIDERS).await? else {
        return Ok(WebhookOutcome::NotFound);
    };
    let Some(secret) = credential(client, &account, CredentialName::APP_SECRET).await? else {
        return Ok(WebhookOutcome::Unauthorized);
    };
    if !meta::verify_signature(&secret, signature, body) {
        return Ok(WebhookOutcome::Unauthorized);
    }
    let events = meta::parse_events(&account.sender, body)?;
    apply_events(client, dispatcher, &account, events).await?;
    Ok(WebhookOutcome::Accepted)
}

#[instrument(skip_all, err)]
pub async fn viber_events(
    client: &mut DbClient,
    dispatcher: &Dispatcher,
    webhook_key: &str,
    body: &[u8],
) -> Result<WebhookOutcome> {
    let Some(account) =
        account_for_key(client, webhook_key, &[MessagingProvider::VIBER_INFOBIP]).await?
    else {
        return Ok(WebhookOutcome::NotFound);
    };
    let events = infobip::parse_events(body)?;
    apply_events(client, dispatcher, &account, events).await?;
    Ok(WebhookOutcome::Accepted)
}

/// SES events delivered by SNS. Only the configured topic is accepted, and
/// the SNS signature must verify with a certificate served by SNS.
#[instrument(skip_all, err)]
pub async fn aws_events(
    client: &mut DbClient,
    dispatcher: &Dispatcher,
    webhook_key: &str,
    body: &[u8],
) -> Result<WebhookOutcome> {
    let Some(account) = account_for_key(client, webhook_key, &[MessagingProvider::AWS_SES]).await?
    else {
        return Ok(WebhookOutcome::NotFound);
    };
    let AccountSender::AWS_SES {
        notification_topic_arn: Some(topic),
        ..
    } = &account.sender
    else {
        return Ok(WebhookOutcome::Unauthorized);
    };
    let Ok(envelope) = SnsEnvelope::parse(body) else {
        return Ok(WebhookOutcome::Unauthorized);
    };
    if envelope.topic_arn != *topic || !is_trusted_certificate_url(&envelope.signing_cert_url) {
        return Ok(WebhookOutcome::Unauthorized);
    }
    let certificate = reqwest::get(&envelope.signing_cert_url)
        .await?
        .error_for_status()?
        .bytes()
        .await?;
    if envelope.verify(&certificate).is_err() {
        return Ok(WebhookOutcome::Unauthorized);
    }
    match envelope.kind.as_str() {
        "SubscriptionConfirmation" => {
            if let Some(url) = envelope
                .subscribe_url
                .as_deref()
                .filter(|url| is_sns_url(url))
            {
                reqwest::get(url).await?.error_for_status()?;
                info!(account = %account.id, "confirmed the SES notification subscription");
            }
        }
        "Notification" => {
            let events = ses_events(&envelope)?;
            apply_events(client, dispatcher, &account, events).await?;
        }
        _ => {}
    }
    Ok(WebhookOutcome::Accepted)
}

async fn apply_status(
    client: &mut DbClient,
    account: &MessagingAccount,
    report: &StatusReport,
) -> Result<()> {
    let tx = client.transaction().await?;
    // By the provider's ID, or by Step's own reference when the provider
    // echoes that back instead.
    let message = match find_message_by_provider_id(&tx, &account.id, &report.provider_message_id)
        .await?
    {
        Some(message) => Some(message),
        None => find_message_by_reference(&tx, &account.id, &report.provider_message_id).await?,
    };
    let Some(message) = message else {
        tx.commit().await?;
        return Ok(());
    };
    let change = StateChange {
        error: report
            .error_code
            .as_ref()
            .map(|code| format!("provider code {code}")),
        failure: (report.state == MessageAttemptState::FAILED)
            .then_some(messaging::sender::FailureKind::PERMANENT),
        billing: report.billing.clone(),
        ..Default::default()
    };
    if transition_message(&tx, &message.id, report.state, &change)
        .await?
        .is_none()
    {
        info!(message_id = %message.id, from = %message.state, to = %report.state, "ignored a provider report");
    }
    tx.commit().await?;
    Ok(())
}

/// Records an incoming message (metadata only) and, at most once a day per
/// recipient, answers with the event's configured reply.
async fn apply_inbound(
    client: &mut DbClient,
    dispatcher: &Dispatcher,
    account: &MessagingAccount,
    inbound: &InboundMessage,
) -> Result<()> {
    let Ok(destination) = Destination::parse(account.channel, &inbound.from) else {
        return Ok(());
    };
    let tx = client.transaction().await?;
    if !inbound.provider_message_id.is_empty()
        && inbound_exists(&tx, &account.id, &inbound.provider_message_id).await?
    {
        tx.commit().await?;
        return Ok(());
    }
    let key = tenant_key(&tx, &account.tenant_id, TenantKey::Destination).await?;
    let digest = destination.digest(&key)?;
    // The event of the latest message to this recipient, for its reply text.
    let event: Option<(Option<Uuid>, Option<String>)> = tx
        .query_opt(
            r#"
            SELECT election_event_id, voter_id FROM sequent_backend.message
            WHERE account_id = $1 AND destination_digest = $2 AND direction = 'OUTBOUND'
            ORDER BY created_at DESC LIMIT 1
            "#,
            &[&account.id, &digest],
        )
        .await?
        .map(|row| (row.get(0), row.get(1)));
    let (election_event_id, voter_id) = event.unwrap_or((None, None));
    insert_message(
        &tx,
        &NewMessage {
            tenant_id: account.tenant_id,
            election_event_id,
            voter_id: voter_id.clone(),
            account_id: Some(account.id),
            channel: account.channel,
            direction: MessageDirection::INBOUND,
            purpose: None,
            template_alias: None,
            language: None,
            masked_destination: destination.masked(),
            destination_digest: digest.clone(),
            destination_country: destination.calling_code().map(str::to_string),
            logical_key: None,
            attempt: 1,
            state: MessageAttemptState::DELIVERED,
            provider_message_id: (!inbound.provider_message_id.is_empty())
                .then(|| inbound.provider_message_id.clone()),
            error: None,
            failure: None,
        },
    )
    .await?;
    let reply = match &election_event_id {
        Some(event) => get_event_messaging_config(&tx, &account.tenant_id, event)
            .await?
            .and_then(|config| {
                config
                    .reply_text
                    .get("en")
                    .or_else(|| config.reply_text.values().next())
                    .cloned()
            }),
        None => None,
    };
    let recently_replied = last_auto_reply_at(&tx, &account.id, &digest)
        .await?
        .is_some_and(|at| Utc::now() - at < Duration::hours(24));
    tx.commit().await?;
    let (Some(text), false) = (reply, recently_replied) else {
        return Ok(());
    };
    let today = Utc::now().format("%Y-%m-%d");
    deliver(
        client,
        dispatcher,
        &Delivery {
            tenant_id: account.tenant_id,
            election_event_id,
            purpose: MessagePurpose::NOTICE,
            first_channel: account.channel,
            fallback: FallbackPolicy::NONE,
            recipient: Recipient {
                voter_id,
                destinations: BTreeMap::from([(account.channel, inbound.from.clone())]),
                verified: vec![account.channel],
                election_ids: vec![],
            },
            contents: BTreeMap::from([(
                account.channel,
                MessageContent {
                    subject: None,
                    text,
                    html: None,
                    template_parameters: vec![],
                    code: None,
                },
            )]),
            language: None,
            template_alias: Some(AUTO_REPLY_ALIAS.to_string()),
            logical_key: format!("{AUTO_REPLY_ALIAS}:{}:{digest}:{today}", account.id),
            expires_at: None,
            account_id: Some(account.id),
            template_key: Some(AUTO_REPLY_ALIAS.to_string()),
            provider_templates: BTreeMap::new(),
        },
    )
    .await?;
    Ok(())
}

pub async fn apply_events(
    client: &mut DbClient,
    dispatcher: &Dispatcher,
    account: &MessagingAccount,
    events: Vec<WebhookEvent>,
) -> Result<()> {
    for event in events {
        let result = match &event {
            WebhookEvent::Status(report) => apply_status(client, account, report).await,
            WebhookEvent::Inbound(inbound) => {
                apply_inbound(client, dispatcher, account, inbound).await
            }
            WebhookEvent::MessengerReferral(referral) => {
                if account.channel == MessageChannel::MESSENGER {
                    record_interaction(client, account, referral).await?;
                    bind_referral(client, dispatcher, account, referral)
                        .await
                        .map(|_| ())
                } else {
                    Ok(())
                }
            }
            WebhookEvent::DeliveredUpTo(delivered) => {
                apply_delivered_up_to(client, account, delivered).await
            }
        };
        if let Err(error) = result {
            warn!(account = %account.id, "could not apply a webhook event: {error:#}");
        }
    }
    Ok(())
}

async fn apply_delivered_up_to(
    client: &mut DbClient,
    account: &MessagingAccount,
    delivered: &DeliveredUpTo,
) -> Result<()> {
    let Ok(destination) = Destination::parse(account.channel, &delivered.recipient) else {
        return Ok(());
    };
    let tx = client.transaction().await?;
    let key = tenant_key(&tx, &account.tenant_id, TenantKey::Destination).await?;
    deliver_messages_up_to(
        &tx,
        &account.id,
        &destination.digest(&key)?,
        delivered.up_to,
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

/// Callbacks of a provider described by configuration, as a JSON body or
/// as query parameters. `headers` have lowercase names.
#[instrument(skip_all, err)]
pub async fn http_events(
    client: &mut DbClient,
    dispatcher: &Dispatcher,
    webhook_key: &str,
    headers: &BTreeMap<String, String>,
    query: &BTreeMap<String, String>,
    body: &[u8],
) -> Result<WebhookOutcome> {
    let Some(account) =
        account_for_key(client, webhook_key, &[MessagingProvider::HTTP_API]).await?
    else {
        return Ok(WebhookOutcome::NotFound);
    };
    let AccountSender::HTTP_API(api) = &account.sender else {
        return Ok(WebhookOutcome::NotFound);
    };
    let Some(reports) = &api.reports else {
        return Ok(WebhookOutcome::NotFound);
    };
    let secret = credential(client, &account, CredentialName::WEBHOOK_SECRET).await?;
    if !http::verify(&reports.auth, secret.as_deref(), headers, body) {
        return Ok(WebhookOutcome::Unauthorized);
    }
    let payload = if body.iter().all(u8::is_ascii_whitespace) {
        http::query_as_json(query)
    } else {
        match serde_json::from_slice(body) {
            Ok(payload) => payload,
            Err(_) => return Ok(WebhookOutcome::Unauthorized),
        }
    };
    let events = http::parse_events(reports, &payload);
    apply_events(client, dispatcher, &account, events).await?;
    Ok(WebhookOutcome::Accepted)
}

/// A referral opens the conversation window even without a message.
async fn record_interaction(
    client: &mut DbClient,
    account: &MessagingAccount,
    referral: &messaging::webhooks::MessengerReferral,
) -> Result<()> {
    let Ok(destination) = Destination::parse(MessageChannel::MESSENGER, &referral.page_scoped_id)
    else {
        return Ok(());
    };
    let tx = client.transaction().await?;
    let key = tenant_key(&tx, &account.tenant_id, TenantKey::Destination).await?;
    insert_message(
        &tx,
        &NewMessage {
            tenant_id: account.tenant_id,
            election_event_id: None,
            voter_id: None,
            account_id: Some(account.id),
            channel: MessageChannel::MESSENGER,
            direction: MessageDirection::INBOUND,
            purpose: None,
            template_alias: None,
            language: None,
            masked_destination: destination.masked(),
            destination_digest: destination.digest(&key)?,
            destination_country: None,
            logical_key: None,
            attempt: 1,
            state: MessageAttemptState::DELIVERED,
            provider_message_id: None,
            error: None,
            failure: None,
        },
    )
    .await?;
    tx.commit().await?;
    Ok(())
}
