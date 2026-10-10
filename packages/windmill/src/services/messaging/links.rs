// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Messenger links. A voter proves a Messenger contact by opening the
//! event's Page with a one-time reference (Get Started or a referral),
//! receiving the code there and entering it in the original authentication
//! session. There is no word to type. Keycloak stays the authority on the
//! code; Step only holds it, encrypted, until it is sent.

use super::accounts::{callback_readiness_with, CallbackReadiness};
use super::config::get_event_messaging_config;
use super::dispatch::{deliver, Delivery, DeliveryResult, Dispatcher, Recipient};
use super::keys::{tenant_key, TenantKey};
use crate::postgres::messaging::{
    get_messaging_account, get_messenger_link_by_reference, get_pending_messenger_link,
    insert_messenger_link, list_messaging_accounts, update_messenger_link, MessagingAccount,
    NewMessengerLink,
};
use crate::services::vault::{decrypt_with_master_secret, encrypt_with_master_secret};
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Duration, Utc};
use deadpool_postgres::{Client as DbClient, Transaction};
use messaging::link::{check_link, digest, messenger_link, new_reference, LinkCheck};
use messaging::webhooks::MessengerReferral;
use sequent_core::types::messaging::{
    CreateMessengerLinkRequest, CreateMessengerLinkResponse, MessageAttemptState, MessageChannel,
    MessageContent, MessagePurpose, MessengerLinkRequest, MessengerLinkState, MessengerLinkStatus,
    OTP_TEMPLATE_KEY,
};
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};
use uuid::Uuid;

/// A reference never lives longer than this, even for longer-lived codes.
pub const MAX_LINK_LIFETIME_MINUTES: i64 = 10;

/// What is held, encrypted, until the voter interacts with the Page.
#[derive(Serialize, Deserialize)]
struct HeldCode {
    content: MessageContent,
    /// The only conversation the code may go to, for a saved voter.
    #[serde(default)]
    expected_recipient: Option<String>,
}

#[derive(Debug, PartialEq)]
pub enum LinkError {
    /// The event offers no Messenger channel for codes.
    MessengerNotEnabled,
    /// Unknown reference, or one that belongs to another session or code.
    NotFound,
    /// The webhook component's public address isn't configured, so Get
    /// Started can't reach Step and no code could be sent:
    /// "Callback address not configured"
    /// ([`super::accounts::CALLBACK_NOT_CONFIGURED`]).
    CallbackNotConfigured,
}

async fn messenger_account(
    tx: &Transaction<'_>,
    tenant_id: &Uuid,
    election_event_id: Option<&Uuid>,
) -> Result<Option<MessagingAccount>> {
    let config = match election_event_id {
        Some(event) => get_event_messaging_config(tx, tenant_id, event).await?,
        None => None,
    };
    let Some(config) = config else {
        return Ok(None);
    };
    if !config
        .enabled_channels(MessagePurpose::OTP, None)
        .contains(&MessageChannel::MESSENGER)
    {
        return Ok(None);
    }
    match config.channel(MessageChannel::MESSENGER) {
        Some(channel) => {
            get_messaging_account(tx, tenant_id, &Uuid::parse_str(&channel.account_id)?).await
        }
        None => Ok(list_messaging_accounts(tx, tenant_id)
            .await?
            .into_iter()
            .find(|a| a.channel == MessageChannel::MESSENGER && a.is_default)),
    }
}

fn parse_time(value: &str) -> Result<DateTime<Utc>> {
    Ok(DateTime::parse_from_rfc3339(value)
        .context("invalid expiry")?
        .with_timezone(&Utc))
}

