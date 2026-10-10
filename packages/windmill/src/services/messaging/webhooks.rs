// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Provider callbacks. The account comes from the URL's webhook key, never
//! from the payload, and each provider's own verification runs before
//! anything is read. Callbacks never authenticate a voter.
//!
//! Only statuses, Messenger's Get Started and referrals are used. Messages
//! voters write are acknowledged and dropped: nothing they send is stored
//! or answered.

use super::accounts::{flag_number_change, runtime_account};
use super::dispatch::{retry_refusal, Dispatcher, SendPolicy};
use super::keys::{tenant_key, TenantKey};
use super::links::{bind_referral, send_bound_code, BoundCode};
use crate::postgres::messaging::{
    deliver_messages_up_to, find_message_by_provider_id, find_message_by_reference,
    get_messaging_account_by_webhook_key, record_messenger_contact, transition_message,
    MessagingAccount, StateChange,
};
use anyhow::Result;
use chrono::{DateTime, Utc};
use deadpool_postgres::Client as DbClient;
use messaging::attempts::{next_retry_at, reported_too_late};
use messaging::destination::Destination;
use messaging::providers::infobip::{verify_report_authorization, ReportCredentials};
use messaging::webhooks::sns::{
    is_sns_url, is_trusted_certificate_url, ses_events, CertificateCache, SnsEnvelope,
};
use messaging::webhooks::{
    infobip, meta, report_failure_kind, DeliveredUpTo, StatusReport, WebhookEvent,
};
use sequent_core::types::messaging::{
    AccountSender, BillingCategory, CredentialName, MessageAttemptState, MessageChannel,
    MessagePurpose, MessageReason, MessagingProvider,
};
use serde_json::Value;
use tracing::{info, instrument, warn};

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

/// Meta's events, applied in full: a code a referral releases is sent
/// before this returns, under windmill's policy. Harvest answers the
/// callback first and sends the codes afterwards ([`meta_callback`]).
pub async fn meta_events(
    client: &mut DbClient,
    dispatcher: &Dispatcher,
    webhook_key: &str,
    signature: Option<&str>,
    body: &[u8],
) -> Result<WebhookOutcome> {
    let (outcome, codes) = meta_callback(client, webhook_key, signature, body).await?;
    send_codes(client, dispatcher, codes).await;
    Ok(outcome)
}

/// Meta's events, stored, with each Messenger link a referral opened
/// bound: what the callback is answered after. The codes those links
/// release are returned, to be sent once the callback is answered
/// ([`send_bound_code`]), each with its own capacity.
#[instrument(skip_all, err)]
pub async fn meta_callback(
    client: &mut DbClient,
    webhook_key: &str,
    signature: Option<&str>,
    body: &[u8],
) -> Result<(WebhookOutcome, Vec<BoundCode>)> {
    let Some(account) = account_for_key(client, webhook_key, META_PROVIDERS).await? else {
        return Ok((WebhookOutcome::NotFound, vec![]));
    };
    let Some(secret) = credential(client, &account, CredentialName::APP_SECRET).await? else {
        return Ok((WebhookOutcome::Unauthorized, vec![]));
    };
    if !meta::verify_signature(&secret, signature, body) {
        return Ok((WebhookOutcome::Unauthorized, vec![]));
    }
    // A verified body that isn't this account's, or isn't JSON, can't
    // become usable by retrying it: taken and ignored, with its reason.
    // Only storage and dispatch failures are retried.
    let events = match meta::parse_events(&account.sender, body) {
        Ok(events) => events,
        Err(reason) => {
            warn!(
                account = %account.id,
                key = webhook_key.get(..6).unwrap_or_default(),
                "Meta callback ignored: {reason}"
            );
            return Ok((WebhookOutcome::Accepted, vec![]));
        }
    };
    let codes = store_events(client, &account, events).await?;
    Ok((WebhookOutcome::Accepted, codes))
}

/// Logged when an authenticated Viber report can't be read: it is answered
/// as taken and dropped, never retried for ever (R7-24).
pub const REPORT_UNREADABLE: &str = "Viber report ignored: it can't be read";

