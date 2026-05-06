// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
//! SMS delivery via AWS SNS or a console logger.
use crate::types::error::Result;

use anyhow::anyhow;
use aws_sdk_sns::{types::MessageAttributeValue, Client as AwsSnsClient};
use sequent_core::serialization::deserialize_with_path::*;
use sequent_core::util::aws::get_from_env_aws_config;
use std::collections::HashMap;
use tracing::{event, instrument, Level};

type MessageAttributes = Option<HashMap<String, MessageAttributeValue>>;

/// Transport backend selected for SMS delivery.
pub enum SmsTransport {
    /// AWS SNS client plus optional per-message attributes.
    AwsSns((AwsSnsClient, MessageAttributes)),
    /// Log-only transport (no delivery).
    Console,
}

/// SMS sender configured from environment variables.
pub struct SmsSender {
    /// Active SMS transport.
    pub transport: SmsTransport,
}

impl SmsSender {
    /// Builds an [`SmsSender`] from environment configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if env/config parsing fails.
    #[instrument(err)]
    pub async fn new() -> Result<Self> {
        let sms_transport_name = std::env::var("SMS_TRANSPORT_NAME")
            .map_err(|_err| anyhow!("SMS_TRANSPORT_NAME env var missing"))?;

        event!(
            Level::INFO,
            "SmsTransport: sms_transport_name={sms_transport_name}"
        );
        Ok(SmsSender {
            transport: match sms_transport_name.as_str() {
                "AwsSns" => {
                    let shared_config = get_from_env_aws_config().await?;
                    let client = AwsSnsClient::new(&shared_config);

                    let base_message_attributes: HashMap<String, String> = deserialize_str(
                        &std::env::var("AWS_SNS_ATTRIBUTES")
                            .map_err(|err| anyhow!("AWS_SNS_ATTRIBUTES env var missing"))?,
                    )
                    .map_err(|err| anyhow!("AWS_SNS_ATTRIBUTES env var parse error: {err:?}"))?;
                    let messsage_attributes = Some(
                        base_message_attributes
                            .into_iter()
                            .map(|(key, value)| {
                                Ok((
                                    key,
                                    MessageAttributeValue::builder()
                                        .set_data_type(Some("String".to_string()))
                                        .set_string_value(Some(value))
                                        .build()
                                        .map_err(|err| {
                                            anyhow!("Error building Message Attribute: {err:?}")
                                        })?,
                                ))
                            })
                            .collect::<Result<HashMap<String, MessageAttributeValue>>>()?,
                    );
                    SmsTransport::AwsSns((client, messsage_attributes))
                }
                _ => SmsTransport::Console,
            },
        })
    }

    /// Sends a single SMS message.
    ///
    /// # Errors
    ///
    /// Returns an error if delivery fails.
    #[instrument(skip(self, message), err)]
    pub async fn send(&self, receiver: String, message: String) -> Result<()> {
        match self.transport {
            SmsTransport::AwsSns((ref aws_client, ref messsage_attributes)) => {
                event!(
                    Level::INFO,
                    "SmsTransport::AwsSes: Sending SMS:\n\t - receiver={receiver}\n\t - message={message:.255}",
                );
                aws_client
                    .publish()
                    .set_message_attributes(messsage_attributes.clone())
                    .set_phone_number(Some(receiver))
                    .set_message(Some(message))
                    .send()
                    .await
                    .map_err(|err| anyhow!("SmsTransport::AwsSes send error: {err:?}"))?;
            }
            SmsTransport::Console => {
                event!(
                    Level::INFO,
                    "SmsTransport::Console: Sending SMS:\n\t - receiver={receiver}\n\t - message={message}",
                );
            }
        }

        Ok(())
    }
}
