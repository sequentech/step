// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Sending accounts: credentials, connection checks and the callback URLs
//! administrators enter at the provider. Callbacks reach Step only through
//! the webhook component (`messaging-webhooks`), never harvest directly.

use super::keys::credential_secret_key;
use crate::postgres::messaging::{
    delete_messaging_account, get_messaging_account, record_credentials_replaced,
    update_messaging_account_status, MessagingAccount,
};
use crate::services::vault::{delete_secret, read_secret, replace_secret};
use anyhow::{anyhow, Result};
use chrono::Utc;
use deadpool_postgres::Transaction;
use messaging::providers::{self, Account, Endpoints};
use messaging::relay::{CallbackProvider, WEBHOOKS_PUBLIC_URL_ENV};
use sequent_core::types::messaging::{AccountCheck, CredentialName, MessagingProvider};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use strum::IntoEnumIterator;
use tracing::warn;
use uuid::Uuid;

/// Why an account has, or lacks, a callback URL. Computed only by
/// [`callback_readiness`]; the account check, the Admin Portal and the logs
/// all show its reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallbackReadiness {
    /// The provider sends no callbacks to Step (SMS through SNS, SMTP).
    NotUsed,
    /// The URL the administrator enters at the provider, on the webhook
    /// component.
    Ready(String),
    /// `MESSAGING_WEBHOOKS_PUBLIC_URL` is unset. Delivery receipts are
    /// unavailable (never shown as "0 delivered"), and Messenger can't take
    /// codes, because they need Get Started.
    NotConfigured,
}

/// The reason of [`CallbackReadiness::NotConfigured`].
pub const CALLBACK_NOT_CONFIGURED: &str = "Callback address not configured";

impl CallbackReadiness {
    /// The callback URL, when there is one.
    pub fn url(&self) -> Option<&str> {
        match self {
            Self::Ready(url) => Some(url),
            Self::NotUsed | Self::NotConfigured => None,
        }
    }

    /// Why there is no callback URL, when the provider needs one.
    pub fn reason(&self) -> Option<&'static str> {
        match self {
            Self::NotConfigured => Some(CALLBACK_NOT_CONFIGURED),
            Self::NotUsed | Self::Ready(_) => None,
        }
    }
}

/// The webhook component's public base URL, from
/// `MESSAGING_WEBHOOKS_PUBLIC_URL`.
fn webhooks_public_url() -> Option<String> {
    std::env::var(WEBHOOKS_PUBLIC_URL_ENV)
        .ok()
        .map(|url| url.trim().trim_end_matches('/').to_string())
        .filter(|url| !url.is_empty())
}

/// The component route family a provider's callbacks use, if any.
fn callback_provider(provider: MessagingProvider) -> Option<CallbackProvider> {
    match provider {
        MessagingProvider::WHATSAPP_CLOUD_API | MessagingProvider::MESSENGER_SEND_API => {
            Some(CallbackProvider::META)
        }
        MessagingProvider::VIBER_INFOBIP => Some(CallbackProvider::VIBER),
        MessagingProvider::AWS_SES => Some(CallbackProvider::AWS),
        MessagingProvider::AWS_SNS
        | MessagingProvider::SMTP
        | MessagingProvider::HTTP_API
        | MessagingProvider::CONSOLE => None,
    }
}

/// [`callback_readiness`] for a given public base URL.
pub fn callback_readiness_with(
    component_url: Option<&str>,
    provider: MessagingProvider,
    webhook_key: &str,
) -> CallbackReadiness {
    let Some(route) = callback_provider(provider) else {
        return CallbackReadiness::NotUsed;
    };
    match component_url.map(str::trim).filter(|url| !url.is_empty()) {
        Some(base) => CallbackReadiness::Ready(route.callback_url(base, webhook_key)),
        None => CallbackReadiness::NotConfigured,
    }
}

/// Where the provider of `account` sends delivery reports and events: the
/// webhook component's route for the provider
/// ([`CallbackProvider::callback_url`] on `MESSAGING_WEBHOOKS_PUBLIC_URL`).
pub fn callback_readiness(account: &MessagingAccount) -> CallbackReadiness {
    callback_readiness_with(
        webhooks_public_url().as_deref(),
        account.provider,
        &account.webhook_key,
    )
}

