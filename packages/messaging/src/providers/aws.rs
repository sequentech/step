// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Amazon SES (email) and SNS (SMS). Without account keys the service's own
//! AWS role is used, as before.

use super::{Account, Endpoints, PROVIDER_TIMEOUT};
use crate::sender::{ChannelSender, FailureKind, OutboundMessage, ProviderFailure, SendOutcome};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use aws_config::retry::RetryConfig;
use aws_config::timeout::TimeoutConfig;
use aws_config::{BehaviorVersion, Region, SdkConfig};
use aws_credential_types::Credentials;
use aws_sdk_sns::error::{ProvideErrorMetadata, SdkError};
use aws_sdk_sns::types::MessageAttributeValue;
use chrono::Utc;
use sequent_core::types::messaging::{
    AccountCheck, AccountSender, CredentialName, MessageChannel, MessagingProvider,
    ProviderCapabilities,
};
use std::collections::HashMap;

const THROTTLING_CODES: &[&str] = &[
    "Throttling",
    "ThrottlingException",
    "TooManyRequestsException",
    "ThrottledException",
];

async fn sdk_config(
    account: &Account,
    region: Option<&String>,
    endpoints: &Endpoints,
) -> SdkConfig {
    // Retries are decided from the ledger: an SDK retry after an ambiguous
    // server error could send twice.
    let mut loader = aws_config::defaults(BehaviorVersion::latest())
        .retry_config(RetryConfig::disabled())
        .timeout_config(
            TimeoutConfig::builder()
                .operation_timeout(PROVIDER_TIMEOUT)
                .build(),
        );
    if let Some(region) = region {
        loader = loader.region(Region::new(region.clone()));
    }
    if let (Ok(id), Ok(secret)) = (
        account.credential(CredentialName::AWS_ACCESS_KEY_ID),
        account.credential(CredentialName::AWS_SECRET_ACCESS_KEY),
    ) {
        loader = loader.credentials_provider(Credentials::new(
            id,
            secret,
            None,
            None,
            "messaging-account",
        ));
    }
    if let Some(endpoint) = &endpoints.aws {
        loader = loader.endpoint_url(endpoint);
    }
    loader.load().await
}

/// Service errors are refusals unless they are throttling or a server
/// error; anything that fails while the request is in flight is unknown.
fn outcome_from_sdk<E: ProvideErrorMetadata, R>(
    error: &SdkError<E, R>,
    status: Option<u16>,
) -> SendOutcome {
    match error {
        SdkError::ConstructionFailure(_) => SendOutcome::Rejected(ProviderFailure {
            kind: FailureKind::PERMANENT,
            code: None,
            reason: "request could not be built".to_string(),
        }),
        SdkError::ServiceError(service) => {
            let code = service.err().code().map(str::to_string);
            if status.is_some_and(|s| s >= 500) {
                return SendOutcome::Unknown {
                    reason: "AWS answered with a server error".to_string(),
                };
            }
            let throttled = code
                .as_deref()
                .is_some_and(|code| THROTTLING_CODES.contains(&code));
            SendOutcome::Rejected(ProviderFailure {
                kind: if throttled {
                    FailureKind::TRANSIENT
                } else {
                    FailureKind::PERMANENT
                },
                code,
                reason: "rejected by AWS".to_string(),
            })
        }
        _ => SendOutcome::Unknown {
            reason: "no answer from AWS".to_string(),
        },
    }
}

fn not_connected(reason: &str) -> AccountCheck {
    AccountCheck {
        checked_at: Some(Utc::now().to_rfc3339()),
        reason: Some(reason.to_string()),
        ..Default::default()
    }
}

pub struct SnsSender {
    client: aws_sdk_sns::Client,
    attributes: HashMap<String, MessageAttributeValue>,
}

impl SnsSender {
    pub async fn new(account: &Account, endpoints: &Endpoints) -> Result<Self> {
        let AccountSender::AWS_SNS {
            sender_id,
            origination_number,
            region,
        } = &account.sender
        else {
            return Err(anyhow!("not an SNS account"));
        };
        let attribute = |value: &str| {
            MessageAttributeValue::builder()
                .data_type("String")
                .string_value(value)
                .build()
                .map_err(|error| anyhow!("invalid SNS attribute: {error}"))
        };
        let mut attributes = HashMap::from([(
            "AWS.SNS.SMS.SMSType".to_string(),
            attribute("Transactional")?,
        )]);
        if let Some(sender_id) = sender_id {
            attributes.insert("AWS.SNS.SMS.SenderID".to_string(), attribute(sender_id)?);
        }
        if let Some(number) = origination_number {
            attributes.insert(
                "AWS.MM.SMS.OriginationNumber".to_string(),
                attribute(number)?,
            );
        }
        Ok(SnsSender {
            client: aws_sdk_sns::Client::new(
                &sdk_config(account, region.as_ref(), endpoints).await,
            ),
            attributes,
        })
    }
}

