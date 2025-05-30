// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! SMS Sender service with support for cross-account AWS SNS
//!
//! ## Environment Variables
//!
//! ### Required:
//! - `SMS_TRANSPORT_NAME`: Transport type ("AwsSns" or "Console")
//! - `AWS_SNS_ATTRIBUTES`: JSON string with SNS message attributes
//! - `AWS_REGION`: AWS region for SMS sending
//!
//! ### Cross-Account Support (Optional):
//! - `AWS_SNS_CROSS_ACCOUNT_ROLE_ARN`: ARN of the role to assume in the target account
//!   Example: "arn:aws:iam::123456789012:role/CrossAccountSMSRole"
//! - `AWS_SNS_CROSS_ACCOUNT_EXTERNAL_ID`: External ID for additional security (recommended)
//!
//! ## Cross-Account Setup
//!
//! To send SMS from Account A using SMS configuration in Account B:
//!
//! 1. In Account B (SMS config account), create an IAM role with SNS permissions:
//!    ```json
//!    {
//!      "Version": "2012-10-17",
//!      "Statement": [
//!        {
//!          "Effect": "Allow",
//!          "Action": [
//!            "sns:Publish",
//!            "sns:GetSMSAttributes",
//!            "sns:SetSMSAttributes"
//!          ],
//!          "Resource": "*"
//!        }
//!      ]
//!    }
//!    ```
//!
//! 2. Configure the role's trust policy to allow Account A to assume it:
//!    ```json
//!    {
//!      "Version": "2012-10-17",
//!      "Statement": [
//!        {
//!          "Effect": "Allow",
//!          "Principal": {
//!            "AWS": "arn:aws:iam::ACCOUNT-A-ID:role/YourApplicationRole"
//!          },
//!          "Action": "sts:AssumeRole",
//!          "Condition": {
//!            "StringEquals": {
//!              "sts:ExternalId": "your-unique-external-id"
//!            }
//!          }
//!        }
//!      ]
//!    }
//!    ```
//!
//! 3. In Account A, ensure the application role has permission to assume the cross-account role:
//!    ```json
//!    {
//!      "Version": "2012-10-17",
//!      "Statement": [
//!        {
//!          "Effect": "Allow",
//!          "Action": "sts:AssumeRole",
//!          "Resource": "arn:aws:iam::ACCOUNT-B-ID:role/CrossAccountSMSRole"
//!        }
//!      ]
//!    }
//!    ```

use crate::types::error::Result;

use anyhow::anyhow;
use aws_sdk_sns::{types::MessageAttributeValue, Client as AwsSnsClient};
use aws_sdk_sts::Client as StsClient;
use aws_config::{Region, SdkConfig};
use sequent_core::serialization::deserialize_with_path::*;
use sequent_core::util::aws::get_from_env_aws_config;
use std::collections::HashMap;
use tracing::{event, instrument, Level};

type MessageAttributes = Option<HashMap<String, MessageAttributeValue>>;

pub enum SmsTransport {
    AwsSns((AwsSnsClient, MessageAttributes)),
    Console,
}

pub struct SmsSender {
    pub transport: SmsTransport,
}

impl SmsSender {
    /// Creates AWS config for cross-account access by assuming a role if configured
    #[instrument(err)]
    async fn get_cross_account_aws_config() -> Result<SdkConfig> {
        let base_config = get_from_env_aws_config().await?;

        // Check if cross-account role assumption is configured
        if let Ok(cross_account_role_arn) = std::env::var("AWS_SNS_CROSS_ACCOUNT_ROLE_ARN") {
            if !cross_account_role_arn.is_empty() {
                event!(
                    Level::INFO,
                    "Assuming cross-account role for SMS: {cross_account_role_arn}"
                );

                let sts_client = StsClient::new(&base_config);

                // Generate a unique session name
                let session_name = format!("sms-cross-account-{}",
                    std::process::id());

                // Get external ID if provided (recommended for security)
                let external_id = std::env::var("AWS_SNS_CROSS_ACCOUNT_EXTERNAL_ID").ok();

                let mut assume_role_request = sts_client
                    .assume_role()
                    .role_arn(&cross_account_role_arn)
                    .role_session_name(&session_name);

                if let Some(ext_id) = external_id {
                    if !ext_id.is_empty() {
                        assume_role_request = assume_role_request.external_id(ext_id);
                    }
                }

                let assume_role_output = assume_role_request
                    .send()
                    .await
                    .map_err(|err| anyhow!("Failed to assume cross-account role: {err:?}"))?;

                let credentials = assume_role_output
                    .credentials()
                    .ok_or_else(|| anyhow!("No credentials returned from assume role"))?;

                // Create new config with assumed role credentials
                let region = Region::new(
                    std::env::var("AWS_REGION")
                        .map_err(|err| anyhow!("AWS_REGION env var missing: {err}"))?,
                );

                let credentials_provider = aws_sdk_sns::config::Credentials::new(
                    credentials.access_key_id(),
                    credentials.secret_access_key(),
                    credentials.session_token().map(|s| s.to_string()),
                    None,
                    "cross-account-assumed-role",
                );

                return Ok(aws_config::SdkConfig::builder()
                    .region(region)
                    .credentials_provider(credentials_provider)
                    .build());
            }
        }

        // Return base config if no cross-account role is configured
        Ok(base_config)
    }

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
                    let shared_config = Self::get_cross_account_aws_config().await?;
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

    #[instrument(skip(self, message), err)]
    pub async fn send(&self, receiver: String, message: String) -> Result<()> {
        match self.transport {
            SmsTransport::AwsSns((ref aws_client, ref messsage_attributes)) => {
                let cross_account_info = if std::env::var("AWS_SNS_CROSS_ACCOUNT_ROLE_ARN").is_ok() {
                    " (cross-account)"
                } else {
                    ""
                };

                event!(
                    Level::INFO,
                    "SmsTransport::AwsSns: Sending SMS{cross_account_info}:\n\t - receiver={receiver}\n\t - message={message:.255}",
                );
                aws_client
                    .publish()
                    .set_message_attributes(messsage_attributes.clone())
                    .set_phone_number(Some(receiver))
                    .set_message(Some(message))
                    .send()
                    .await
                    .map_err(|err| anyhow!("SmsTransport::AwsSns send error: {err:?}"))?;
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
