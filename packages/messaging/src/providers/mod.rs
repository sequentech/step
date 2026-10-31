// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Provider adapters. Each one sends through one provider's API and checks
//! an account's readiness; [`build`] picks the adapter from the account's
//! configuration, never from a customer or Post.

pub mod aws;
pub mod infobip;
pub mod messenger;
pub mod smtp;
pub mod whatsapp;

use crate::sender::{ChannelSender, FailureKind, SendOutcome};
use anyhow::{anyhow, Result};
use chrono::{TimeZone, Utc};
use sequent_core::types::messaging::{
    AccountLimits, AccountSender, AppSecretStatus, CredentialName, MessageChannel, MessagePurpose,
    MessageReason, MessagingProvider, ProviderSupport, TemplateStatus, TokenStatus,
};
use serde_json::Value;
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

/// The Graph API address for WhatsApp and Messenger.
pub const META_GRAPH_URL_ENV: &str = "MESSAGING_META_GRAPH_URL";
/// The Infobip API address for Viber.
pub const INFOBIP_URL_ENV: &str = "MESSAGING_INFOBIP_URL";
/// The endpoint of the AWS services (SES and SNS).
pub const AWS_ENDPOINT_URL_ENV: &str = "MESSAGING_AWS_ENDPOINT_URL";

/// Provider API addresses set by the deployment. The rule is the same for
/// every provider: an address the deployment sets wins over the account's
/// own; when it is unset, the account's own address is used, else the
/// provider's default.
#[derive(Debug, Clone, Default)]
pub struct Endpoints {
    /// [`META_GRAPH_URL_ENV`]. Default: the account's `api_base_url`, else
    /// [`META_GRAPH_URL`].
    pub meta_graph: Option<String>,
    /// [`INFOBIP_URL_ENV`]. Default: the account's `base_url`.
    pub infobip: Option<String>,
    /// [`AWS_ENDPOINT_URL_ENV`]. Default: the real AWS endpoints.
    pub aws: Option<String>,
}

/// What the log shows of a deployment address: its host and port only,
/// never credentials, a path or a query that may come with it.
pub fn logged_address(url: &str) -> String {
    match reqwest::Url::parse(url) {
        Ok(url) => match (url.host_str(), url.port()) {
            (Some(host), Some(port)) => format!("{host}:{port}"),
            (Some(host), None) => host.to_string(),
            (None, _) => "an address without a host".to_string(),
        },
        Err(_) => "an unreadable address".to_string(),
    }
}

impl Endpoints {
    /// Read the same way by harvest and windmill.
    pub fn from_env() -> Self {
        Self::from_vars(|name| std::env::var(name).ok())
    }

    /// Unset and blank variables leave the address to the account.
    pub fn from_vars(var: impl Fn(&str) -> Option<String>) -> Self {
        let read = |name: &str| {
            var(name)
                .map(|value| value.trim().trim_end_matches('/').to_string())
                .filter(|value| !value.is_empty())
        };
        let endpoints = Endpoints {
            meta_graph: read(META_GRAPH_URL_ENV),
            infobip: read(INFOBIP_URL_ENV),
            aws: read(AWS_ENDPOINT_URL_ENV),
        };
        for (name, value) in [
            (META_GRAPH_URL_ENV, &endpoints.meta_graph),
            (INFOBIP_URL_ENV, &endpoints.infobip),
            (AWS_ENDPOINT_URL_ENV, &endpoints.aws),
        ] {
            if let Some(value) = value {
                tracing::info!(
                    "{name} is set: accounts' own addresses are not used ({})",
                    logged_address(value)
                );
            }
        }
        endpoints
    }

    /// The deployment address that replaces this provider's account
    /// address, if one is set.
    pub fn deployment_address(&self, provider: MessagingProvider) -> Option<&str> {
        match provider {
            MessagingProvider::WHATSAPP_CLOUD_API | MessagingProvider::MESSENGER_SEND_API => {
                self.meta_graph.as_deref()
            }
            MessagingProvider::VIBER_INFOBIP => self.infobip.as_deref(),
            MessagingProvider::AWS_SES | MessagingProvider::AWS_SNS => self.aws.as_deref(),
            MessagingProvider::SMTP | MessagingProvider::HTTP_API | MessagingProvider::CONSOLE => {
                None
            }
        }
    }