#[async_trait]
impl ChannelSender for SnsSender {
    fn capabilities(&self) -> ProviderCapabilities {
        MessagingProvider::AWS_SNS
            .capabilities(MessageChannel::SMS)
            .unwrap_or_else(|| unreachable!("SNS declares its own channel"))
    }

    async fn send(&self, message: &OutboundMessage) -> SendOutcome {
        let result = self
            .client
            .publish()
            .phone_number(message.destination.as_str())
            .message(&message.content.text)
            .set_message_attributes(Some(self.attributes.clone()))
            .send()
            .await;
        match result {
            Ok(output) => SendOutcome::Accepted {
                provider_message_id: output.message_id().map(str::to_string),
            },
            Err(error) => {
                let status = error.raw_response().map(|r| r.status().as_u16());
                outcome_from_sdk(&error, status)
            }
        }
    }

    async fn check(&self) -> AccountCheck {
        match self.client.get_sms_sandbox_account_status().send().await {
            Ok(status) => AccountCheck {
                connected: true,
                production_access: !status.is_in_sandbox(),
                approved_templates: Default::default(),
                checked_at: Some(Utc::now().to_rfc3339()),
                reason: status
                    .is_in_sandbox()
                    .then(|| "the account is in the SMS sandbox".to_string()),
            },
            Err(error) => not_connected(&format!(
                "SNS check failed{}",
                error
                    .as_service_error()
                    .and_then(|e| e.code())
                    .map(|code| format!(": {code}"))
                    .unwrap_or_default()
            )),
        }
    }
}

pub struct SesSender {
    client: aws_sdk_sesv2::Client,
    from: String,
}

impl SesSender {
    pub async fn new(account: &Account, endpoints: &Endpoints) -> Result<Self> {
        let AccountSender::AWS_SES {
            from_address,
            from_name,
            region,
            ..
        } = &account.sender
        else {
            return Err(anyhow!("not an SES account"));
        };
        Ok(SesSender {
            client: aws_sdk_sesv2::Client::new(
                &sdk_config(account, region.as_ref(), endpoints).await,
            ),
            from: match from_name {
                Some(name) => format!("{name} <{from_address}>"),
                None => from_address.clone(),
            },
        })
    }

    fn content(message: &OutboundMessage) -> Result<aws_sdk_sesv2::types::EmailContent> {
        use aws_sdk_sesv2::types::{Body, Content, EmailContent, Message};
        let text = |data: &str| {
            Content::builder()
                .data(data)
                .charset("UTF-8")
                .build()
                .map_err(|error| anyhow!("invalid email content: {error}"))
        };
        let mut body = Body::builder().text(text(&message.content.text)?);
        if let Some(html) = &message.content.html {
            body = body.html(text(html)?);
        }
        Ok(EmailContent::builder()
            .simple(
                Message::builder()
                    .subject(text(
                        message.content.subject.as_deref().unwrap_or_default(),
                    )?)
                    .body(body.build())
                    .build(),
            )
            .build())
    }
}

#[async_trait]
impl ChannelSender for SesSender {
    fn capabilities(&self) -> ProviderCapabilities {
        MessagingProvider::AWS_SES
            .capabilities(MessageChannel::EMAIL)
            .unwrap_or_else(|| unreachable!("SES declares its own channel"))
    }

    async fn send(&self, message: &OutboundMessage) -> SendOutcome {
        let content = match Self::content(message) {
            Ok(content) => content,
            Err(error) => {
                return SendOutcome::Rejected(ProviderFailure {
                    kind: FailureKind::PERMANENT,
                    code: None,
                    reason: error.to_string(),
                })
            }
        };
        let result = self
            .client
            .send_email()
            .from_email_address(&self.from)
            .destination(
                aws_sdk_sesv2::types::Destination::builder()
                    .to_addresses(message.destination.as_str())
                    .build(),
            )
            .content(content)
            .send()
            .await;
        match result {
            Ok(output) => SendOutcome::Accepted {
                provider_message_id: output.message_id().map(str::to_string),
            },
            Err(error) => {
                let status = error.raw_response().map(|r| r.status().as_u16());
                outcome_from_sdk(&error, status)
            }
        }
    }

