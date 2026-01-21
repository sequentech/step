// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::area::get_elections_by_area;
use crate::postgres::election_event::get_election_event_by_id_if_exist;
use crate::services::celery_app::get_celery_app;
use crate::services::election_event_board::get_election_event_board;
use crate::services::election_event_statistics::update_election_event_statistics;
use crate::services::election_statistics::update_election_statistics;
use crate::services::electoral_log::ElectoralLog;
use crate::services::providers::{email_sender::EmailSender, sms_sender::SmsSender};
use crate::services::users::{list_users, list_users_with_vote_info, ListUsersFilter};
use crate::types::error::Result;

use crate::services::database::{get_hasura_pool, get_keycloak_pool, PgConfig};
use crate::types::error::Error;
use deadpool_postgres::{Client as DbClient, Transaction};

use anyhow::{anyhow, Context};
use aws_sdk_sesv2::types::{Body, Content, Destination, EmailContent, Message as AwsMessage};
use aws_sdk_sesv2::Client as AwsSesClient;
use aws_sdk_sns::{types::MessageAttributeValue, Client as AwsSnsClient};
use celery::error::TaskError;
use lettre::message::MultiPart;
use lettre::Message;
use sequent_core::serialization::deserialize_with_path::*;
use sequent_core::services::generate_urls::get_auth_url;
use sequent_core::services::generate_urls::AuthAction;
use sequent_core::services::keycloak::{get_event_realm, get_tenant_realm};
use sequent_core::services::{keycloak, reports};
use sequent_core::types::hasura::core::ElectionEvent;
use sequent_core::types::keycloak::{User, UserArea, AREA_ID_ATTR_NAME};
use sequent_core::types::templates::{
    AudienceSelection, EmailConfig, SendTemplateBody, SmsConfig, TemplateMethod,
};
use sequent_core::util::aws::get_from_env_aws_config;
use serde_json::json;
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::default::Default;
use strand::info;
use tracing::{event, info, instrument, Level};

