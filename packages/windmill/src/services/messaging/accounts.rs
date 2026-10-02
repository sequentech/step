// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Sending accounts: credentials, connection checks and the callback URLs
//! administrators enter at the provider.

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
use sequent_core::types::messaging::{AccountCheck, CredentialName, MessagingProvider};
use std::collections::BTreeMap;
use strum::IntoEnumIterator;
use uuid::Uuid;

/// Public base URL of harvest, where providers send webhooks.
pub const HARVEST_PUBLIC_URL_ENV: &str = "HARVEST_PUBLIC_URL";

pub fn public_base_url() -> Option<String> {
    std::env::var(HARVEST_PUBLIC_URL_ENV)
        .ok()
        .map(|url| url.trim_end_matches('/').to_string())
        .filter(|url| !url.is_empty())
}

/// Where the provider of `account` sends delivery reports and replies.
pub fn callback_url(base_url: &str, account: &MessagingAccount) -> Option<String> {
    let path = match account.provider {
        MessagingProvider::WHATSAPP_CLOUD_API | MessagingProvider::MESSENGER_SEND_API => "meta",
        MessagingProvider::VIBER_INFOBIP => "viber",
        MessagingProvider::AWS_SES => "aws",
        MessagingProvider::AWS_SNS | MessagingProvider::SMTP | MessagingProvider::CONSOLE => {
            return None
        }
    };
    Some(format!(
        "{base_url}/webhooks/{path}/{}",
        account.webhook_key
    ))
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
        callback_url: public_base_url().and_then(|base| callback_url(&base, account)),
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
