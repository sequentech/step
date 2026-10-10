// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Provider adapters. Each one sends through one provider's API and checks
//! an account's readiness; [`build`] picks the adapter from the account's
//! configuration, never from a customer or Post.

pub mod aws;
pub mod console;
pub mod http_api;
pub mod infobip;
pub mod messenger;
pub mod smtp;
pub mod whatsapp;

use crate::sender::{ChannelSender, FailureKind, SendOutcome};
use anyhow::{anyhow, Result};
use sequent_core::types::messaging::{
    AccountLimits, AccountSender, CredentialName, MessageChannel,
};
use std::collections::BTreeMap;
use std::time::Duration;

pub const META_GRAPH_URL: &str = "https://graph.facebook.com";
/// Requests that take longer end as unknown outcomes.
pub const PROVIDER_TIMEOUT: Duration = Duration::from_secs(15);

/// A sending account with its decrypted credentials.
#[derive(Clone)]
pub struct Account {
    pub id: String,
    pub channel: MessageChannel,
    pub sender: AccountSender,
    pub credentials: BTreeMap<CredentialName, String>,
    pub limits: AccountLimits,
    /// Where the provider sends delivery reports, when it takes one per
    /// request.
    pub callback_url: Option<String>,
}

impl std::fmt::Debug for Account {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Account")
            .field("id", &self.id)
            .field("channel", &self.channel)
            .field("sender", &self.sender)
            .field("credentials", &self.credentials.keys().collect::<Vec<_>>())
            .field("limits", &self.limits)
            .finish()
    }
}

impl Account {
    pub fn credential(&self, name: CredentialName) -> Result<&str> {
        self.credentials
            .get(&name)
            .map(String::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| anyhow!("account {} has no {name}", self.id))
    }
}

/// Base URLs of provider APIs that are not per account.
#[derive(Debug, Clone)]
pub struct Endpoints {
    pub meta_graph: String,
    /// Overrides the AWS service endpoints, such as a local test double.
    pub aws: Option<String>,
}

impl Default for Endpoints {
    fn default() -> Self {
        Endpoints {
            meta_graph: META_GRAPH_URL.to_string(),
            aws: None,
        }
    }
}

pub fn http_client() -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .timeout(PROVIDER_TIMEOUT)
        .build()?)
}

pub async fn build(
    account: &Account,
    http: &reqwest::Client,
    endpoints: &Endpoints,
) -> Result<Box<dyn ChannelSender>> {
    let provider = account.sender.provider();
    if provider.capabilities(account.channel).is_none() {
        return Err(anyhow!(
            "account {} uses {provider}, which cannot send {}",
            account.id,
            account.channel
        ));
    }
    for credential in provider.required_credentials() {
        account.credential(credential)?;
    }
    Ok(match &account.sender {
        AccountSender::WHATSAPP_CLOUD_API { .. } => Box::new(whatsapp::WhatsAppSender::new(
            account.clone(),
            http.clone(),
            endpoints.meta_graph.clone(),
        )?),
        AccountSender::MESSENGER_SEND_API { .. } => Box::new(messenger::MessengerSender::new(
            account.clone(),
            http.clone(),
            endpoints.meta_graph.clone(),
        )?),
        AccountSender::VIBER_INFOBIP { .. } => Box::new(infobip::InfobipViberSender::new(
            account.clone(),
            http.clone(),
        )?),
        AccountSender::AWS_SNS { .. } => Box::new(aws::SnsSender::new(account, endpoints).await?),
        AccountSender::AWS_SES { .. } => Box::new(aws::SesSender::new(account, endpoints).await?),
        AccountSender::SMTP { .. } => Box::new(smtp::SmtpSender::new(account)?),
        AccountSender::HTTP_API(_) => Box::new(http_api::HttpApiChannelSender::new(
            account.clone(),
            http.clone(),
        )?),
        AccountSender::CONSOLE {} => Box::new(console::ConsoleSender::new(account)),
    })
}

/// Graph API error codes that mean "try again later": application and
/// account rate limits, throughput and pair limits. Meta answers them with
/// a 4xx status, like permanent refusals.
const META_TRANSIENT_CODES: &[&str] = &["4", "17", "32", "613", "80007", "130429", "131056"];

/// Marks Meta's throttling refusals as transient.
pub(crate) fn meta_outcome(outcome: SendOutcome) -> SendOutcome {
    match outcome {
        SendOutcome::Rejected(mut failure)
            if failure
                .code
                .as_deref()
                .is_some_and(|code| META_TRANSIENT_CODES.contains(&code)) =>
        {
            failure.kind = FailureKind::TRANSIENT;
            SendOutcome::Rejected(failure)
        }
        other => other,
    }
}

/// Sends a request and reads its body, keeping the error for outcome
/// classification.
pub(crate) async fn exchange(
    request: reqwest::RequestBuilder,
) -> std::result::Result<(u16, String), reqwest::Error> {
    let response = request.send().await?;
    let status = response.status().as_u16();
    let body = response.text().await?;
    Ok((status, body))
}
