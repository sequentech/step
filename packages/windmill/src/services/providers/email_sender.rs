// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
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

/// Why the environment transport didn't hand a message over, typed so the
/// message ledger can tell a definite refusal (FAILED, retried when
/// transient) from a lost answer (UNKNOWN, never sent again). Reasons are
/// fixed strings: never a provider body, an address or the content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportFailure {
    /// Not delivered: refused before the message left Step, or refused by
    /// the provider. `transient`: sending it again later may pass.
    Refused { transient: bool, reason: String },
    /// The provider may have taken it: its answer was lost (a timeout, a
    /// dropped connection, an unreadable answer).
    Unknown { reason: String },
}

impl std::fmt::Display for TransportFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransportFailure::Refused { reason, .. } | TransportFailure::Unknown { reason } => {
                f.write_str(reason)
            }
        }
    }
}

impl TransportFailure {
    fn permanent(reason: &str) -> Self {
        TransportFailure::Refused {
            transient: false,
            reason: reason.to_string(),
        }
    }

    fn lost() -> Self {
        TransportFailure::Unknown {
            reason: "the provider's answer was lost".to_string(),
        }
    }

    /// A provider's error answer: throttling (429 or a throttling code) and
    /// 5xx may pass later; other 4xx are final.
    pub fn from_http_status(status: u16, code: Option<&str>) -> Self {
        let throttled = status == 429
            || code.is_some_and(|code| {
                let code = code.to_ascii_lowercase();
                code.contains("throttl") || code.contains("toomanyrequests")
            });
        if throttled {
            return TransportFailure::Refused {
                transient: true,
                reason: "rate limited by the provider".to_string(),
            };
        }
        TransportFailure::Refused {
            transient: status >= 500,
            reason: format!("refused by the provider (HTTP {status})"),
        }
    }

    /// An AWS SDK error: a request that couldn't be built never left Step;
    /// an error answer is a refusal; anything else (dispatch, timeout, an
    /// unreadable answer) may have reached AWS, so its outcome is unknown.
    pub fn from_aws<E: aws_sdk_sesv2::error::ProvideErrorMetadata>(
        error: &aws_sdk_sesv2::error::SdkError<E, aws_sdk_sesv2::config::http::HttpResponse>,
    ) -> Self {
        use aws_sdk_sesv2::error::SdkError;
        match error {
            SdkError::ConstructionFailure(_) => {
                Self::permanent("the request to the provider couldn't be built")
            }
            SdkError::ServiceError(service) => {
                Self::from_http_status(service.raw().status().as_u16(), service.err().code())
            }
            _ => Self::lost(),
        }
    }

    /// An SMTP error: a 4xx or 5xx reply is a refusal (4xx may pass later);
    /// a TLS handshake failure happens before the mail transaction; any
    /// other error (connection, timeout, an unreadable reply) may have
    /// come after the server took the message.
    pub fn from_smtp(error: &lettre::transport::smtp::Error) -> Self {
        let code = error
            .status()
            .map(|code| code.to_string())
            .unwrap_or_default();
        if error.is_permanent() {
            TransportFailure::Refused {
                transient: false,
                reason: format!("refused by the mail server ({code})"),
            }
        } else if error.is_transient() {
            TransportFailure::Refused {
                transient: true,
                reason: format!("deferred by the mail server ({code})"),
            }
        } else if error.is_tls() {
            TransportFailure::Refused {
                transient: true,
                reason: "the mail server's TLS handshake failed".to_string(),
            }
        } else {
            Self::lost()
        }
    }
}

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

#[cfg(test)]
mod failure_tests {
    use super::*;
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;

    #[test]
    fn provider_answers_are_refusals_and_only_throttling_and_5xx_may_pass() {
        let refused = |transient| move |failure| matches!(failure, TransportFailure::Refused { transient: t, .. } if t == transient);
        assert!(refused(true)(TransportFailure::from_http_status(429, None)));
        assert!(refused(true)(TransportFailure::from_http_status(
            400,
            Some("Throttling")
        )));
        assert!(refused(true)(TransportFailure::from_http_status(503, None)));
        assert!(refused(false)(TransportFailure::from_http_status(
            400,
            Some("InvalidParameter")
        )));
        assert!(refused(false)(TransportFailure::from_http_status(
            403, None
        )));
    }