/// Infobip Viber delivery reports. They come only through the account's
/// Infobip subscription, whose Basic auth (`authorization`, forwarded by the
/// webhook component) must be the account's report credentials, compared in
/// constant time. A report without it, with another one, or for an account
/// whose report credentials were never generated is refused unread.
#[instrument(skip_all, err)]
pub async fn viber_events(
    client: &mut DbClient,
    dispatcher: &Dispatcher,
    webhook_key: &str,
    authorization: Option<&str>,
    body: &[u8],
) -> Result<WebhookOutcome> {
    let Some(account) =
        account_for_key(client, webhook_key, &[MessagingProvider::VIBER_INFOBIP]).await?
    else {
        return Ok(WebhookOutcome::NotFound);
    };
    let expected = match (
        credential(client, &account, CredentialName::REPORT_USERNAME).await?,
        credential(client, &account, CredentialName::REPORT_PASSWORD).await?,
    ) {
        (Some(username), Some(password)) => Some(ReportCredentials { username, password }),
        _ => None,
    };
    if let Err(refusal) = verify_report_authorization(authorization, expected.as_ref()) {
        warn!(
            "Viber report for account {} refused: {}",
            account.id,
            refusal.reason()
        );
        return Ok(WebhookOutcome::Unauthorized);
    }
    // An authenticated report that can't be read can't become readable by
    // retrying it: taken and dropped, with its reason (R7-24, as for Meta).
    let events = match infobip::parse_events(body) {
        Ok(events) => events,
        Err(error) => {
            warn!(
                account = %account.id,
                key = webhook_key.get(..6).unwrap_or_default(),
                "{REPORT_UNREADABLE}: {error}"
            );
            return Ok(WebhookOutcome::Accepted);
        }
    };
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
    let certificate = CertificateCache::global()
        .get(&envelope.signing_cert_url)
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
                CertificateCache::global().confirm_subscription(url).await?;
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

/// What a provider's billing evidence says: WhatsApp's `pricing`
/// (`billable`, `category`) or Infobip's `price`.
fn billing_from_report(billing: Option<&Value>) -> (Option<BillingCategory>, Option<bool>) {
    let Some(billing) = billing else {
        return (None, None);
    };
    let category = billing["category"]
        .as_str()
        .and_then(BillingCategory::from_report);
    let billable = billing["billable"].as_bool().or_else(|| {
        billing["price"]["pricePerMessage"]
            .as_f64()
            .map(|price| price > 0.0)
    });
    (category, billable)
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
    let failure = (report.state == MessageAttemptState::FAILED)
        .then(|| report_failure_kind(account.provider, report.error_code.as_deref()));
    let mut error = report
        .error_code
        .as_ref()
        .map(|code| format!("provider code {code}"));
    // A failure the provider reports later is retried on the same channel
    // like one reported at once, if it may pass, unless it comes too late
    // or the message's origin can't render it again. Why not is recorded
    // next to the provider's reason.
    let now = Utc::now();
    let mut retry_at = failure.and_then(|kind| {
        next_retry_at(
            message.purpose.unwrap_or(MessagePurpose::NOTICE),
            message.attempt,
            kind,
            now,
        )
    });
    if retry_at.is_some() {
        let refusal = if reported_too_late(message.created_at, now) {
            Some(MessageReason::REPORTED_TOO_LATE)
        } else {
            retry_refusal(
                &tx,
                &message.tenant_id,
                message.logical_key.as_deref(),
                message.template_alias.as_deref(),
            )
            .await?
        };
        if let Some(reason) = refusal {
            retry_at = None;
            error = Some(reason.not_retried_after(error.as_deref()));
        }
    }
    let (billing_category, billable) = billing_from_report(report.billing.as_ref());
    let change = StateChange {
        error,
        failure,
        billing: report.billing.clone(),
        billing_category,
        billable,
        next_retry_at: retry_at,
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

/// Records a Messenger recipient's Get Started or referral: the 24-hour
/// window opens. Only this opens it; text the voter writes doesn't.
async fn record_contact(
    client: &mut DbClient,
    account: &MessagingAccount,
    page_scoped_id: &str,
    at: Option<DateTime<Utc>>,
) -> Result<()> {
    let Ok(destination) = Destination::parse(MessageChannel::MESSENGER, page_scoped_id) else {
        return Ok(());
    };
    let tx = client.transaction().await?;
    let key = tenant_key(&tx, &account.tenant_id, TenantKey::Destination).await?;
    // Never later than now, so a skewed timestamp can't stretch the window.
    let at = at.map_or_else(Utc::now, |at| at.min(Utc::now()));
    record_messenger_contact(
        &tx,
        &account.tenant_id,
        &account.id,
        &destination.digest(&key)?,
        at,
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

/// The events, applied in full: a code a referral releases is sent before
/// this returns, under windmill's policy.
pub async fn apply_events(
    client: &mut DbClient,
    dispatcher: &Dispatcher,
    account: &MessagingAccount,
    events: Vec<WebhookEvent>,
) -> Result<()> {
    let codes = store_events(client, account, events).await?;
    send_codes(client, dispatcher, codes).await;
    Ok(())
}

/// Sends the codes referrals released, one after the other, under
/// windmill's policy; a failure is logged and the next one still goes.
async fn send_codes(client: &mut DbClient, dispatcher: &Dispatcher, codes: Vec<BoundCode>) {
    for code in codes {
        if let Err(error) = send_bound_code(client, dispatcher, &code, SendPolicy::default()).await
        {
            warn!(link = %code.link_id, "could not send a Messenger code: {error:#}");
        }
    }
}

/// Applies the events: statuses, Get Started, number changes; a referral
/// binds its link. Returns the codes the bound links release, unsent.
///
/// A failure to store an event is an error (R7-04): the callback isn't
/// answered as taken, so the provider sends it again. Every event is
/// idempotent (`transition_message` moves only forward, a contact keeps
/// its latest time, a bound link binds again only for its own
/// conversation), so the events stored before the failure are harmless
/// when the retry applies them again; the codes bound before it are
/// released by the retry.
pub async fn store_events(
    client: &mut DbClient,
    account: &MessagingAccount,
    events: Vec<WebhookEvent>,
) -> Result<Vec<BoundCode>> {
    let mut codes = vec![];
    for event in events {
        let result = match &event {
            WebhookEvent::Status(report) => apply_status(client, account, report).await,
            WebhookEvent::MessengerGetStarted(contact)
                if account.channel == MessageChannel::MESSENGER =>
            {
                record_contact(client, account, &contact.page_scoped_id, contact.at).await
            }
            WebhookEvent::MessengerReferral(referral)
                if account.channel == MessageChannel::MESSENGER =>
            {
                // The window opens before the code that follows is sent.
                match record_contact(client, account, &referral.page_scoped_id, referral.at).await {
                    Ok(()) => match bind_referral(client, account, referral).await {
                        Ok(Some(code)) => {
                            codes.push(code);
                            Ok(())
                        }
                        Ok(None) => Ok(()),
                        Err(error) => Err(error),
                    },
                    Err(error) => Err(error),
                }
            }
            WebhookEvent::MessengerGetStarted(_) | WebhookEvent::MessengerReferral(_) => Ok(()),
            WebhookEvent::DeliveredUpTo(delivered) => {
                apply_delivered_up_to(client, account, delivered).await
            }
            // Flagged and logged; no voter's contact changes automatically.
            WebhookEvent::NumberChanged(change) if account.channel == MessageChannel::WHATSAPP => {
                let tx = client.transaction().await?;
                match flag_number_change(&tx, account, change).await {
                    Ok(_) => tx.commit().await.map_err(Into::into),
                    Err(error) => Err(error),
                }
            }
            WebhookEvent::NumberChanged(_) => Ok(()),
        };
        if let Err(error) = result {
            warn!(account = %account.id, "{EVENT_NOT_STORED}: {error:#}");
            return Err(error.context(EVENT_NOT_STORED));
        }
    }
    Ok(codes)
}

/// Logged when a callback's event couldn't be stored: the callback is
/// answered with an error so the provider sends it again.
pub const EVENT_NOT_STORED: &str =
    "A callback event couldn't be stored: the provider will send it again";

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

#[cfg(test)]
mod tests {
    use super::*;
    use messaging::sender::FailureKind;
    use serde_json::json;

    #[test]
    fn billing_evidence_is_read_from_reports() {
        assert_eq!(
            billing_from_report(Some(
                &json!({"billable": true, "category": "authentication"})
            )),
            (Some(BillingCategory::AUTHENTICATION), Some(true))
        );
        assert_eq!(
            billing_from_report(Some(
                &json!({"price": {"pricePerMessage": 0.02, "currency": "EUR"}})
            )),
            (None, Some(true))
        );
        assert_eq!(billing_from_report(None), (None, None));
    }

    #[test]
    fn late_failures_keep_their_classification() {
        assert_eq!(
            report_failure_kind(MessagingProvider::WHATSAPP_CLOUD_API, Some("130429")),
            FailureKind::TRANSIENT
        );
    }
}
