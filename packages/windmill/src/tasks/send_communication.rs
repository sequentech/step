// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::hasura::area::get_election_event_areas;
use crate::hasura::election_event::get_election_event;
use crate::hasura::tally_session::{get_tally_session_highest_batch, insert_tally_session};
use crate::hasura::tally_session_contest::insert_tally_session_contest;
use crate::hasura::trustee::get_trustees_by_id;
use crate::services::celery_app::get_celery_app;
use crate::services::election_event_statistics::{
    get_election_event_statistics, update_election_event_statistics,
};
use crate::services::users::list_users;
use crate::tasks::insert_ballots::{insert_ballots, InsertBallotsPayload};
use crate::tasks::send_communication::get_election_event::GetElectionEventSequentBackendElectionEvent;
use crate::types::error::Result;

use crate::services::database::{get_database_pool, PgConfig};
use deadpool_postgres::Client as DbClient;
use sequent_core::ballot::ElectionEventStatistics;

use anyhow::{anyhow, Context};
use aws_config::{meta::region::RegionProviderChain, Region};
use aws_sdk_sesv2::types::{Body, Content, Destination, EmailContent, Message as AwsMessage};
use aws_sdk_sesv2::{Client as AwsSesClient, Error as AwsSesError};
use aws_sdk_sns::{types::MessageAttributeValue, Client as AwsSnsClient, Error as AwsSnsError};
use lettre::message::{header::ContentType, MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};

use board_messages::braid::newtypes::BatchNumber;
use celery::error::TaskError;
use sequent_core::services::keycloak::{get_event_realm, get_tenant_realm, KeycloakAdminClient};
use sequent_core::services::{keycloak, pdf, reports};
use sequent_core::types::ceremonies::*;
use sequent_core::types::keycloak::User;
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};
use std::default::Default;
use strum_macros::{Display, EnumString};
use tracing::{event, instrument, Level};

#[allow(non_camel_case_types)]
#[derive(Display, Serialize, Deserialize, Debug, PartialEq, Eq, Clone, EnumString)]
pub enum AudienceSelection {
    #[strum(serialize = "ALL_USERS")]
    ALL_USERS,
    #[strum(serialize = "NOT_VOTED")]
    NOT_VOTED,
    #[strum(serialize = "VOTED")]
    VOTED,
    #[strum(serialize = "SELECTED")]
    SELECTED,
}

#[allow(non_camel_case_types)]
#[derive(Display, Serialize, Deserialize, Debug, PartialEq, Eq, Clone, EnumString)]
enum CommunicationType {
    #[strum(serialize = "CREDENTIALS")]
    CREDENTIALS,
    #[strum(serialize = "RECEIPT")]
    RECEIPT,
}

#[allow(non_camel_case_types)]
#[derive(Display, Serialize, Deserialize, Debug, PartialEq, Eq, Clone, EnumString)]
enum CommunicationMethod {
    #[strum(serialize = "EMAIL")]
    EMAIL,
    #[strum(serialize = "SMS")]
    SMS,
}

#[derive(Deserialize, Debug, Serialize, Clone)]
pub struct EmailConfig {
    subject: String,
    plaintext_body: String,
    html_body: String,
}

#[derive(Deserialize, Debug, Serialize, Clone)]
pub struct SmsConfig {
    message: String,
}

#[derive(Deserialize, Debug, Serialize, Clone)]
pub struct SendCommunicationBody {
    audience_selection: AudienceSelection,
    audience_voter_ids: Option<Vec<String>>,
    communication_type: CommunicationType,
    communication_method: CommunicationMethod,
    schedule_now: bool,
    schedule_date: Option<String>,
    email: Option<EmailConfig>,
    sms: Option<SmsConfig>,
}