#[instrument(err)]
fn get_variables(
    user: &User,
    election_event: Option<ElectionEvent>,
    tenant_id: String,
    auth_action: AuthAction,
) -> Result<Map<String, Value>> {
    let mut variables: Map<String, Value> = Default::default();
    variables.insert(
        "user".to_string(),
        json!({
            "first_name": user.first_name.clone(),
            "last_name": user.last_name.clone(),
            "username": user.username.clone(),
            "first_name": user.first_name.clone(),
            "email": user.email.clone(),
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
            json!(get_auth_url(
                std::env::var("VOTING_PORTAL_URL")
                    .map_err(|err| anyhow!("VOTING_PORTAL_URL env var missing"))?
                    .as_str(),
                &tenant_id,
                &election_event.id,
                auth_action
            )),
        );
    }
    Ok(variables)
}

#[instrument(skip(sender), err)]
async fn send_template_sms(
    receiver: &Option<String>,
    template: &Option<SmsConfig>,
    variables: &Map<String, Value>,
    sender: &SmsSender,
) -> Result<Option<String>> {
    if let (Some(receiver), Some(config)) = (receiver, template) {
        let message = reports::render_template_text(config.message.as_str(), variables.clone())
            .map_err(|err| anyhow!("{}", err))?;

        sender.send(receiver.into(), message.clone()).await?;
        return Ok(Some(
            json!({
                "receiver": receiver,
                "message": message
            })
            .to_string(),
        ));
    } else {
        event!(Level::INFO, "Receiver empty, ignoring..");
    }
    Ok(None)
}

#[instrument(skip(sender), err)]
pub async fn send_template_email(
    receiver: &Option<String>,
    template: &Option<EmailConfig>,
    variables: &Map<String, Value>,
    sender: &EmailSender,
) -> Result<Option<String>> {
    if let (Some(receiver), Some(config)) = (receiver, template) {
        let subject = reports::render_template_text(config.subject.as_str(), variables.clone())
            .map_err(|err| anyhow!("Error rendering subject template: {err:?}"))?;

        let plaintext_body =
            reports::render_template_text(config.plaintext_body.as_str(), variables.clone())
                .map_err(|err| anyhow!("Error rendering plaintext body: {err:?}"))?;

        let html_body = match &config.html_body {
            Some(html_body) => Some(
                reports::render_template_text(html_body, variables.clone())
                    .map_err(|err| anyhow!("error rendering html body: {err:?}"))?,
            ),
            None => None,
        };
        info!("html_body: {html_body:?}");

        sender
            .send(
                vec![receiver.to_string()],
                subject.clone(),
                plaintext_body.clone(),
                html_body.clone(),
                /* attachments */ Vec::new(),
            )
            .await
            .map_err(|err| anyhow!("error sending email: {err:?}"))?;

        return Ok(Some(
            json!({
                "receiver": receiver,
                "subject": subject,
                "html_body": html_body,
                "plaintext_body": plaintext_body
            })
            .to_string(),
        ));
    } else {
        // Log the event if the receiver or template is missing
        event!(
            Level::INFO,
            "Receiver or template is empty, email not sent."
        );
    }
    Ok(None)
}

#[derive(Default, Debug)]
struct MetricsUnit {
    num_emails_sent: i64,
    num_sms_sent: i64,
}
#[derive(Default, Debug)]
struct Metrics {
    election_event: MetricsUnit,
    metrics_by_election_id: HashMap<String, MetricsUnit>,
}

fn update_metrics_unit(metrics_unit: &mut MetricsUnit, communication_method: &TemplateMethod) {
    match communication_method {
        TemplateMethod::EMAIL => {
            metrics_unit.num_emails_sent += 1;
        }
        TemplateMethod::SMS => {
            metrics_unit.num_sms_sent += 1;
        }
        TemplateMethod::DOCUMENT => {}
    };
}

fn update_metrics(
    metrics: &mut Metrics,
    elections_by_area: &HashMap<String, Vec<String>>,
    user: &User,
    communication_method: &TemplateMethod,
    success: bool,
) {
    // if the op was not successful, then do not update
    if !success {
        return;
    }
    update_metrics_unit(&mut metrics.election_event, communication_method);
    let Some(UserArea {
        id: Some(ref area_id),
        ..
    }) = user.area
    else {
        // voter has no area associated, so no need to update metrics related to
        // the area
        return;
    };
    let Some(election_ids) = elections_by_area.get(area_id) else {
        // area not found in list. strange! but we continue
        event!(
            Level::INFO,
            "Area id={area_id} not found in elections_by_area, strange"
        );
        return;
    };
    election_ids.iter().for_each(|election_id| {
        metrics
            .metrics_by_election_id
            .entry(election_id.clone())
            .and_modify(|metrics_unit| update_metrics_unit(metrics_unit, communication_method))
            .or_insert_with(|| {
                let mut metrics_unit = Default::default();
                update_metrics_unit(&mut metrics_unit, communication_method);
                metrics_unit
            });
    });
}

async fn update_stats(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &String,
    election_event_id: &Option<String>,
    metrics: &Metrics,
) -> Result<()> {
    let &Some(ref election_event_id) = election_event_id else {
        return Ok(());
    };
    let totals = metrics.election_event.num_emails_sent + metrics.election_event.num_sms_sent;
    if totals > 0 {
        event!(Level::INFO, "updating election event statistics");

        update_election_event_statistics(
            hasura_transaction,
            tenant_id.as_str(),
            election_event_id.as_str(),
            /* inc_emails_sent */ metrics.election_event.num_emails_sent,
            /* inc_sms_sent */ metrics.election_event.num_sms_sent,
        )
        .await
        .with_context(|| "can't updated election event statistics")?;
    }
    for (election_id, election_metrics) in metrics.metrics_by_election_id.iter() {
        let totals = election_metrics.num_emails_sent + election_metrics.num_sms_sent;
        if totals > 0 {
            update_election_statistics(
                hasura_transaction,
                tenant_id.as_str(),
                election_event_id.as_str(),
                election_id.as_str(),
                /* inc_emails_sent */ metrics.election_event.num_emails_sent,
                /* inc_sms_sent */ metrics.election_event.num_sms_sent,
            )
            .await
            .with_context(|| "can't updated election event statistics")?;
        }
    }
    Ok(())
}

async fn on_success_send_message(
    hasura_transaction: &Transaction<'_>,
    election_event: Option<ElectionEvent>,
    user_id: Option<String>,
    username: Option<String>,
    message: &str,
    tenant_id: &str,
    admin_id: &str,
    area_id: Option<String>,
) -> Result<()> {
    if let Some(election_event) = election_event {
        let board_name = get_election_event_board(election_event.bulletin_board_reference.clone())
            .with_context(|| "missing bulletin board")?;

        let electoral_log = ElectoralLog::for_admin_user(
            hasura_transaction,
            &board_name,
            tenant_id,
            &election_event.id,
            admin_id,
            username.clone(),
            None,
            area_id.clone(),
        )
        .await
        .map_err(|e| anyhow!("Error obtaining the electoral log: {e:?}"))?;

        electoral_log
            .post_send_template(
                Some(message.into()),
                election_event.id.clone(),
                user_id,
                username,
                None,
                area_id,
            )
            .await
            .map_err(|e| anyhow!("error posting to the electoral log: {e:?}"))?;
    } else {
        event!(
            Level::WARN,
            "No election event provided for user: {username:?} ({user_id:?})"
        );
    }

    Ok(())
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task]
pub async fn send_template(
    body: SendTemplateBody,
    tenant_id: String,
    admin_id: String,
    election_event_id: Option<String>,
) -> Result<()> {
    let celery_app = get_celery_app().await;
    let realm = match election_event_id {
        Some(ref election_event_id) => get_event_realm(&tenant_id, &election_event_id),
        None => get_tenant_realm(&tenant_id),
    };

    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|err| format!("Error getting hasura db pool: {err}"))?;

    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|err| format!("Error starting hasura transaction: {err}"))?;

    let election_event = match election_event_id.clone() {
        None => None,
        Some(election_event_id) => {
            get_election_event_by_id_if_exist(&hasura_transaction, &tenant_id, &election_event_id)
                .await?
        }
    };

    let mut keycloak_db_client: DbClient = get_keycloak_pool()
        .await
        .get()
        .await
        .map_err(|err| anyhow!("{}", err))?;
    let batch_size = PgConfig::from_env()?.default_sql_batch_size;

    let Some(audience_selection) = body.audience_selection.clone() else {
        return Err(Error::String("Missing audience selection".to_string()));
    };
    let user_ids = match audience_selection {
        AudienceSelection::SELECTED => body.audience_voter_ids.clone(),
        _ => None,
    };

    // perform listing in batches in a read-only repeatable transaction, and
    // perform stats updates in a new stats transaction each time - because for
    // each mail/sms sent, there's no rollback for that.
    let mut processed: i32 = 0;
    event!(Level::INFO, "before transaction");
    let keycloak_transaction = keycloak_db_client
        .transaction()
        .await
        .map_err(|err| anyhow!("{err}"))?;
    event!(Level::INFO, "before isolation");
    keycloak_transaction
        .simple_query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ;")
        .await
        .with_context(|| "can't set transaction isolation level")?;
    event!(Level::INFO, "after isolation");
    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .with_context(|| "Error loading hasura db client")?;

    let elections_by_area = match election_event_id.clone() {
        None => HashMap::new(),
        Some(ref election_event_id) => get_elections_by_area(
            &hasura_transaction,
            tenant_id.as_str(),
            election_event_id.as_str(),
        )
        .await
        .with_context(|| "Error listing elections by area")?,
    };

    loop {
        let hasura_transaction = hasura_db_client
            .transaction()
            .await
            .with_context(|| "Error creating a transaction")?;

        let filter = ListUsersFilter {
            tenant_id: tenant_id.clone(),
            election_event_id: election_event_id.clone(),
            election_id: None,
            area_id: None,
            realm: realm.clone(),
            search: None,
            first_name: None,
            last_name: None,
            username: None,
            email: None,
            limit: Some(batch_size),
            offset: Some(processed),
            user_ids: user_ids.clone(),
            attributes: None,
            enabled: None,
            email_verified: None,
            sort: None,
            has_voted: None,
            authorized_to_election_alias: None,
        };

        let (users, total_count) = match audience_selection {
            AudienceSelection::NOT_VOTED | AudienceSelection::VOTED => {
                list_users_with_vote_info(&hasura_transaction, &keycloak_transaction, filter)
                    .await
                    .with_context(|| "Failed to featch list_users_with_vote_info")?
            }
            _ => list_users(&hasura_transaction, &keycloak_transaction, filter)
                .await
                .with_context(|| "Failed to featch list_users")?,
        };

        let mut filtered_users = users.clone();

        match audience_selection {
            AudienceSelection::NOT_VOTED => filtered_users.retain(|user| {
                user.votes_info
                    .as_ref()
                    .map_or(false, |vote_info| vote_info.is_empty())
            }),
            AudienceSelection::VOTED => filtered_users.retain(|user| {
                user.votes_info
                    .as_ref()
                    .map_or(false, |vote_info| !vote_info.is_empty())
            }),
            _ => {}
        };

        let email_sender = EmailSender::new().await?;
        let sms_sender = SmsSender::new().await?;
        let mut metrics = Metrics {
            election_event: MetricsUnit {
                num_emails_sent: 0,
                num_sms_sent: 0,
            },
            metrics_by_election_id: Default::default(),
        };

        let Some(communication_method) = body.communication_method.clone() else {
            return Err(Error::String("Missing template method".into()));
        };

        for user in filtered_users.iter() {
            let success = send_template_email_or_sms(
                &hasura_transaction,
                &user,
                &election_event,
                &tenant_id,
                Some(admin_id.clone()),
                &body.email,
                &body.sms,
                &email_sender,
                &sms_sender,
                Some(communication_method.clone()),
            )
            .await;
            update_metrics(
                &mut metrics,
                &elections_by_area,
                &user,
                /* communication_method */ &communication_method,
                /* success */ success.is_ok(),
            );
        }

        processed += TryInto::<i32>::try_into(users.len()).map_err(|err| anyhow!("{err}"))?;

        // update stats
        update_stats(
            &hasura_transaction,
            &tenant_id,
            &election_event_id,
            &metrics,
        )
        .await
        .with_context(|| "Error updating stats")?;

        hasura_transaction
            .commit()
            .await
            .with_context(|| "Error committing update stats transaction")?;

        if processed >= total_count {
            break;
        }
    }
    keycloak_transaction
        .commit()
        .await
        .with_context(|| "error comitting transaction")?;

    Ok(())
}