/// The account's callback URL for its adapter. Says once per process when
/// callbacks are not configured.
fn runtime_callback_url(account: &MessagingAccount) -> Option<String> {
    static WARNED: AtomicBool = AtomicBool::new(false);
    let readiness = callback_readiness(account);
    if let Some(reason) = readiness.reason() {
        if !WARNED.swap(true, Ordering::Relaxed) {
            warn!(provider = %account.provider, "{reason}: delivery reports and Get Started can't reach Step");
        }
    }
    readiness.url().map(str::to_string)
}

/// The account with its credentials decrypted, ready for an adapter.
pub async fn runtime_account(tx: &Transaction<'_>, account: &MessagingAccount) -> Result<Account> {
    let tenant = account.tenant_id.to_string();
    let mut credentials = BTreeMap::new();
    for name in account.provider.accepted_credentials() {
        if let Some(value) =
            read_secret(tx, &tenant, None, &credential_secret_key(&account.id, name)).await?
        {
            credentials.insert(name, value);
        }
    }
    Ok(Account {
        id: account.id.to_string(),
        channel: account.channel,
        sender: account.sender.clone(),
        credentials,
        limits: account.limits.clone(),
        callback_url: runtime_callback_url(account),
    })
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReplacedCredentials {
    pub replaced: Vec<CredentialName>,
    /// Only when a new verify token was generated. Shown once.
    pub verify_token: Option<String>,
}

/// Replaces credentials. The values go to the secret store; the account
/// only records when each was replaced.
pub async fn replace_credentials(
    tx: &Transaction<'_>,
    tenant_id: &Uuid,
    account_id: &Uuid,
    values: &BTreeMap<CredentialName, String>,
    generate_verify_token: bool,
) -> Result<Option<ReplacedCredentials>> {
    let Some(account) = get_messaging_account(tx, tenant_id, account_id).await? else {
        return Ok(None);
    };
    let accepted = account.provider.accepted_credentials();
    let mut to_store: BTreeMap<CredentialName, String> = BTreeMap::new();
    for (name, value) in values {
        if *name == CredentialName::VERIFY_TOKEN {
            return Err(anyhow!("the verify token is generated, not entered"));
        }
        if name.is_generated() {
            return Err(anyhow!("{name} is generated, not entered"));
        }
        if !accepted.contains(name) {
            return Err(anyhow!("{} accounts do not use {name}", account.provider));
        }
        if value.trim().is_empty() {
            return Err(anyhow!("{name} cannot be empty"));
        }
        to_store.insert(*name, value.trim().to_string());
    }
    let verify_token = if generate_verify_token {
        if !accepted.contains(&CredentialName::VERIFY_TOKEN) {
            return Err(anyhow!(
                "{} accounts have no verify token",
                account.provider
            ));
        }
        let token = messaging::link::new_reference();
        to_store.insert(CredentialName::VERIFY_TOKEN, token.clone());
        Some(token)
    } else {
        None
    };
    let tenant = tenant_id.to_string();
    for (name, value) in &to_store {
        replace_secret(
            tx,
            &tenant,
            &credential_secret_key(account_id, *name),
            value,
        )
        .await?;
    }
    let replaced: Vec<CredentialName> = to_store.keys().copied().collect();
    record_credentials_replaced(tx, tenant_id, account_id, &replaced, Utc::now()).await?;
    Ok(Some(ReplacedCredentials {
        replaced,
        verify_token,
    }))
}

/// Generates new report credentials for an Infobip Viber account: the Basic
/// auth its report subscription sends. Returned once, to be shown once;
/// only the vault keeps them. The previous ones stop working.
pub async fn generate_report_credentials(
    tx: &Transaction<'_>,
    tenant_id: &Uuid,
    account_id: &Uuid,
) -> Result<Option<messaging::providers::infobip::ReportCredentials>> {
    let Some(account) = get_messaging_account(tx, tenant_id, account_id).await? else {
        return Ok(None);
    };
    if !account
        .provider
        .accepted_credentials()
        .contains(&CredentialName::REPORT_PASSWORD)
    {
        return Err(anyhow!(
            "{} accounts have no report credentials",
            account.provider
        ));
    }
    let generated = messaging::providers::infobip::new_report_credentials();
    let tenant = tenant_id.to_string();
    for (name, value) in [
        (CredentialName::REPORT_USERNAME, &generated.username),
        (CredentialName::REPORT_PASSWORD, &generated.password),
    ] {
        replace_secret(tx, &tenant, &credential_secret_key(account_id, name), value).await?;
    }
    record_credentials_replaced(
        tx,
        tenant_id,
        account_id,
        &[
            CredentialName::REPORT_USERNAME,
            CredentialName::REPORT_PASSWORD,
        ],
        Utc::now(),
    )
    .await?;
    Ok(Some(generated))
}

/// Checks the account with its provider and stores the result.
pub async fn check_account(
    tx: &Transaction<'_>,
    tenant_id: &Uuid,
    account_id: &Uuid,
    endpoints: &Endpoints,
) -> Result<Option<AccountCheck>> {
    let Some(account) = get_messaging_account(tx, tenant_id, account_id).await? else {
        return Ok(None);
    };
    let runtime = runtime_account(tx, &account).await?;
    let check = match providers::build(&runtime, &providers::http_client()?, endpoints).await {
        Ok(sender) => sender.check().await,
        Err(error) => AccountCheck {
            checked_at: Some(Utc::now().to_rfc3339()),
            reason: Some(error.to_string()),
            ..Default::default()
        },
    };
    update_messaging_account_status(tx, tenant_id, account_id, &check).await?;
    Ok(Some(check))
}

/// Deletes the account and its credentials.
pub async fn delete_account(
    tx: &Transaction<'_>,
    tenant_id: &Uuid,
    account_id: &Uuid,
) -> Result<bool> {
    let tenant = tenant_id.to_string();
    for name in CredentialName::iter() {
        delete_secret(tx, &tenant, &credential_secret_key(account_id, name)).await?;
    }
    delete_messaging_account(tx, tenant_id, account_id).await
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    #[test]
    fn callbacks_go_to_the_webhook_component() {
        // The route shape itself is pinned by the relay's tests.
        let base = "https://callbacks.example.org/";
        for (provider, route) in [
            (
                MessagingProvider::WHATSAPP_CLOUD_API,
                CallbackProvider::META,
            ),
            (
                MessagingProvider::MESSENGER_SEND_API,
                CallbackProvider::META,
            ),
            (MessagingProvider::VIBER_INFOBIP, CallbackProvider::VIBER),
            (MessagingProvider::AWS_SES, CallbackProvider::AWS),
        ] {
            let readiness = callback_readiness_with(Some(base), provider, KEY);
            let expected = route.callback_url(base, KEY);
            assert!(expected.starts_with("https://callbacks.example.org/"));
            assert!(expected.ends_with(KEY));
            assert_eq!(readiness.url(), Some(expected.as_str()), "{provider}");
            assert_eq!(readiness.reason(), None);
        }
    }

    #[test]
    fn without_the_public_address_the_reason_says_so() {
        for base in [None, Some(""), Some("  ")] {
            let readiness =
                callback_readiness_with(base, MessagingProvider::MESSENGER_SEND_API, KEY);
            assert_eq!(readiness, CallbackReadiness::NotConfigured);
            assert_eq!(readiness.url(), None);
            assert_eq!(readiness.reason(), Some("Callback address not configured"));
        }
    }

    #[test]
    fn providers_without_callbacks_need_no_address() {
        for provider in [
            MessagingProvider::AWS_SNS,
            MessagingProvider::SMTP,
            MessagingProvider::HTTP_API,
            MessagingProvider::CONSOLE,
        ] {
            for base in [None, Some("https://callbacks.example.org")] {
                let readiness = callback_readiness_with(base, provider, KEY);
                assert_eq!(readiness, CallbackReadiness::NotUsed, "{provider}");
                assert_eq!(readiness.reason(), None);
            }
        }
    }
}