fn get_variables(
    user: &User,
    election_event: Option<GetElectionEventSequentBackendElectionEvent>,
    tenant_id: String,
) -> Result<Map<String, Value>> {
    let mut variables: Map<String, Value> = Default::default();
    variables.insert(
        "user".to_string(),
        json!({
            "first_name": user.first_name.clone(),
            "last_name": user.last_name.clone(),
            "username": user.username.clone(),
            "first_name": user.first_name.clone(),
        }),
    );
    variables.insert("tenant_id".to_string(), json!(tenant_id.clone()));
    if let Some(ref election_event) = election_event {
        variables.insert(
            "election_event".to_string(),
            json!({
                "id": election_event.id.clone(),
                "name": election_event.name.clone(),
            }),
        );
        variables.insert(
            "vote_url".to_string(),
            json!(format!(
                "{base_url}/tenant/{tenant_id}/event/{event_id}/login",
                base_url = std::env::var("VOTING_PORTAL_URL")
                    .map_err(|err| anyhow!("VOTING_PORTAL_URL env var missing"))?,
                tenant_id = tenant_id,
                event_id = election_event.id,
            )),
        );
    }
    Ok(variables)
}

type MessageAttributes = Option<HashMap<String, MessageAttributeValue>>;

enum SmsTransport {
    AwsSns((AwsSnsClient, MessageAttributes)),
    Console,
}

struct SmsSender {
    transport: SmsTransport,
}

pub async fn get_aws_config() -> Result<aws_config::SdkConfig> {
    let region_provider = RegionProviderChain::first_try(Region::new(
        std::env::var("AWS_REGION").map_err(|err| anyhow!("AWS_REGION env var missing"))?,
    ))
    .or_default_provider()
    .or_else(Region::new("us-east-1"));
    Ok(aws_config::from_env().region(region_provider).load().await)
}