/// In the case of rejection:
/// admin_id and election_event are not needed so both can be set to None.
///
/// Since there is no user_id, the User object must be constructed from the data in applications table and its id set to None.
///
/// Also we do not write the rejections in the Elecoral log since anyone can apply, it would overbloat the electoral log.
///
/// In the case of acceptance:
/// All the fields are required.
#[instrument(err, skip(election_event, email_sender, sms_sender))]
pub async fn send_template_email_or_sms(
    hasura_transaction: &Transaction<'_>,
    user: &User,
    election_event: &Option<ElectionEvent>,
    tenant_id: &str,
    admin_id_opt: Option<String>,
    email_config: &Option<EmailConfig>,
    sms_config: &Option<SmsConfig>,
    email_sender: &EmailSender,
    sms_sender: &SmsSender,
    communication_method: Option<TemplateMethod>,
) -> Result<()> {
    event!(
        Level::INFO,
        "Sending template to user with id={id:?} and email={email:?}",
        id = user.id,
        email = user.email,
    );
    let admin_id = admin_id_opt.unwrap_or("".into());
    let variables: Map<String, Value> = get_variables(
        user,
        election_event.clone(),
        tenant_id.to_string(),
        AuthAction::Login,
    )?;

    let user_area_id = user.attributes.as_ref().and_then(|attributes| {
        attributes
            .get(AREA_ID_ATTR_NAME)
            .and_then(|area_id| area_id.first().cloned())
    });

    match communication_method {
        Some(TemplateMethod::EMAIL) => {
            let sending_result = send_template_email(
                &user.email,   // receiver user email
                email_config,  // Template content: EmailConfig
                &variables,    // Variables for the template
                &email_sender, // Sender client to send emails: EmailSender
            )
            .await;
            match sending_result {
                Ok(Some(message)) if user.id.is_some() => {
                    if let Err(e) = on_success_send_message(
                        hasura_transaction,
                        election_event.clone(),
                        user.id.clone(),
                        user.username.clone(),
                        &message,
                        &tenant_id,
                        &admin_id,
                        user_area_id,
                    )
                    .await
                    {
                        event!(Level::ERROR, "Error processing success message: {e:?}");
                    }
                    Ok(())
                }
                Ok(Some(_)) => Ok(()),
                Ok(None) => {
                    event!(Level::WARN, "No email was sent.");
                    Ok(())
                }
                Err(error) => {
                    event!(Level::ERROR, "error sending email: {error:?}, continuing..");
                    Err(error)
                }
            }
        }
        Some(TemplateMethod::SMS) => {
            let sending_result = send_template_sms(
                /* receiver */ &user.get_mobile_phone(),
                /* template */ sms_config,
                /* variables */ &variables,
                /* sender */ &sms_sender,
            )
            .await;
            match sending_result {
                Ok(Some(message)) if user.id.is_some() => {
                    if let Err(e) = on_success_send_message(
                        hasura_transaction,
                        election_event.clone(),
                        user.id.clone(),
                        None,
                        &message,
                        tenant_id,
                        &admin_id,
                        user_area_id,
                    )
                    .await
                    {
                        event!(Level::ERROR, "Error processing success message: {e:?}");
                    }
                    Ok(())
                }
                Ok(Some(_)) => Ok(()),
                Ok(None) => {
                    event!(Level::WARN, "No sms was sent.");
                    Ok(())
                }
                Err(error) => {
                    event!(Level::ERROR, "error sending sms: {error:?}, continuing..");
                    Err(error)
                    // Err(Error::String(format!("")))
                }
            }
        }
        _ => {
            //nothing to do
            Ok(())
        }
    }
}
