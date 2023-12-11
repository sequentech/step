// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::hasura::area::get_election_event_areas;
use crate::hasura::election_event::get_election_event;
use crate::hasura::tally_session::{get_tally_session_highest_batch, insert_tally_session};
use crate::hasura::tally_session_contest::insert_tally_session_contest;
use crate::hasura::trustee::get_trustees_by_id;
use crate::services::celery_app::get_celery_app;
use crate::services::users::list_users;
use crate::tasks::insert_ballots::{insert_ballots, InsertBallotsPayload};
use crate::tasks::send_communication::get_election_event::GetElectionEventSequentBackendElectionEvent;
use crate::types::error::Result;

use anyhow::{anyhow, Context};

use aws_config::meta::region::RegionProviderChain;
use aws_sdk_sesv2::types::{Body, Content, Destination, EmailContent, Message as AwsMessage};
use aws_sdk_sesv2::{config::Region, meta::PKG_VERSION, Client as AwsClient, Error};
use lettre::message::{header::ContentType, MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};

use braid_messages::newtypes::BatchNumber;
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

enum EmailTransport {
    AwsSes(AwsClient),
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
                    let region_provider = RegionProviderChain::first_try(Region::new(
                        std::env::var("AWS_REGION")
                            .map_err(|err| anyhow!("AWS_REGION env var missing"))?,
                    ))
                    .or_default_provider()
                    .or_else(Region::new("us-west-2"));
                    let shared_config = aws_config::from_env().region(region_provider).load().await;
                    EmailTransport::AwsSes(AwsClient::new(&shared_config))
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
            EmailTransport::AwsSes(ref aws_sender) => {
                // TODO
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
                    "EmailTransport::Console: Sending email:\n\t - subject={subject}\n\t - plaintext_body={plaintext_body}\n\t - html_body={html_body}",
                );
            }
        }
        Ok(())
    }
}

#[instrument(skip(sender))]
async fn send_communication_email(
    receiver: &Option<String>,
    template: &Option<EmailConfig>,
    variables: &Map<String, Value>,
    sender: &EmailSender,
) -> Result<()> {
    event!(
        Level::INFO,
        "TODO: Send email receiver={:?}",
        receiver = receiver,
    );
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

    let (users, count) = list_users(
        auth_headers.clone(),
        &client,
        tenant_id.clone(),
        election_event_id.clone(),
        &realm,
        None,
        None,
        None,
        None,
    )
    .await?;

    let email_sender = EmailSender::new().await?;

    for user in users.iter().filter(|user| {
        (body.audience_selection == AudienceSelection::ALL_USERS
            || (body.audience_selection == AudienceSelection::SELECTED
                && user.id.is_some()
                && body
                    .audience_voter_ids
                    .as_ref()
                    .unwrap_or(&vec![])
                    .contains(&user.id.as_ref().unwrap())))
    }) {
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
                    /* to */ &user.email,
                    /* template */ &body.email,
                    /* variables */ &variables,
                    /* sender */ &email_sender,
                )
                .await;
                if let Err(error) = sending_result {
                    event!(Level::ERROR, "error sending email: {error:?}, continuing..");
                }
            }
            CommunicationMethod::SMS => {
                event!(Level::INFO, "TODO: Send SMS");
            }
        };
    }
    Ok(())
}