/// Starts a Messenger link. `webhooks_public_url` is the webhook
/// component's public address (`MESSAGING_WEBHOOKS_PUBLIC_URL`, passed in
/// by the caller): without it Get Started can't reach Step, so the link is
/// refused.
#[instrument(skip_all, err)]
pub async fn create_link(
    client: &mut DbClient,
    request: &CreateMessengerLinkRequest,
    webhooks_public_url: Option<&str>,
) -> Result<std::result::Result<CreateMessengerLinkResponse, LinkError>> {
    let tenant_id = Uuid::parse_str(&request.tenant_id)?;
    let event_id = request
        .election_event_id
        .as_deref()
        .map(Uuid::parse_str)
        .transpose()?;
    let now = Utc::now();
    let expires_at =
        parse_time(&request.expires_at)?.min(now + Duration::minutes(MAX_LINK_LIFETIME_MINUTES));
    if expires_at <= now {
        return Err(anyhow!("the code has already expired"));
    }
    if request.code.trim().is_empty() {
        return Err(anyhow!("missing code"));
    }
    let tx = client.transaction().await?;
    let Some(account) = messenger_account(&tx, &tenant_id, event_id.as_ref()).await? else {
        tx.commit().await?;
        return Ok(Err(LinkError::MessengerNotEnabled));
    };
    if matches!(
        callback_readiness_with(webhooks_public_url, account.provider, &account.webhook_key),
        CallbackReadiness::NotConfigured
    ) {
        tx.commit().await?;
        return Ok(Err(LinkError::CallbackNotConfigured));
    }
    let Some(page) = account.sender.messenger_page() else {
        return Err(anyhow!("the Messenger account has no Page"));
    };
    let key = tenant_key(&tx, &tenant_id, TenantKey::MessengerLink).await?;
    let reference = new_reference();
    let mut content = request.content.clone();
    content.code = Some(request.code.clone());
    let expected_recipient = request
        .expected_recipient
        .as_deref()
        .map(str::trim)
        .filter(|psid| !psid.is_empty())
        .map(str::to_string);
    let payload = encrypt_with_master_secret(&serde_json::to_vec(&HeldCode {
        content,
        expected_recipient,
    })?)
    .await?;
    insert_messenger_link(
        &tx,
        &NewMessengerLink {
            tenant_id,
            election_event_id: event_id,
            account_id: account.id,
            reference_digest: digest(&key, &reference)?,
            auth_session_digest: request.auth_session.clone(),
            challenge_digest: request.challenge.clone(),
            encrypted_payload: payload,
            language: request.language.clone(),
            expires_at,
        },
    )
    .await?;
    tx.commit().await?;
    Ok(Ok(CreateMessengerLinkResponse {
        link: messenger_link(&page, &reference),
        reference,
        expires_at: expires_at.to_rfc3339(),
    }))
}

async fn checked_link(
    tx: &Transaction<'_>,
    request: &MessengerLinkRequest,
) -> Result<std::result::Result<(crate::postgres::messaging::MessengerLink, LinkCheck), LinkError>>
{
    let tenant_id = Uuid::parse_str(&request.tenant_id)?;
    let key = tenant_key(tx, &tenant_id, TenantKey::MessengerLink).await?;
    let Some(link) =
        get_messenger_link_by_reference(tx, &tenant_id, &digest(&key, &request.reference)?).await?
    else {
        return Ok(Err(LinkError::NotFound));
    };
    let check = check_link(
        link.state,
        link.expires_at,
        &link.auth_session_digest,
        &link.challenge_digest,
        &request.auth_session,
        &request.challenge,
        Utc::now(),
    );
    if check == LinkCheck::Mismatch {
        return Ok(Err(LinkError::NotFound));
    }
    if check == LinkCheck::Expired {
        update_messenger_link(tx, &link.id, MessengerLinkState::EXPIRED, None, None).await?;
    }
    Ok(Ok((link, check)))
}

fn status(state: MessengerLinkState) -> MessengerLinkStatus {
    MessengerLinkStatus {
        state,
        page_scoped_id: None,
        page_id: None,
    }
}

/// Progress of a link, for the page that waits for the code.
#[instrument(skip_all, err)]
pub async fn link_status(
    client: &mut DbClient,
    request: &MessengerLinkRequest,
) -> Result<std::result::Result<MessengerLinkStatus, LinkError>> {
    let tx = client.transaction().await?;
    let result = checked_link(&tx, request)
        .await?
        .map(|(link, check)| match check {
            LinkCheck::Valid => status(link.state),
            LinkCheck::Expired => status(MessengerLinkState::EXPIRED),
            LinkCheck::Finished(state) => status(state),
            LinkCheck::Mismatch => status(MessengerLinkState::EXPIRED),
        });
    tx.commit().await?;
    Ok(result)
}

