// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::types::error::Result;

use anyhow::anyhow;
use aws_sdk_sesv2::types::{EmailContent, RawMessage};
use aws_sdk_sesv2::Client as AwsSesClient;
use aws_smithy_types::Blob;
use lettre::message::header::{ContentDisposition, ContentType};
use lettre::message::{MultiPart, SinglePart};
use lettre::Message;
use tracing::{event, instrument, Level};

// Import required for SMTP transport
use config::{Config, File, FileFormat};
use lettre::transport::smtp::SmtpTransport;
use lettre::transport::smtp::SmtpTransportBuilder;
use lettre::Transport;
use sequent_core::util::aws::get_from_env_aws_config;
use serde::Deserialize;

pub struct Attachment {
    pub filename: String,
    pub mimetype: String,
    pub content: Vec<u8>,
}

#[derive(Debug, Deserialize)]
struct SmtpConfig {
    server_url: String,
    timeout: Option<u64>, // in seconds
}

pub enum EmailTransport {
    AwsSes(AwsSesClient),
    Smtp(SmtpTransport),
    Console,
}

pub struct EmailSender {
    transport: EmailTransport,
    email_from: String,
}

impl EmailSender {
    #[instrument(err)]
    pub async fn new() -> Result<Self> {
        let email_from = std::env::var("EMAIL_FROM")
            .map_err(|err| anyhow!("EMAIL_FROM env var missing: {err:?}"))?;
        let email_transport_name = std::env::var("EMAIL_TRANSPORT_NAME")
            .map_err(|err| anyhow!("EMAIL_TRANSPORT_NAME env var missing: {err:?}"))?;

        event!(
            Level::INFO,
            "EmailTransport: from_address={email_from}, email_transport_name={email_transport_name}"
        );

        let transport = match email_transport_name.as_str() {
            "AwsSes" => {
                let shared_config = get_from_env_aws_config().await?;
                EmailTransport::AwsSes(AwsSesClient::new(&shared_config))
            }
            "smtp" => {
                let smtp_config_str = std::env::var("EMAIL_TRANSPORT_CONFIG")
                    .map_err(|err| anyhow!("EMAIL_TRANSPORT_CONFIG env var missing: {err:?}"))?;

                let smtp_config: SmtpConfig = Config::builder()
                    .add_source(File::from_str(&smtp_config_str, FileFormat::Json))
                    .build()
                    .map_err(|err| anyhow!("Error parsing SMTP config: {err:?}"))?
                    .try_deserialize()
                    .map_err(|err| anyhow!("Error deserializing SMTP config: {err:?}"))?;

                // Build the SMTP transport using the smtp_config
                let mut builder = SmtpTransport::from_url(smtp_config.server_url.as_str())
                    .map_err(|err| {
                        anyhow!("Error creating SMTP transport builder from url: {err:?}")
                    })?;

                if let Some(timeout) = smtp_config.timeout {
                    builder = builder.timeout(Some(std::time::Duration::from_secs(timeout)));
                }

                let smtp_transport = builder.build();

                EmailTransport::Smtp(smtp_transport)
            }
            _ => EmailTransport::Console,
        };

        Ok(EmailSender {
            transport,
            email_from,
        })
    }