    #[tokio::test]
    async fn an_invalid_address_is_refused_before_anything_is_sent() {
        let sender = EmailSender::from_transport_name("Console", "test@example.invalid".into())
            .await
            .unwrap();
        let failure = sender
            .send_message(
                vec!["not an address".into()],
                "s".into(),
                "b".into(),
                None,
                vec![],
            )
            .await
            .unwrap_err();
        assert_eq!(
            failure,
            TransportFailure::Refused {
                transient: false,
                reason: "the email address isn't valid".to_string()
            }
        );
    }

    /// A one-connection SMTP server that answers `MAIL FROM` with `reply`,
    /// or, with `None`, accepts the message and drops the connection before
    /// answering its end.
    fn smtp_server(reply: Option<&'static str>) -> u16 {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut writer = stream.try_clone().unwrap();
            let mut reader = BufReader::new(stream);
            writer.write_all(b"220 test ESMTP\r\n").unwrap();
            let mut in_data = false;
            let mut line = String::new();
            loop {
                line.clear();
                if reader.read_line(&mut line).unwrap_or(0) == 0 {
                    return;
                }
                let command = line.trim_end().to_ascii_uppercase();
                if in_data {
                    if command == "." {
                        return; // the answer to the message is lost
                    }
                    continue;
                }
                let answer = if command.starts_with("EHLO") {
                    "250 test\r\n".to_string()
                } else if command.starts_with("MAIL FROM") {
                    match reply {
                        Some(reply) => format!("{reply}\r\n"),
                        None => "250 ok\r\n".to_string(),
                    }
                } else if command.starts_with("RCPT TO") {
                    "250 ok\r\n".to_string()
                } else if command == "DATA" {
                    in_data = true;
                    "354 go on\r\n".to_string()
                } else {
                    "221 bye\r\n".to_string()
                };
                if writer.write_all(answer.as_bytes()).is_err() {
                    return;
                }
            }
        });
        port
    }

    async fn send_through(port: u16) -> std::result::Result<(), TransportFailure> {
        let sender = EmailSender {
            transport: EmailTransport::Smtp(
                SmtpTransport::builder_dangerous("127.0.0.1")
                    .port(port)
                    .timeout(Some(std::time::Duration::from_secs(5)))
                    .build(),
            ),
            email_from: "from@example.org".to_string(),
        };
        sender
            .send_message(
                vec!["to@example.org".into()],
                "s".into(),
                "b".into(),
                None,
                vec![],
            )
            .await
    }

    #[tokio::test]
    async fn smtp_refusals_are_failures_and_a_lost_answer_is_unknown() {
        assert_eq!(
            send_through(smtp_server(Some("550 5.7.1 rejected"))).await,
            Err(TransportFailure::Refused {
                transient: false,
                reason: "refused by the mail server (550)".to_string()
            })
        );
        assert_eq!(
            send_through(smtp_server(Some("451 4.3.0 try later"))).await,
            Err(TransportFailure::Refused {
                transient: true,
                reason: "deferred by the mail server (451)".to_string()
            })
        );
        assert!(matches!(
            send_through(smtp_server(None)).await,
            Err(TransportFailure::Unknown { .. })
        ));
    }
}

#[cfg(test)]
mod confidentiality_tests {
    use super::*;
    #[tokio::test]
    async fn explicit_console_accepts_test_messages_but_typos_fail() {
        let subscriber = tracing_subscriber::fmt()
            .with_ansi(false)
            .with_test_writer()
            .finish();
        let _guard = tracing::subscriber::set_default(subscriber);
        let sender = EmailSender::from_transport_name("Console", "test@example.invalid".into())
            .await
            .unwrap();
        let body = format!("{}synthetic-secret-at-end", "x".repeat(300));
        sender
            .send(
                vec!["recipient@example.invalid".into()],
                "Test".into(),
                body.clone(),
                Some(body),
                vec![],
            )
            .await
            .unwrap();
        assert!(
            EmailSender::from_transport_name("AwsSES", "test@example.invalid".into())
                .await
                .is_err()
        );
    }
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

