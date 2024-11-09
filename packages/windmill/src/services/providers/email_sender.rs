// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::types::error::Result;
use crate::util::aws::get_from_env_aws_config;

use anyhow::anyhow;
use aws_sdk_sesv2::types::{EmailContent, RawMessage};
use aws_sdk_sesv2::Client as AwsSesClient;
use aws_smithy_types::Blob;
use lettre::message::header::{ContentDisposition, ContentType};
use lettre::message::{MultiPart, SinglePart};
use lettre::Message;
use tracing::{event, instrument, Level};

pub struct Attachment {
    pub filename: String,
    pub mimetype: String,
    pub content: Vec<u8>,
}

pub enum EmailTransport {
    AwsSes(AwsSesClient),
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

        Ok(EmailSender {
            transport: match email_transport_name.as_str() {
                "AwsSes" => {
                    let shared_config = get_from_env_aws_config().await?;
                    EmailTransport::AwsSes(AwsSesClient::new(&shared_config))
                }
                _ => EmailTransport::Console,
            },
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
        let alternative = match html_body {
            Some(ref html_body) => {
                let html_part = SinglePart::builder()
                    .header(ContentType::TEXT_HTML)
                    .body(html_body.clone());

                MultiPart::alternative()
                    .singlepart(plaintext_part)
                    .singlepart(html_part)
            }
            None => MultiPart::alternative().singlepart(plaintext_part),
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
                    html_body=html_body.unwrap_or_default(),
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
            EmailTransport::Console => {
                event!(
                    Level::INFO,
                    "EmailTransport::AwsSes: Sending email:\n\t - receivers={receivers:?}\n\t - subject={subject}\n\t - plaintext_body={plaintext_body:.255}\n\t - html_body={html_body:.255}",
                    html_body=html_body.unwrap_or_default(),
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