    /// The sender with the deployment's address in place of the account's,
    /// for the adapters that take their address from the account.
    pub fn apply(&self, sender: &AccountSender) -> AccountSender {
        let mut sender = sender.clone();
        let Some(address) = self.deployment_address(sender.provider()) else {
            return sender;
        };
        match &mut sender {
            AccountSender::WHATSAPP_CLOUD_API { api_base_url, .. }
            | AccountSender::MESSENGER_SEND_API { api_base_url, .. } => {
                *api_base_url = Some(address.to_string());
            }
            AccountSender::VIBER_INFOBIP { base_url, .. } => {
                *base_url = address.to_string();
            }
            _ => {}
        }
        sender
    }
}

/// What an administrator is told when the deployment's address replaces
/// the account's: the account's own address isn't used.
pub fn endpoint_notice(endpoints: &Endpoints, provider: MessagingProvider) -> Option<String> {
    endpoints.deployment_address(provider).map(|address| {
        format!("This deployment sends {provider} requests to {address}; the account's own address is not used")
    })
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
    if provider.support() == ProviderSupport::UNSUPPORTED {
        return Err(anyhow!(MessageReason::UNSUPPORTED_PROVIDER.as_str()));
    }
    if provider.capabilities(account.channel).is_none() {
        return Err(anyhow!(
            "account {} uses {provider}, which cannot send {}",
            account.id,
            account.channel
        ));
    }
    for credential in provider.required_credentials() {
        account
            .credential(credential)
            .map_err(|_| anyhow!(missing_credential_reason(credential)))?;
    }
    let account = &Account {
        sender: endpoints.apply(&account.sender),
        ..account.clone()
    };
    Ok(match &account.sender {
        AccountSender::WHATSAPP_CLOUD_API { .. } => Box::new(whatsapp::WhatsAppSender::new(
            account.clone(),
            http.clone(),
            META_GRAPH_URL.to_string(),
        )?),
        AccountSender::MESSENGER_SEND_API { .. } => Box::new(messenger::MessengerSender::new(
            account.clone(),
            http.clone(),
            META_GRAPH_URL.to_string(),
        )?),
        AccountSender::VIBER_INFOBIP { .. } => Box::new(infobip::InfobipViberSender::new(
            account.clone(),
            http.clone(),
        )?),
        AccountSender::AWS_SNS { .. } => Box::new(aws::SnsSender::new(account, endpoints).await?),
        AccountSender::AWS_SES { .. } => Box::new(aws::SesSender::new(account, endpoints).await?),
        AccountSender::SMTP { .. } => Box::new(smtp::SmtpSender::new(account)?),
        // Removed providers were refused above: their accounts load, and
        // can't send.
        AccountSender::HTTP_API(_) | AccountSender::CONSOLE {} => {
            return Err(anyhow!(MessageReason::UNSUPPORTED_PROVIDER.as_str()))
        }
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

/// Meta's `debug_token` on the account's own token: whether it is valid,
/// when it expires, and which of the `needed` permissions it lacks. `None`
/// when Meta didn't answer it: the check says nothing it didn't read.
pub(crate) async fn meta_token_status(
    http: &reqwest::Client,
    base_url: &str,
    token: &str,
    needed: &[&str],
) -> Option<TokenStatus> {
    let (status, body) = exchange(
        http.get(format!("{base_url}/debug_token"))
            .query(&[("input_token", token)])
            .bearer_auth(token),
    )
    .await
    .ok()?;
    if status != 200 {
        return None;
    }
    let body: Value = serde_json::from_str(&body).ok()?;
    let data = body.get("data")?;
    let valid = data["is_valid"].as_bool()?;
    // 0 means the token doesn't expire.
    let expires_at = data["expires_at"]
        .as_i64()
        .filter(|seconds| *seconds > 0)
        .and_then(|seconds| Utc.timestamp_opt(seconds, 0).single())
        .map(|at| at.to_rfc3339());
    let missing_scopes = match data["scopes"].as_array() {
        Some(scopes) => needed
            .iter()
            .filter(|scope| !scopes.iter().any(|s| s.as_str() == Some(scope)))
            .map(|scope| scope.to_string())
            .collect(),
        None => vec![],
    };
    Some(TokenStatus {
        valid,
        expires_at,
        missing_scopes,
    })
}

/// Meta's `message_templates` answer: every template with its category
/// and status per language, and the languages approved per purpose
/// (authentication templates carry codes, utility templates notices).
pub(crate) fn meta_templates(
    body: &str,
) -> (Vec<TemplateStatus>, BTreeMap<MessagePurpose, Vec<String>>) {
    let body: Value = serde_json::from_str(body).unwrap_or_default();
    let mut templates = vec![];
    let mut approved: BTreeMap<MessagePurpose, Vec<String>> = BTreeMap::new();
    for template in body["data"].as_array().cloned().unwrap_or_default() {
        let (Some(name), Some(language), Some(status)) = (
            template["name"].as_str(),
            template["language"].as_str(),
            template["status"].as_str(),
        ) else {
            continue;
        };
        let category = template["category"].as_str();
        templates.push(TemplateStatus {
            name: name.to_string(),
            language: language.to_string(),
            category: category.map(str::to_string),
            status: status.to_string(),
        });
        if status != "APPROVED" {
            continue;
        }
        let purpose = match category {
            Some("AUTHENTICATION") => MessagePurpose::OTP,
            Some("UTILITY") => MessagePurpose::NOTICE,
            _ => continue,
        };
        let languages = approved.entry(purpose).or_default();
        if !languages.iter().any(|l| l == language) {
            languages.push(language.to_string());
        }
    }
    (templates, approved)
}

/// The reason given when a provider has an API for templates and it
/// couldn't be read.
pub const TEMPLATES_NOT_READ: &str = "The provider's templates couldn't be read";

// The provider checks' reasons (R5-15): constants, or a constant start
// followed by one value, so the Admin Portal translates every one of them
// (`messagingReasons.ts`). Never a provider's body or a credential.

/// The provider's API didn't answer.
pub const PROVIDER_UNREACHABLE: &str = "The provider couldn't be reached";
/// Followed by the HTTP status the provider answered a check with.
pub const PROVIDER_STATUS_PREFIX: &str = "The provider answered with status ";
/// Followed by the name of the credential the account lacks.
pub const MISSING_CREDENTIAL_PREFIX: &str = "Missing credential: ";
/// Followed by what the provider's SDK said (AWS).
pub const CHECK_FAILED_PREFIX: &str = "The provider's check failed: ";
/// Followed by Meta's code for the number's status (`PENDING`, ...).
pub const NUMBER_STATUS_PREFIX: &str = "The number's status is ";
/// Followed by Meta's code for the display name's review.
pub const NUMBER_NAME_PREFIX: &str = "The number's display name is ";
pub const NUMBER_IN_SANDBOX: &str = "The number is in sandbox mode";
pub const NUMBER_NOT_VERIFIED: &str = "The number's code is not verified";
pub const SMS_SANDBOX: &str = "The account is in the SMS sandbox";
pub const SES_SANDBOX: &str = "The account is in the SES sandbox";
pub const SES_SENDING_DISABLED: &str = "Sending is disabled for this account";
pub const SMTP_REFUSED: &str = "SMTP server unreachable or credentials rejected";
/// Meta's `debug_token` says the token isn't valid, or it has expired.
pub const TOKEN_INVALID: &str = "The access token isn't valid or has expired";
/// The token lacks the permission to send (R5-11).
pub const TOKEN_CANNOT_SEND: &str = "The access token lacks the permission to send";
/// Meta's `debug_token` didn't answer.
pub const TOKEN_NOT_CHECKED: &str = "The access token couldn't be checked";
/// Meta refused the proof made with the app secret (R5-16).
pub const APP_SECRET_REFUSED: &str = "Meta refused the app secret";
/// The proof made with the app secret couldn't be checked (R5-16).
pub const APP_SECRET_NOT_CHECKED: &str = "The app secret couldn't be checked";

/// [`PROVIDER_STATUS_PREFIX`] with the status.
pub fn provider_status_reason(status: u16) -> String {
    format!("{PROVIDER_STATUS_PREFIX}{status}")
}

/// [`MISSING_CREDENTIAL_PREFIX`] with the credential's name.
pub fn missing_credential_reason(name: CredentialName) -> String {
    format!("{MISSING_CREDENTIAL_PREFIX}{name}")
}

/// [`CHECK_FAILED_PREFIX`] with what failed.
pub fn check_failed_reason(detail: impl std::fmt::Display) -> String {
    format!("{CHECK_FAILED_PREFIX}{detail}")
}

/// The permission a Meta token needs to send, by provider: WhatsApp's and
/// the Page's messaging permission. `None` for other providers.
pub fn meta_send_scope(provider: MessagingProvider) -> Option<&'static str> {
    match provider {
        MessagingProvider::WHATSAPP_CLOUD_API => Some("whatsapp_business_messaging"),
        MessagingProvider::MESSENGER_SEND_API => Some("pages_messaging"),
        _ => None,
    }
}

/// Checks the account's app secret with Meta (R5-16): a Graph lookup of
/// `object_id` with `appsecret_proof`, the HMAC-SHA256 of the token keyed
/// by the secret. Graph refuses a proof that doesn't match the app's
/// secret, with an error that names `appsecret_proof`. `None` when the
/// account has no app secret.
pub(crate) async fn meta_app_secret_status(
    http: &reqwest::Client,
    base_url: &str,
    object_id: &str,
    token: &str,
    app_secret: Option<&str>,
) -> Option<AppSecretStatus> {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    let app_secret = app_secret.filter(|secret| !secret.is_empty())?;
    let not_checked = || AppSecretStatus {
        valid: None,
        reason: Some(APP_SECRET_NOT_CHECKED.to_string()),
    };
    let Ok(mut mac) = Hmac::<Sha256>::new_from_slice(app_secret.as_bytes()) else {
        return Some(not_checked());
    };
    mac.update(token.as_bytes());
    let proof = hex::encode(mac.finalize().into_bytes());
    Some(
        match exchange(
            http.get(format!("{base_url}/{object_id}"))
                .query(&[("fields", "id"), ("appsecret_proof", proof.as_str())])
                .bearer_auth(token),
        )
        .await
        {
            Ok((200, _)) => AppSecretStatus {
                valid: Some(true),
                reason: None,
            },
            Ok((400 | 403, body)) if body.contains("appsecret_proof") => AppSecretStatus {
                valid: Some(false),
                reason: Some(APP_SECRET_REFUSED.to_string()),
            },
            _ => not_checked(),
        },
    )
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

#[cfg(test)]
mod tests {

    /// Regression (R1-17): an override URL was logged whole, credentials
    /// in it too.
    #[test]
    fn logs_name_an_override_by_its_host_only() {
        assert_eq!(
            super::logged_address("https://user:pass@graph.example.org:8443/v1?token=x"),
            "graph.example.org:8443"
        );
        assert_eq!(
            super::logged_address("http://harvest:8400/fake-provider/meta"),
            "harvest:8400"
        );
        assert_eq!(super::logged_address("not a url"), "an unreadable address");
    }
    use super::*;
    use crate::destination::Destination;
    use crate::sender::OutboundMessage;
    use crate::test_server::{Reply, TestServer};
    use sequent_core::types::messaging::{MessageContent, MessagePurpose, OutOfWindowPolicy};
    use serde_json::json;
    use std::collections::HashMap;

    fn vars(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let map: HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        move |name| map.get(name).cloned()
    }

    fn whatsapp(api_base_url: Option<&str>) -> AccountSender {
        AccountSender::WHATSAPP_CLOUD_API {
            business_account_id: "waba-1".to_string(),
            phone_number_id: "pn-1".to_string(),
            display_phone_number: "+63 917 000 0000".to_string(),
            display_name: None,
            api_version: "v23.0".to_string(),
            api_base_url: api_base_url.map(str::to_string),
        }
    }

    fn viber(base_url: &str) -> AccountSender {
        AccountSender::VIBER_INFOBIP {
            base_url: base_url.to_string(),
            sender: "Elections".to_string(),
            approved_templates: BTreeMap::new(),
        }
    }

    #[test]
    fn deployment_addresses_come_from_the_environment() {
        let endpoints = Endpoints::from_vars(vars(&[
            (
                META_GRAPH_URL_ENV,
                " http://harvest:8400/fake-provider/meta/ ",
            ),
            (INFOBIP_URL_ENV, "http://harvest:8400/fake-provider/infobip"),
            (AWS_ENDPOINT_URL_ENV, "  "),
        ]));
        assert_eq!(
            endpoints.meta_graph.as_deref(),
            Some("http://harvest:8400/fake-provider/meta")
        );
        assert_eq!(
            endpoints.infobip.as_deref(),
            Some("http://harvest:8400/fake-provider/infobip")
        );
        assert_eq!(endpoints.aws, None);
        let unset = Endpoints::from_vars(vars(&[]));
        assert_eq!(
            (unset.meta_graph, unset.infobip, unset.aws),
            (None, None, None)
        );
    }

    #[test]
    fn a_deployment_address_wins_over_the_accounts_for_meta_and_infobip_alike() {
        let deployment = Endpoints {
            meta_graph: Some("http://graph.internal".to_string()),
            infobip: Some("http://infobip.internal".to_string()),
            aws: None,
        };
        let unset = Endpoints::default();
        for (sender, expected_set, expected_unset) in [
            (
                whatsapp(Some("https://bsp.example.org")),
                whatsapp(Some("http://graph.internal")),
                whatsapp(Some("https://bsp.example.org")),
            ),
            (
                whatsapp(None),
                whatsapp(Some("http://graph.internal")),
                whatsapp(None),
            ),
            (
                viber("https://abc.api.infobip.com"),
                viber("http://infobip.internal"),
                viber("https://abc.api.infobip.com"),
            ),
        ] {
            assert_eq!(deployment.apply(&sender), expected_set);
            assert_eq!(unset.apply(&sender), expected_unset);
        }
        assert!(
            endpoint_notice(&deployment, MessagingProvider::VIBER_INFOBIP)
                .unwrap()
                .contains("http://infobip.internal")
        );
        assert_eq!(
            endpoint_notice(&unset, MessagingProvider::VIBER_INFOBIP),
            None
        );
        assert_eq!(endpoint_notice(&deployment, MessagingProvider::SMTP), None);
    }

    fn account(channel: MessageChannel, sender: AccountSender) -> Account {
        Account {
            id: "account-1".to_string(),
            channel,
            sender,
            credentials: BTreeMap::from([(CredentialName::API_KEY, "key-1".to_string())]),
            limits: AccountLimits::default(),
            callback_url: None,
        }
    }

    #[tokio::test]
    async fn infobip_sends_to_the_deployment_address() {
        let server = TestServer::start(vec![Reply::Json(
            200,
            json!({"messages": [{"messageId": "k:1"}]}),
        )])
        .await;
        let endpoints = Endpoints {
            infobip: Some(server.base_url.clone()),
            ..Default::default()
        };
        let sender = build(
            &account(MessageChannel::VIBER, viber("https://unused.example.org")),
            &reqwest::Client::new(),
            &endpoints,
        )
        .await
        .unwrap();
        let outcome = sender
            .send(&OutboundMessage {
                channel: MessageChannel::VIBER,
                purpose: MessagePurpose::NOTICE,
                destination: Destination::parse(MessageChannel::VIBER, "+639171234567").unwrap(),
                language: Some("en".to_string()),
                content: MessageContent {
                    subject: None,
                    text: "Voting opens tomorrow.".to_string(),
                    html: None,
                    template_parameters: vec![],
                    code: None,
                },
                provider_template: Some("tpl".to_string()),
                idempotency_key: "k:1".to_string(),
                expires_at: None,
                last_inbound_at: None,
                out_of_window: OutOfWindowPolicy::DISABLED,
            })
            .await;
        assert!(
            matches!(outcome, SendOutcome::Accepted { .. }),
            "{outcome:?}"
        );
        assert_eq!(server.requests()[0].path, "/viber/2/messages");
    }

    #[tokio::test]
    async fn removed_providers_load_and_cannot_send() {
        for sender in [
            serde_json::from_value::<AccountSender>(json!({"provider": "CONSOLE"})).unwrap(),
            serde_json::from_value::<AccountSender>(json!({
                "provider": "HTTP_API",
                "send": {"url": "https://gateway.example.org/send"},
            }))
            .unwrap(),
        ] {
            let error = build(
                &account(MessageChannel::SMS, sender),
                &reqwest::Client::new(),
                &Endpoints::default(),
            )
            .await
            .err()
            .expect("refused");
            assert_eq!(
                error.to_string(),
                "Unsupported provider: replace this account"
            );
        }
    }
}