/// Called once Keycloak accepted the code in the original session: the
/// link is spent and the Page-scoped ID may be stored on the voter.
#[instrument(skip_all, err)]
pub async fn confirm_link(
    client: &mut DbClient,
    request: &MessengerLinkRequest,
) -> Result<std::result::Result<MessengerLinkStatus, LinkError>> {
    let tx = client.transaction().await?;
    let result = match checked_link(&tx, request).await? {
        Ok((link, LinkCheck::Valid)) if link.state == MessengerLinkState::CODE_SENT => {
            update_messenger_link(&tx, &link.id, MessengerLinkState::CONFIRMED, None, None).await?;
            let account = get_messaging_account(&tx, &link.tenant_id, &link.account_id).await?;
            Ok(MessengerLinkStatus {
                state: MessengerLinkState::CONFIRMED,
                page_scoped_id: link.page_scoped_id,
                page_id: account
                    .and_then(|a| a.sender.messenger_page())
                    .map(|page| page.page_id),
            })
        }
        Ok((link, LinkCheck::Valid)) => Ok(status(link.state)),
        Ok((_, LinkCheck::Expired)) => Ok(status(MessengerLinkState::EXPIRED)),
        Ok((_, LinkCheck::Finished(state))) => Ok(status(state)),
        Ok((_, LinkCheck::Mismatch)) | Err(_) => Err(LinkError::NotFound),
    };
    tx.commit().await?;
    Ok(result)
}

/// A voter opened the Page with a reference: bind the conversation to the
/// pending link and send the held code there. The caller recorded the
/// conversation's Get Started first, so the code goes out inside the
/// 24-hour window. Returns the code's delivery, or `None` when no link
/// matched.
#[instrument(skip_all, err)]
pub async fn bind_referral(
    client: &mut DbClient,
    dispatcher: &Dispatcher,
    account: &MessagingAccount,
    referral: &MessengerReferral,
) -> Result<Option<DeliveryResult>> {
    if referral.reference.is_empty() {
        return Ok(None);
    }
    let tx = client.transaction().await?;
    let key = tenant_key(&tx, &account.tenant_id, TenantKey::MessengerLink).await?;
    let reference_digest = digest(&key, &referral.reference)?;
    let Some(link) = get_pending_messenger_link(&tx, &account.id, &reference_digest).await? else {
        tx.commit().await?;
        return Ok(None);
    };
    if link.expires_at <= Utc::now() {
        update_messenger_link(&tx, &link.id, MessengerLinkState::EXPIRED, None, None).await?;
        tx.commit().await?;
        return Ok(None);
    }
    // A link binds one conversation: a replay from another one is ignored.
    if link
        .page_scoped_id
        .as_ref()
        .is_some_and(|bound| *bound != referral.page_scoped_id)
    {
        tx.commit().await?;
        return Ok(None);
    }
    let Some(payload) = &link.encrypted_payload else {
        tx.commit().await?;
        return Ok(None);
    };
    let held: HeldCode = serde_json::from_slice(&decrypt_with_master_secret(payload).await?)?;
    if held
        .expected_recipient
        .as_ref()
        .is_some_and(|expected| *expected != referral.page_scoped_id)
    {
        // Not the saved voter's conversation: the link is used up and
        // nothing is sent, answered exactly as an unknown reference, so a
        // live link can't be told from a decoy.
        update_messenger_link(&tx, &link.id, MessengerLinkState::EXPIRED, None, None).await?;
        tx.commit().await?;
        info!(link = %link.id, "Messenger link opened from another conversation: not sent");
        return Ok(None);
    }
    update_messenger_link(
        &tx,
        &link.id,
        MessengerLinkState::PENDING,
        Some(&referral.page_scoped_id),
        None,
    )
    .await?;
    tx.commit().await?;

    let result = deliver(
        client,
        dispatcher,
        &Delivery {
            tenant_id: link.tenant_id,
            election_event_id: link.election_event_id,
            purpose: MessagePurpose::OTP,
            recipient: Recipient {
                voter_id: None,
                channel: MessageChannel::MESSENGER,
                destination: referral.page_scoped_id.clone(),
                election_ids: vec![],
            },
            content: Some(held.content),
            language: link.language.clone(),
            template_alias: None,
            logical_key: format!("messenger-link:{}", link.id),
            expires_at: Some(link.expires_at),
            account_id: Some(link.account_id),
            template_key: Some(OTP_TEMPLATE_KEY.to_string()),
            provider_template: None,
        },
    )
    .await?;
    let sent = matches!(
        result.state,
        MessageAttemptState::ACCEPTED
            | MessageAttemptState::DELIVERED
            | MessageAttemptState::UNKNOWN
    );
    let tx = client.transaction().await?;
    if sent {
        update_messenger_link(
            &tx,
            &link.id,
            MessengerLinkState::CODE_SENT,
            None,
            result.message_id.as_ref(),
        )
        .await?;
    }
    tx.commit().await?;
    info!(link = %link.id, state = %result.state, "Messenger link interaction");
    Ok(Some(result))
}