impl SmsSender {
    async fn new() -> Result<Self> {
        let sms_transport_name = std::env::var("SMS_TRANSPORT_NAME")
            .map_err(|err| anyhow!("SMS_TRANSPORT_NAME env var missing"))?;

        event!(
            Level::INFO,
            "SmsTransport: sms_transport_name={sms_transport_name}"
        );
        Ok(SmsSender {
            transport: match sms_transport_name.as_str() {
                "AwsSns" => {
                    let shared_config = get_aws_config().await?;
                    let client = AwsSnsClient::new(&shared_config);

                    let base_message_attributes: HashMap<String, String> = serde_json::from_str(
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

    async fn send(&self, receiver: String, message: String) -> Result<()> {
        match self.transport {
            SmsTransport::AwsSns((ref aws_client, ref messsage_attributes)) => {
                event!(
                    Level::INFO,
                    "SmsTransport::AwsSes: Sending SMS:\n\t - receiver={receiver}\n\t - message={message}",
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

enum EmailTransport {
    AwsSes(AwsSesClient),
    Console,
}

struct EmailSender {
    transport: EmailTransport,
    email_from: String,
}

impl EmailSender {
    async fn new() -> Result<Self> {
        let email_from =
            std::env::var("EMAIL_FROM").map_err(|err| anyhow!("EMAIL_FROM env var missing"))?;
        let email_transport_name = std::env::var("EMAIL_TRANSPORT_NAME")
            .map_err(|err| anyhow!("EMAIL_TRANSPORT_NAME env var missing"))?;

        event!(
            Level::INFO,
            "EmailTransport: from_address={email_from}, email_transport_name={email_transport_name}"
        );

        Ok(EmailSender {
            transport: match email_transport_name.as_str() {
                "AwsSes" => {
                    let shared_config = get_aws_config().await?;
                    EmailTransport::AwsSes(AwsSesClient::new(&shared_config))
                }
                _ => EmailTransport::Console,
            },
            email_from,
        })
    }

    async fn send(
        &self,
        receiver: String,
        subject: String,
        plaintext_body: String,
        html_body: String,
    ) -> Result<()> {
        match self.transport {
            EmailTransport::AwsSes(ref aws_client) => {
                event!(
                    Level::INFO,
                    "EmailTransport::AwsSes: Sending email:\n\t - receiver={receiver}\n\t - subject={subject}\n\t - plaintext_body={plaintext_body}\n\t - html_body={html_body}",
                );
                let mut dest: Destination = Destination::builder().build();
                dest.to_addresses = Some(vec![receiver]);
                let subject_content = Content::builder()
                    .data(subject)
                    .charset("UTF-8")
                    .build()
                    .map_err(|err| anyhow!("invalid subject: {:?}", err))?;
                let body_content = Content::builder()
                    .data(plaintext_body)
                    .charset("UTF-8")
                    .build()
                    .map_err(|err| anyhow!("invalid body: {:?}", err))?;
                let body = Body::builder().text(body_content).build();

                let msg = AwsMessage::builder()
                    .subject(subject_content)
                    .body(body)
                    .build();

                let email_content = EmailContent::builder().simple(msg).build();

                aws_client
                    .send_email()
                    .from_email_address(self.email_from.as_str())
                    .destination(dest)
                    .content(email_content)
                    .send()
                    .await
                    .map_err(|err| anyhow!("invalid subject: {:?}", err))?;
            }
            EmailTransport::Console => {
                let email = Message::builder()
                    .from(
                        self.email_from
                            .parse()
                            .map_err(|err| anyhow!("invalid email_from: {:?}", err))?,
                    )
                    .to(receiver
                        .parse()
                        .map_err(|err| anyhow!("invalid receiver: {:?}", err))?)
                    .subject(subject.clone())
                    .multipart(MultiPart::alternative_plain_html(
                        plaintext_body.clone(),
                        html_body.clone(),
                    ))
                    .map_err(|error| format!("{:?}", error))?;

                event!(
                    Level::INFO,
                    "EmailTransport::Console: Sending email:\n\t - receiver={receiver}\n\t - subject={subject}\n\t - plaintext_body={plaintext_body}\n\t - html_body={html_body}",
                );
            }
        }
        Ok(())
    }
}

#[instrument(skip(sender))]
async fn send_communication_sms(
    receiver: &Option<String>,
    template: &Option<SmsConfig>,
    variables: &Map<String, Value>,
    sender: &SmsSender,
) -> Result<()> {
    if let (Some(receiver), Some(config)) = (receiver, template) {
        let message = reports::render_template_text(config.message.as_str(), variables.clone())?;

        sender.send(receiver.into(), message).await?;
    } else {
        event!(Level::INFO, "Receiver empty, ignoring..");
    }
    Ok(())
}

#[instrument(skip(sender))]
async fn send_communication_email(
    receiver: &Option<String>,
    template: &Option<EmailConfig>,
    variables: &Map<String, Value>,
    sender: &EmailSender,
) -> Result<()> {
    if let (Some(receiver), Some(config)) = (receiver, template) {
        let subject = reports::render_template_text(config.subject.as_str(), variables.clone())?;
        let plaintext_body =
            reports::render_template_text(config.plaintext_body.as_str(), variables.clone())?;
        let html_body =
            reports::render_template_text(config.html_body.as_str(), variables.clone())?;

        sender
            .send(receiver.to_string(), subject, plaintext_body, html_body)
            .await?;
    } else {
        event!(Level::INFO, "Receiver empty, ignoring..");
    }
    Ok(())
}

#[instrument]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task]
pub async fn send_communication(
    body: SendCommunicationBody,
    tenant_id: String,
    election_event_id: Option<String>,
) -> Result<()> {
    let auth_headers = keycloak::get_client_credentials().await?;
    let celery_app = get_celery_app().await;
    let client = KeycloakAdminClient::new().await?;
    let realm = match election_event_id {
        Some(ref election_event_id) => get_event_realm(&tenant_id, &election_event_id),
        None => get_tenant_realm(&tenant_id),
    };

    let election_event = match election_event_id.clone() {
        None => None,
        Some(election_event_id) => {
            let event = get_election_event(
                auth_headers.clone(),
                tenant_id.clone(),
                election_event_id.clone(),
            )
            .await?
            .data
            .ok_or(anyhow!("Election event not found: {}", election_event_id))?
            .sequent_backend_election_event;
            if (event.is_empty()) {
                None
            } else {
                Some(event[0].clone())
            }
        }
    };

    let mut db_client: DbClient = get_database_pool()
        .await
        .get()
        .await
        .map_err(|err| anyhow!("{}", err))?;
    let batch_size = PgConfig::from_env()?.default_sql_batch_size;

    let user_ids = match body.audience_selection {
        AudienceSelection::SELECTED => body.audience_voter_ids.clone(),
        // TODO: managed "not voted" and "voted"
        _ => None,
    };

    // perform listing in batches in a read-only repeatable transaction, and
    // perform stats updates in a new stats transaction each time - because for
    // each mail/sms sent, there's no rollback for that.
    let mut processed: i32 = 0;
    event!(Level::INFO, "before transaction");
    let transaction = db_client
        .transaction()
        .await
        .map_err(|err| anyhow!("{err}"))?;
    event!(Level::INFO, "before isolation");
    transaction
        .simple_query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ;")
        .await
        .with_context(|| "can't set transaction isolation level")?;
    event!(Level::INFO, "after isolation");
    while true {
        let (users, total_count) = list_users(
            auth_headers.clone(),
            &transaction,
            &client,
            tenant_id.clone(),
            election_event_id.clone(),
            &realm,
            /* search */ None,
            /* email */ None,
            /* limit */ Some(batch_size),
            /* offset */ Some(processed),
            /* user_ids */ user_ids.clone(),
        )
        .await?;
        event!(Level::INFO, "after list_users");


        let email_sender = EmailSender::new().await?;
        let sms_sender = SmsSender::new().await?;
        let mut num_emails_sent = 0;
        let mut num_sms_sent = 0;

        for user in users.iter() {
            event!(
                Level::INFO,
                "Sending communication to user with id={:?} and email={:?}",
                id = user.id,
                email = user.email,
            );
            let variables: Map<String, Value> =
                get_variables(user, election_event.clone(), tenant_id.clone())?;
            match body.communication_method {
                CommunicationMethod::EMAIL => {
                    let sending_result = send_communication_email(
                        /* receiver */ &user.email,
                        /* template */ &body.email,
                        /* variables */ &variables,
                        /* sender */ &email_sender,
                    )
                    .await;
                    if let Err(error) = sending_result {
                        event!(Level::ERROR, "error sending email: {error:?}, continuing..");
                    } else {
                        num_emails_sent += 1;
                    }
                }
                CommunicationMethod::SMS => {
                    let sending_result = send_communication_sms(
                        /* receiver */ &user.get_mobile_phone(),
                        /* template */ &body.sms,
                        /* variables */ &variables,
                        /* sender */ &sms_sender,
                    )
                    .await;
                    if let Err(error) = sending_result {
                        event!(Level::ERROR, "error sending sms: {error:?}, continuing..");
                    } else {
                        num_sms_sent += 1;
                    }
                }
            };
        }

        processed += TryInto::<i32>::try_into(users.len()).map_err(|err| anyhow!("{err}"))?;

        // update stats
        if let Some(ref election_event_id) = election_event_id {
            let election_event_response = get_election_event(
                auth_headers.clone(),
                tenant_id.clone(),
                election_event_id.clone(),
            )
            .await
            .with_context(|| "can't find election event")?;
            event!(Level::INFO, "election_event_response={election_event_response:?}");

            let election_event = &election_event_response
                .data
                .with_context(|| "can't find election event")?
                .sequent_backend_election_event[0];
            event!(Level::INFO, "election_event={election_event:?}");

            let mut statistics = get_election_event_statistics(election_event.status.clone())
                .unwrap_or(Default::default());
            event!(Level::INFO, "statistics= {statistics:?}");

            statistics.num_emails_sent += num_emails_sent;
            statistics.num_sms_sent += num_sms_sent;

            update_election_event_statistics(
                tenant_id.clone(),
                election_event_id.clone(),
                statistics,
            )
            .await
            .with_context(|| "can't updated election event statistics")?;
        }

        if (processed >= total_count) {
            break;
        }
    }
    transaction.commit()
        .await
        .with_context(|| "error comitting transaction")?;
    Ok(())
}