    #[instrument(skip(self, plaintext_body, html_body, attachments), err)]
    pub async fn send(
        &self,
        receivers: Vec<String>,
        subject: String,
        plaintext_body: String,
        html_body: Option<String>,
        attachments: Vec<Attachment>,
    ) -> Result<()> {
        // Build the email message using lettre
        let mut email_builder = Message::builder()
            .from(
                self.email_from
                    .parse()
                    .map_err(|err| anyhow!("invalid email_from: {:?}", err))?,
            )
            .subject(subject.clone());

        for receiver in &receivers {
            email_builder = email_builder.to(receiver
                .parse()
                .map_err(|err| anyhow!("invalid receiver: {:?}", err))?);
        }

        // Build the plaintext and HTML body parts
        let plaintext_part = SinglePart::builder()
            .header(ContentType::TEXT_PLAIN)
            .body(plaintext_body.clone());

        // Create the alternative part
        let alternative = if let Some(ref html_body) = html_body {
            let html_part = SinglePart::builder()
                .header(ContentType::TEXT_HTML)
                .body(html_body.clone());

            MultiPart::alternative()
                .singlepart(plaintext_part)
                .singlepart(html_part)
        } else {
            MultiPart::alternative().singlepart(plaintext_part)
        };

        // If there are attachments, create a mixed multipart
        let email_message = if !attachments.is_empty() {
            let mut mixed = MultiPart::mixed().multipart(alternative);

            for attachment in &attachments {
                let content_type = ContentType::from(
                    attachment
                        .mimetype
                        .parse()
                        .map_err(|err| anyhow!("invalid mimetype: {:?}", err))?,
                );
                let attachment_part = SinglePart::builder()
                    .header(content_type)
                    .header(ContentDisposition::attachment(&attachment.filename))
                    .body(attachment.content.clone());
                mixed = mixed.singlepart(attachment_part);
            }
            email_builder
                .multipart(mixed)
                .map_err(|err| anyhow!("{:?}", err))?
        } else {
            email_builder
                .multipart(alternative)
                .map_err(|err| anyhow!("{:?}", err))?
        };

        match self.transport {
            EmailTransport::AwsSes(ref aws_client) => {
                event!(
                    Level::INFO,
                    "EmailTransport::AwsSes: Sending email:\n\t - receivers={receivers:?}\n\t - subject={subject}\n\t - plaintext_body={plaintext_body:.255}\n\t - html_body={html_body:.255}",
                    html_body=html_body.clone().unwrap_or_default(),
                );
                if !attachments.is_empty() {
                    for attachment in &attachments {
                        event!(
                            Level::INFO,
                            "Attachment: name={filename}, mimetype={mimetype}",
                            filename = attachment.filename,
                            mimetype = attachment.mimetype
                        );
                    }
                }
                // Serialize the email message to bytes
                let email_bytes = email_message.formatted();

                // Send the email as a raw email
                aws_client
                    .send_email()
                    .from_email_address(self.email_from.as_str())
                    .content(
                        EmailContent::builder()
                            .raw(
                                RawMessage::builder()
                                    .data(Blob::new(email_bytes))
                                    .build()
                                    .map_err(|err| {
                                        anyhow!("error building raw message: {err:?}")
                                    })?,
                            )
                            .build(),
                    )
                    .send()
                    .await
                    .map_err(|err| anyhow!("error sending email: {err:?}"))?;
            }
            EmailTransport::Smtp(ref smtp_transport) => {
                event!(
                    Level::INFO,
                    "EmailTransport::Smtp: Sending email:\n\t - receivers={receivers:?}\n\t - subject={subject}",
                );
                if !attachments.is_empty() {
                    for attachment in &attachments {
                        event!(
                            Level::INFO,
                            "Attachment: name={filename}, mimetype={mimetype}",
                            filename = attachment.filename,
                            mimetype = attachment.mimetype
                        );
                    }
                }
                // Send the email via SMTP
                smtp_transport
                    .send(&email_message)
                    .map_err(|err| anyhow!("Error sending email via SMTP: {err:?}"))?;
                event!(Level::INFO, "Email sent successfully via SMTP");
            }
            EmailTransport::Console => {
                event!(
                    Level::INFO,
                    "EmailTransport::Console: Sending email:\n\t - receivers={receivers:?}\n\t - subject={subject}\n\t - plaintext_body={plaintext_body:.255}\n\t - html_body={html_body:.255}",
                    html_body=html_body.clone().unwrap_or_default(),
                );
                if !attachments.is_empty() {
                    for attachment in &attachments {
                        event!(
                            Level::INFO,
                            "Attachment: name={filename}, mimetype={mimetype}",
                            filename = attachment.filename,
                            mimetype = attachment.mimetype
                        );
                    }
                }
            }
        }
        Ok(())
    }
}