        Self::from_transport_name(&email_transport_name, email_from).await
    }

    async fn from_transport_name(email_transport_name: &str, email_from: String) -> Result<Self> {
        let transport = match email_transport_name {
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
            "Console" => EmailTransport::Console,
            _ => {
                return Err(anyhow!(
                    "Unsupported EMAIL_TRANSPORT_NAME; expected AwsSes, smtp or Console"
                )
                .into())
            }
        };

        Ok(EmailSender {
            transport,
            email_from,
        })
    }

    #[instrument(skip_all, err)]
    pub async fn send(
        &self,
        receivers: Vec<String>,
        subject: String,
        plaintext_body: String,
        html_body: Option<String>,
        attachments: Vec<Attachment>,
    ) -> Result<()> {
        self.send_message(receivers, subject, plaintext_body, html_body, attachments)
            .await
            .map_err(|failure| anyhow!("error sending email: {failure}").into())
    }

    /// [`EmailSender::send`] with a typed failure, for the message ledger.
    pub async fn send_message(
        &self,
        receivers: Vec<String>,
        subject: String,
        plaintext_body: String,
        html_body: Option<String>,
        attachments: Vec<Attachment>,
    ) -> std::result::Result<(), TransportFailure> {
        // Build the email message using lettre
        let mut email_builder = Message::builder()
            .from(self.email_from.parse().map_err(|_| {
                TransportFailure::permanent("the configured sender address isn't valid")
            })?)
            .subject(subject.clone());

        for receiver in &receivers {
            email_builder = email_builder.to(receiver
                .parse()
                .map_err(|_| TransportFailure::permanent("the email address isn't valid"))?);
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
                let content_type =
                    ContentType::from(attachment.mimetype.parse().map_err(|_| {
                        TransportFailure::permanent("an attachment's type isn't valid")
                    })?);
                let attachment_part = SinglePart::builder()
                    .header(content_type)
                    .header(ContentDisposition::attachment(&attachment.filename))
                    .body(attachment.content.clone());
                mixed = mixed.singlepart(attachment_part);
            }
            email_builder
                .multipart(mixed)
                .map_err(|_| TransportFailure::permanent("the email couldn't be built"))?
        } else {
            email_builder
                .multipart(alternative)
                .map_err(|_| TransportFailure::permanent("the email couldn't be built"))?
        };

        match self.transport {
            EmailTransport::AwsSes(ref aws_client) => {
                event!(Level::INFO, "EmailTransport::AwsSes: Sending email");
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
                                    .map_err(|_| {
                                        TransportFailure::permanent("the email couldn't be built")
                                    })?,
                            )
                            .build(),
                    )
                    .send()
                    .await
                    .map_err(|err| {
                        event!(Level::WARN, "EmailTransport::AwsSes: send failed: {err:?}");
                        TransportFailure::from_aws(&err)
                    })?;
            }
            EmailTransport::Smtp(ref smtp_transport) => {
                event!(Level::INFO, "EmailTransport::Smtp: Sending email");
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
                smtp_transport.send(&email_message).map_err(|err| {
                    event!(Level::WARN, "EmailTransport::Smtp: send failed: {err:?}");
                    TransportFailure::from_smtp(&err)
                })?;
                event!(Level::INFO, "Email sent successfully via SMTP");
            }
            EmailTransport::Console => {
                // The console transport does not deliver anything, so printing the rendered
                // message is its only effect. Keep it verbose: it is how a developer retrieves
                // an enrollment or login link locally.
                event!(
                    Level::INFO,
                    "EmailTransport::Console: Sending email:\n\t - receivers={receivers:?}\n\t - subject={subject}\n\t - plaintext_body={plaintext_body}\n\t - html_body={html_body}",
                    html_body = html_body.clone().unwrap_or_default(),
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