    async fn check(&self) -> AccountCheck {
        match self.client.get_account().send().await {
            Ok(account) => AccountCheck {
                connected: account.sending_enabled(),
                production_access: account.production_access_enabled(),
                approved_templates: Default::default(),
                checked_at: Some(Utc::now().to_rfc3339()),
                reason: if !account.sending_enabled() {
                    Some("sending is disabled for this account".to_string())
                } else if !account.production_access_enabled() {
                    Some("the account is in the SES sandbox".to_string())
                } else {
                    None
                },
            },
            Err(error) => not_connected(&format!(
                "SES check failed{}",
                error
                    .as_service_error()
                    .and_then(|e| e.code())
                    .map(|code| format!(": {code}"))
                    .unwrap_or_default()
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::destination::Destination;
    use crate::test_server::{Reply, TestServer};
    use sequent_core::types::messaging::{
        AccountLimits, MessageAttemptState, MessageContent, MessagePurpose, OutOfWindowPolicy,
    };
    use serde_json::json;
    use std::collections::BTreeMap;

    fn account(sender: AccountSender) -> Account {
        Account {
            id: "aws-1".to_string(),
            channel: sender
                .provider()
                .channel()
                .expect("AWS providers have a channel"),
            sender,
            credentials: BTreeMap::from([
                (CredentialName::AWS_ACCESS_KEY_ID, "AKIDEXAMPLE".to_string()),
                (CredentialName::AWS_SECRET_ACCESS_KEY, "secret".to_string()),
            ]),
            limits: AccountLimits::default(),
            callback_url: None,
        }
    }

    fn message(channel: MessageChannel, to: &str) -> OutboundMessage {
        OutboundMessage {
            channel,
            purpose: MessagePurpose::NOTICE,
            destination: Destination::parse(channel, to).unwrap(),
            language: None,
            content: MessageContent {
                subject: Some("Enrollment".to_string()),
                text: "Your enrollment was approved.".to_string(),
                html: None,
                template_parameters: vec![],
                code: None,
            },
            provider_template: None,
            idempotency_key: "k".to_string(),
            expires_at: None,
            last_inbound_at: None,
            out_of_window: OutOfWindowPolicy::DISABLED,
        }
    }

    fn endpoints(server: &TestServer) -> Endpoints {
        Endpoints {
            aws: Some(server.base_url.clone()),
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn ses_sends_from_the_account_address() {
        let server = TestServer::start(vec![Reply::Json(200, json!({"MessageId": "ses-1"}))]).await;
        let sender = SesSender::new(
            &account(AccountSender::AWS_SES {
                from_address: "noreply@comelec.example".to_string(),
                from_name: Some("COMELEC".to_string()),
                region: Some("eu-west-1".to_string()),
                notification_topic_arn: None,
            }),
            &endpoints(&server),
        )
        .await
        .unwrap();
        assert_eq!(
            sender
                .send(&message(MessageChannel::EMAIL, "voter@example.org"))
                .await,
            SendOutcome::Accepted {
                provider_message_id: Some("ses-1".to_string())
            }
        );
        let body = server.requests()[0].json();
        assert_eq!(
            body["FromEmailAddress"],
            "COMELEC <noreply@comelec.example>"
        );
        assert_eq!(body["Destination"]["ToAddresses"][0], "voter@example.org");
    }

    #[tokio::test]
    async fn ses_throttling_is_transient_and_server_errors_unknown() {
        let server = TestServer::start(vec![
            Reply::Json(
                429,
                json!({"__type": "TooManyRequestsException", "message": "slow down"}),
            ),
            Reply::Json(500, json!({"__type": "InternalFailure"})),
        ])
        .await;
        let sender = SesSender::new(
            &account(AccountSender::AWS_SES {
                from_address: "noreply@comelec.example".to_string(),
                from_name: None,
                region: Some("eu-west-1".to_string()),
                notification_topic_arn: None,
            }),
            &endpoints(&server),
        )
        .await
        .unwrap();
        let email = message(MessageChannel::EMAIL, "voter@example.org");
        assert!(matches!(
            sender.send(&email).await,
            SendOutcome::Rejected(ProviderFailure {
                kind: FailureKind::TRANSIENT,
                ..
            })
        ));
        assert_eq!(
            sender.send(&email).await.state(),
            MessageAttemptState::UNKNOWN
        );
    }

    #[tokio::test]
    async fn sns_publishes_transactional_sms_with_the_sender_id() {
        let server = TestServer::start(vec![Reply::Raw(
            200,
            "text/xml",
            r#"<PublishResponse xmlns="http://sns.amazonaws.com/doc/2010-03-31/"><PublishResult><MessageId>sns-1</MessageId></PublishResult><ResponseMetadata><RequestId>r</RequestId></ResponseMetadata></PublishResponse>"#.to_string(),
        )])
        .await;
        let sender = SnsSender::new(
            &account(AccountSender::AWS_SNS {
                sender_id: Some("COMELEC".to_string()),
                origination_number: None,
                region: Some("eu-west-1".to_string()),
            }),
            &endpoints(&server),
        )
        .await
        .unwrap();
        assert_eq!(
            sender
                .send(&message(MessageChannel::SMS, "+639171234567"))
                .await,
            SendOutcome::Accepted {
                provider_message_id: Some("sns-1".to_string())
            }
        );
        let body = urlencoding::decode(&server.requests()[0].body)
            .unwrap()
            .to_string();
        assert!(body.contains("PhoneNumber=+639171234567"));
        assert!(body.contains("COMELEC"));
        assert!(body.contains("Transactional"));
    }
}
