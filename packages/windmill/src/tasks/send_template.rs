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
use crate::services::voter_secret_attributes::{
    decrypt_user_attributes, get_secret_attribute_config, strip_undeclared_secret_attributes,
};
use crate::types::error::Result;

use crate::services::database::{get_hasura_pool, get_keycloak_pool, PgConfig};
use crate::types::error::Error;
use deadpool_postgres::{Client as DbClient, Transaction};

use crate::services::messaging::dispatch::{
    deliver, Delivery, DeliveryResult, Dispatcher, FallbackPolicy, ProviderTemplate, Recipient,
};
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
use sequent_core::services::translations::Name;
use sequent_core::services::{keycloak, reports};
use sequent_core::types::hasura::core::ElectionEvent;
use sequent_core::types::keycloak::{
    User, UserArea, AREA_ID_ATTR_NAME, LOCALE_ATTR_NAME, MESSAGE_CHANNEL_ATTR_NAME,
    MESSENGER_ID_ATTR_NAME, MOBILE_PHONE_ATTR_NAME, VERIFIED_CHANNELS_ATTR_NAME,
    VIBER_NUMBER_ATTR_NAME, WHATSAPP_NUMBER_ATTR_NAME,
};
use sequent_core::types::messaging::{
    MessageAttemptState, MessageChannel, MessageContent, MessagePurpose,
};
use sequent_core::types::templates::{
    AudienceSelection, ChannelSelection, EmailConfig, SendTemplateBody, SmsConfig, TemplateMethod,
};
use sequent_core::util::aws::get_from_env_aws_config;
use serde_json::json;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::default::Default;
use std::str::FromStr;
use strand::info;
use tracing::{event, info, instrument, Level};
use uuid::Uuid;

#[instrument(skip_all, err)]
fn get_variables(
    user: &User,
    election_event: Option<ElectionEvent>,
    tenant_id: String,
    auth_action: AuthAction,
) -> Result<Map<String, Value>> {
    let mut variables: Map<String, Value> = Default::default();
    let mut user_variables = Map::new();
    user_variables.insert("first_name".to_string(), json!(user.first_name));
    user_variables.insert("last_name".to_string(), json!(user.last_name));
    user_variables.insert("username".to_string(), json!(user.username));
    user_variables.insert("email".to_string(), json!(user.email));

    let attributes = user.attributes.clone().unwrap_or_default();
    for (attribute_name, values) in &attributes {
        if let Some(first_value) = values.first() {
            user_variables
                .entry(attribute_name.clone())
                .or_insert_with(|| json!(first_value));
        }
    }
    user_variables.insert("attributes".to_string(), json!(attributes));
    variables.insert("user".to_string(), Value::Object(user_variables));
    variables.insert("tenant_id".to_string(), json!(tenant_id.clone()));
    if let Some(ref election_event) = election_event {
        let default_language = election_event.get_default_language();
        variables.insert(
            "election_event".to_string(),
            json!({
                "id": election_event.id.clone(),
                "name": election_event.get_name(&default_language)
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

/// Builds the delivery record posted to the immutable electoral log.
///
/// The record keeps what an auditor needs to confirm a delivery — the channel, the address it
/// went to — but never rendered subjects or bodies. Those fields can carry
/// notification URL, including any `login_hint__*` values, and the electoral log cannot be
/// redacted once written.
fn delivery_audit_message(channel: &str, receiver: &str) -> String {
    let mut record = Map::new();
    record.insert("channel".to_string(), json!(channel));
    record.insert("receiver".to_string(), json!(receiver));
    Value::Object(record).to_string()
}

#[instrument(skip_all, err)]
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
        return Ok(Some(delivery_audit_message("sms", receiver)));
    } else {
        event!(Level::INFO, "Receiver empty, ignoring..");
    }
    Ok(None)
}

#[instrument(skip_all, err)]
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

        return Ok(Some(delivery_audit_message("email", receiver)));
    } else {
        // Log the event if the receiver or template is missing
        event!(
            Level::INFO,
            "Receiver or template is empty, email not sent."
        );
    }
    Ok(None)
}

/// Messages sent per channel.
#[derive(Default, Debug, PartialEq)]
struct MetricsUnit {
    sent: BTreeMap<MessageChannel, i64>,
}

impl MetricsUnit {
    fn record(&mut self, channel: MessageChannel) {
        *self.sent.entry(channel).or_default() += 1;
    }

    fn is_empty(&self) -> bool {
        self.sent.values().all(|count| *count == 0)
    }

    /// Increments keyed by their `statistics` counter name.
    fn statistics_increments(&self) -> BTreeMap<String, i64> {
        self.sent
            .iter()
            .filter(|(_, count)| **count > 0)
            .map(|(channel, count)| (channel.statistics_key().to_string(), *count))
            .collect()
    }
}

#[derive(Default, Debug)]
struct Metrics {
    election_event: MetricsUnit,
    metrics_by_election_id: HashMap<String, MetricsUnit>,
}

fn update_metrics(
    metrics: &mut Metrics,
    elections_by_area: &HashMap<String, Vec<String>>,
    user: &User,
    channel: MessageChannel,
    success: bool,
) {
    // if the op was not successful, then do not update
    if !success {
        return;
    }
    metrics.election_event.record(channel);
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
            .or_default()
            .record(channel);
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
    if !metrics.election_event.is_empty() {
        event!(Level::INFO, "updating election event statistics");

        update_election_event_statistics(
            hasura_transaction,
            tenant_id.as_str(),
            election_event_id.as_str(),
            &metrics.election_event.statistics_increments(),
        )
        .await
        .with_context(|| "can't updated election event statistics")?;
    }
    for (election_id, increments) in election_statistics_increments(metrics) {
        update_election_statistics(
            hasura_transaction,
            tenant_id.as_str(),
            election_event_id.as_str(),
            election_id,
            &increments,
        )
        .await
        .with_context(|| "can't updated election statistics")?;
    }
    Ok(())
}

/// Per-election counter increments, skipping elections with nothing sent.
fn election_statistics_increments(metrics: &Metrics) -> Vec<(&str, BTreeMap<String, i64>)> {
    metrics
        .metrics_by_election_id
        .iter()
        .filter(|(_, unit)| !unit.is_empty())
        .map(|(election_id, unit)| (election_id.as_str(), unit.statistics_increments()))
        .collect()
}

/// The voter IDs that scope the audience. `SELECTED` must name at least one
/// voter: an empty selection would otherwise reach the whole realm.
fn audience_user_ids(
    audience_selection: &AudienceSelection,
    audience_voter_ids: &Option<Vec<String>>,
) -> Result<Option<Vec<String>>> {
    match audience_selection {
        AudienceSelection::SELECTED => match audience_voter_ids {
            Some(ids) if !ids.is_empty() => Ok(Some(ids.clone())),
            _ => Err(Error::String(
                "Audience SELECTED requires at least one voter id".to_string(),
            )),
        },
        _ => Ok(None),
    }
}

fn first_attribute<'a>(user: &'a User, name: &str) -> Option<&'a String> {
    user.attributes
        .as_ref()
        .and_then(|attributes| attributes.get(name))
        .and_then(|values| values.first())
        .filter(|value| !value.is_empty())
}

/// The voter's address on each channel, the channels they verified, and
/// their elections.
fn voter_recipient(user: &User, elections_by_area: &HashMap<String, Vec<String>>) -> Recipient {
    let mut destinations = BTreeMap::new();
    if let Some(email) = user.email.as_ref().filter(|email| !email.is_empty()) {
        destinations.insert(MessageChannel::EMAIL, email.clone());
    }
    for (channel, attribute) in [
        (MessageChannel::SMS, MOBILE_PHONE_ATTR_NAME),
        (MessageChannel::WHATSAPP, WHATSAPP_NUMBER_ATTR_NAME),
        (MessageChannel::VIBER, VIBER_NUMBER_ATTR_NAME),
        (MessageChannel::MESSENGER, MESSENGER_ID_ATTR_NAME),
    ] {
        if let Some(value) = first_attribute(user, attribute) {
            destinations.insert(channel, value.clone());
        }
    }
    let declared: Option<Vec<MessageChannel>> = user
        .attributes
        .as_ref()
        .and_then(|attributes| attributes.get(VERIFIED_CHANNELS_ATTR_NAME))
        .map(|values| {
            values
                .iter()
                .filter_map(|value| MessageChannel::from_str(value).ok())
                .collect()
        });
    // Voters enrolled before verified channels existed keep their email and
    // mobile number, as before.
    let mut verified: Vec<MessageChannel> = declared.unwrap_or_else(|| {
        [MessageChannel::EMAIL, MessageChannel::SMS]
            .into_iter()
            .filter(|channel| destinations.contains_key(channel))
            .collect()
    });
    verified.sort();
    verified.dedup();
    let election_ids = user
        .area
        .as_ref()
        .and_then(|area| area.id.as_ref())
        .and_then(|area_id| elections_by_area.get(area_id))
        .cloned()
        .unwrap_or_default();
    Recipient {
        voter_id: user.id.clone(),
        destinations,
        verified,
        election_ids,
    }
}

/// Where a voter's notice goes first.
fn first_channel(
    selection: ChannelSelection,
    method_channel: Option<MessageChannel>,
    user: &User,
) -> Option<MessageChannel> {
    match selection {
        ChannelSelection::SINGLE_CHANNEL => method_channel,
        ChannelSelection::VOTER_PREFERENCE => first_attribute(user, MESSAGE_CHANNEL_ATTR_NAME)
            .and_then(|value| MessageChannel::from_str(value).ok())
            .or(method_channel)
            .or(Some(MessageChannel::EMAIL)),
    }
}

/// Approved templates named by the send itself, per channel.
fn provider_templates(body: &SendTemplateBody) -> BTreeMap<MessageChannel, ProviderTemplate> {
    [
        (MessageChannel::WHATSAPP, &body.whatsapp),
        (MessageChannel::VIBER, &body.viber),
        (MessageChannel::MESSENGER, &body.messenger),
    ]
    .into_iter()
    .filter_map(|(channel, config)| {
        let config = config.as_ref()?;
        let name = config
            .provider_template
            .clone()
            .filter(|name| !name.is_empty())?;
        Some((
            channel,
            ProviderTemplate {
                name,
                language: config.provider_language.clone().filter(|l| !l.is_empty()),
            },
        ))
    })
    .collect()
}

fn render(text: &str, variables: &Map<String, Value>) -> Result<String> {
    reports::render_template_text(text, variables.clone())
        .map_err(|err| Error::String(format!("Error rendering template: {err:?}")))
}

/// The send's content rendered for this voter, on each channel it has.
fn render_contents(
    body: &SendTemplateBody,
    variables: &Map<String, Value>,
) -> Result<BTreeMap<MessageChannel, MessageContent>> {
    let mut contents = BTreeMap::new();
    if let Some(email) = &body.email {
        contents.insert(
            MessageChannel::EMAIL,
            MessageContent {
                subject: Some(render(&email.subject, variables)?),
                text: render(&email.plaintext_body, variables)?,
                html: email
                    .html_body
                    .as_deref()
                    .map(|html| render(html, variables))
                    .transpose()?,
                template_parameters: vec![],
                code: None,
            },
        );
    }
    if let Some(sms) = &body.sms {
        contents.insert(
            MessageChannel::SMS,
            MessageContent {
                subject: None,
                text: render(&sms.message, variables)?,
                html: None,
                template_parameters: vec![],
                code: None,
            },
        );
    }
    for (channel, config) in [
        (MessageChannel::WHATSAPP, &body.whatsapp),
        (MessageChannel::VIBER, &body.viber),
        (MessageChannel::MESSENGER, &body.messenger),
    ] {
        if let Some(config) = config {
            contents.insert(
                channel,
                MessageContent {
                    subject: None,
                    text: render(&config.message, variables)?,
                    html: None,
                    template_parameters: config
                        .parameters
                        .iter()
                        .map(|parameter| render(parameter, variables))
                        .collect::<Result<_>>()?,
                    code: None,
                },
            );
        }
    }
    Ok(contents)
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

/// Sends the voter's notice again after a transient failure. The retry is
/// the same send for this voter only, so attempts already recorded decide
/// what happens.
async fn retry_voter(
    body: &SendTemplateBody,
    send_id: &str,
    voter_id: &str,
    tenant_id: &str,
    admin_id: &str,
    election_event_id: &Option<String>,
    after: chrono::Duration,
) {
    let mut retry = body.clone();
    retry.audience_selection = Some(AudienceSelection::SELECTED);
    retry.audience_voter_ids = Some(vec![voter_id.to_string()]);
    retry.send_id = Some(send_id.to_string());
    let countdown = u32::try_from(after.num_seconds().max(1)).unwrap_or(u32::MAX);
    let celery_app = get_celery_app().await;
    if let Err(error) = celery_app
        .send_task(
            send_template::new(
                retry,
                tenant_id.to_string(),
                admin_id.to_string(),
                election_event_id.clone(),
            )
            .with_countdown(countdown),
        )
        .await
    {
        event!(
            Level::ERROR,
            "could not schedule a retry for voter {voter_id}: {error:?}"
        );
    }
}

/// The electoral log record of a sent message: channel, masked address,
/// attempt and outcome, never the address or the content.
fn ledger_audit_message(result: &DeliveryResult) -> String {
    let mut record = Map::new();
    record.insert(
        "channel".to_string(),
        json!(result.channel.map(|c| c.to_string().to_lowercase())),
    );
    record.insert("receiver".to_string(), json!(result.masked_destination));
    record.insert(
        "message_id".to_string(),
        json!(result.message_id.map(|id| id.to_string())),
    );
    record.insert("state".to_string(), json!(result.state.to_string()));
    Value::Object(record).to_string()
}

#[instrument(skip_all, err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(bind = true, max_retries = 0)]
pub async fn send_template(
    task: &Self,
    body: SendTemplateBody,
    tenant_id: String,
    admin_id: String,
    election_event_id: Option<String>,
) -> Result<()> {
    let send_id = body
        .send_id
        .clone()
        .unwrap_or_else(|| task.request.id.clone());
    let tenant_uuid = Uuid::parse_str(&tenant_id).map_err(|err| anyhow!("{err}"))?;
    let event_uuid = election_event_id
        .as_deref()
        .map(Uuid::parse_str)
        .transpose()
        .map_err(|err| anyhow!("{err}"))?;
    let dispatcher = Dispatcher::global().await?;
    let mut delivery_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|err| format!("Error getting hasura db pool: {err}"))?;
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
    let requested_secret_names = body
        .secret_attribute_names
        .iter()
        .cloned()
        .collect::<HashSet<_>>();
    // Undeclared secrets are stripped whatever the profile's state; decrypting
    // declared ones additionally requires a valid configuration.
    let configured_secret_names = if let Some(event_id) = election_event_id.as_deref() {
        let config = get_secret_attribute_config(&tenant_id, event_id)
            .await
            .map_err(|error| {
                anyhow!("Error reading the secret-attribute configuration: {error:#}")
            })?;
        if requested_secret_names.is_empty() {
            config.redacted_names().clone()
        } else {
            config.validated_names()?
        }
    } else {
        HashSet::new()
    };
    if let Some(name) = requested_secret_names
        .iter()
        .find(|name| !configured_secret_names.contains(*name))
    {
        return Err(Error::String(format!(
            "Template requested `{name}`, which is not configured as an encrypted voter attribute"
        )));
    }
    if !requested_secret_names.is_empty() && election_event_id.is_none() {
        return Err(Error::String(
            "Encrypted voter attributes require an election event".to_string(),
        ));
    }

    let mut keycloak_db_client: DbClient = get_keycloak_pool()
        .await
        .get()
        .await
        .map_err(|err| anyhow!("{}", err))?;
    let batch_size = PgConfig::from_env()?.default_sql_batch_size;

    let Some(audience_selection) = body.audience_selection.clone() else {
        return Err(Error::String("Missing audience selection".to_string()));
    };
    let user_ids = audience_user_ids(&audience_selection, &body.audience_voter_ids)?;

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

        let mut metrics = Metrics::default();
        let method_channel = body
            .communication_method
            .as_ref()
            .and_then(TemplateMethod::channel);
        if body.channel_selection == ChannelSelection::SINGLE_CHANNEL && method_channel.is_none() {
            return Err(Error::String("Missing template method".into()));
        }
        let fallback = match body.channel_selection {
            ChannelSelection::SINGLE_CHANNEL => FallbackPolicy::NONE,
            ChannelSelection::VOTER_PREFERENCE => FallbackPolicy::EVENT_ORDER,
        };

        for user in filtered_users.iter() {
            let mut render_user = user.clone();
            if let Some(event_id) = election_event_id.as_deref() {
                decrypt_user_attributes(
                    &mut render_user,
                    &tenant_id,
                    event_id,
                    &requested_secret_names,
                )
                .await
                .with_context(|| {
                    format!(
                        "Failed to decrypt declared secret attributes for voter {}",
                        user.id.as_deref().unwrap_or("unknown")
                    )
                })?;
            }
            strip_undeclared_secret_attributes(
                &mut render_user,
                &configured_secret_names,
                &requested_secret_names,
            );
            let (Some(voter_id), Some(first)) = (
                user.id.clone(),
                first_channel(body.channel_selection, method_channel, user),
            ) else {
                continue;
            };
            let variables = get_variables(
                &render_user,
                election_event.clone(),
                tenant_id.clone(),
                AuthAction::Login,
            )?;
            let contents = match render_contents(&body, &variables) {
                Ok(contents) => contents,
                Err(error) => {
                    event!(Level::ERROR, "voter {voter_id}: {error:?}, continuing..");
                    continue;
                }
            };
            let delivery = Delivery {
                tenant_id: tenant_uuid,
                election_event_id: event_uuid,
                purpose: MessagePurpose::NOTICE,
                first_channel: first,
                fallback,
                recipient: voter_recipient(user, &elections_by_area),
                contents,
                language: first_attribute(user, LOCALE_ATTR_NAME).cloned(),
                template_alias: body.alias.clone(),
                logical_key: format!("send-template:{send_id}:{voter_id}"),
                expires_at: None,
                account_id: None,
                template_key: body.alias.clone(),
                provider_templates: provider_templates(&body),
            };
            let result = match deliver(&mut delivery_client, dispatcher, &delivery).await {
                Ok(result) => result,
                Err(error) => {
                    event!(Level::ERROR, "voter {voter_id}: {error:?}, continuing..");
                    continue;
                }
            };
            if let Some(after) = result.retry_after {
                retry_voter(
                    &body,
                    &send_id,
                    &voter_id,
                    &tenant_id,
                    &admin_id,
                    &election_event_id,
                    after,
                )
                .await;
            }
            let sent = matches!(
                result.state,
                MessageAttemptState::ACCEPTED | MessageAttemptState::DELIVERED
            );
            if let (true, Some(channel)) = (sent, result.channel) {
                update_metrics(&mut metrics, &elections_by_area, user, channel, true);
                let area_id = user.area.as_ref().and_then(|area| area.id.clone());
                if let Err(error) = on_success_send_message(
                    &hasura_transaction,
                    election_event.clone(),
                    Some(voter_id.clone()),
                    user.username.clone(),
                    &ledger_audit_message(&result),
                    &tenant_id,
                    &admin_id,
                    area_id,
                )
                .await
                {
                    event!(Level::ERROR, "Error processing success message: {error:?}");
                }
            }
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
#[instrument(skip_all, err)]
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

#[cfg(test)]
mod tests {
    use super::{
        audience_user_ids, delivery_audit_message, election_statistics_increments, first_channel,
        get_variables, render_contents, voter_recipient, Metrics, MetricsUnit,
    };
    use sequent_core::services::generate_urls::AuthAction;
    use sequent_core::types::keycloak::{User, UserArea};
    use sequent_core::types::messaging::MessageChannel;
    use sequent_core::types::templates::{AudienceSelection, ChannelSelection, SendTemplateBody};
    use serde_json::{json, Map};
    use std::collections::BTreeMap;
    use std::collections::HashMap;

    fn voter(attributes: &[(&str, &[&str])]) -> User {
        User {
            id: Some("voter-1".to_string()),
            email: Some("voter@example.org".to_string()),
            attributes: Some(
                attributes
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.iter().map(|s| s.to_string()).collect()))
                    .collect(),
            ),
            area: Some(UserArea {
                id: Some("area-1".to_string()),
                name: None,
            }),
            ..User::default()
        }
    }

    #[test]
    fn recipients_take_each_channel_address_from_the_voter() {
        let user = voter(&[
            ("sequent.read-only.mobile-number", &["+639171234567"]),
            ("sequent.read-only.whatsapp-number", &["+639181234567"]),
            ("sequent.read-only.messenger-id", &["6543210"]),
            (
                "sequent.read-only.verified-channels",
                &["WHATSAPP", "EMAIL", "NOT_A_CHANNEL"],
            ),
        ]);
        let elections = HashMap::from([("area-1".to_string(), vec!["election-1".to_string()])]);
        let recipient = voter_recipient(&user, &elections);
        assert_eq!(
            recipient.destinations,
            BTreeMap::from([
                (MessageChannel::EMAIL, "voter@example.org".to_string()),
                (MessageChannel::SMS, "+639171234567".to_string()),
                (MessageChannel::WHATSAPP, "+639181234567".to_string()),
                (MessageChannel::MESSENGER, "6543210".to_string()),
            ])
        );
        assert_eq!(
            recipient.verified,
            vec![MessageChannel::EMAIL, MessageChannel::WHATSAPP]
        );
        assert_eq!(recipient.election_ids, vec!["election-1".to_string()]);
    }

    #[test]
    fn voters_without_verified_channels_keep_email_and_sms() {
        let user = voter(&[("sequent.read-only.mobile-number", &["+639171234567"])]);
        let recipient = voter_recipient(&user, &HashMap::new());
        assert_eq!(
            recipient.verified,
            vec![MessageChannel::EMAIL, MessageChannel::SMS]
        );
    }

    #[test]
    fn the_first_channel_follows_the_selection_policy() {
        let user = voter(&[("sequent.read-only.message-channel", &["VIBER"])]);
        assert_eq!(
            first_channel(
                ChannelSelection::SINGLE_CHANNEL,
                Some(MessageChannel::SMS),
                &user
            ),
            Some(MessageChannel::SMS)
        );
        assert_eq!(
            first_channel(
                ChannelSelection::VOTER_PREFERENCE,
                Some(MessageChannel::SMS),
                &user
            ),
            Some(MessageChannel::VIBER)
        );
        let no_preference = voter(&[]);
        assert_eq!(
            first_channel(
                ChannelSelection::VOTER_PREFERENCE,
                Some(MessageChannel::SMS),
                &no_preference
            ),
            Some(MessageChannel::SMS)
        );
        assert_eq!(
            first_channel(ChannelSelection::VOTER_PREFERENCE, None, &no_preference),
            Some(MessageChannel::EMAIL)
        );
        assert_eq!(
            first_channel(ChannelSelection::SINGLE_CHANNEL, None, &user),
            None
        );
    }

    #[test]
    fn contents_are_rendered_for_every_configured_channel() {
        let body: SendTemplateBody = serde_json::from_value(json!({
            "audience_selection": "ALL_USERS",
            "communication_method": "WHATSAPP",
            "email": {"subject": "Hi {{user.first_name}}", "plaintext_body": "Vote, {{user.first_name}}", "html_body": null},
            "sms": {"message": "Vote now, {{user.first_name}}"},
            "whatsapp": {"message": "Hello {{user.first_name}}", "parameters": ["{{user.first_name}}", "May 12"]},
        }))
        .unwrap();
        let mut variables = Map::new();
        variables.insert("user".to_string(), json!({"first_name": "Ana"}));
        let contents = render_contents(&body, &variables).unwrap();
        assert_eq!(
            contents[&MessageChannel::EMAIL].subject.as_deref(),
            Some("Hi Ana")
        );
        assert_eq!(contents[&MessageChannel::SMS].text, "Vote now, Ana");
        assert_eq!(contents[&MessageChannel::WHATSAPP].text, "Hello Ana");
        assert_eq!(
            contents[&MessageChannel::WHATSAPP].template_parameters,
            vec!["Ana".to_string(), "May 12".to_string()]
        );
        assert!(!contents.contains_key(&MessageChannel::VIBER));
    }

    #[test]
    fn election_statistics_use_each_election_metrics() {
        let unit = |counts: &[(MessageChannel, i64)]| MetricsUnit {
            sent: counts.iter().copied().collect(),
        };
        let metrics = Metrics {
            election_event: unit(&[(MessageChannel::EMAIL, 5), (MessageChannel::SMS, 3)]),
            metrics_by_election_id: HashMap::from([
                (
                    "election-a".to_string(),
                    unit(&[(MessageChannel::EMAIL, 4)]),
                ),
                (
                    "election-b".to_string(),
                    unit(&[(MessageChannel::EMAIL, 1), (MessageChannel::VIBER, 3)]),
                ),
                ("election-c".to_string(), MetricsUnit::default()),
            ]),
        };

        let mut increments = election_statistics_increments(&metrics);
        increments.sort();

        let counters = |pairs: &[(&str, i64)]| -> BTreeMap<String, i64> {
            pairs.iter().map(|(k, v)| (k.to_string(), *v)).collect()
        };
        assert_eq!(
            increments,
            vec![
                ("election-a", counters(&[("num_emails_sent", 4)])),
                (
                    "election-b",
                    counters(&[("num_emails_sent", 1), ("num_viber_sent", 3)])
                ),
            ]
        );
    }

    #[test]
    fn metrics_count_each_channel_under_its_statistics_key() {
        let mut unit = MetricsUnit::default();
        assert!(unit.is_empty());
        unit.record(MessageChannel::WHATSAPP);
        unit.record(MessageChannel::WHATSAPP);
        unit.record(MessageChannel::MESSENGER);
        assert_eq!(
            unit.statistics_increments(),
            BTreeMap::from([
                ("num_messenger_sent".to_string(), 1),
                ("num_whatsapp_sent".to_string(), 2),
            ])
        );
    }

    #[test]
    fn selected_audience_requires_voter_ids() {
        assert!(audience_user_ids(&AudienceSelection::SELECTED, &None).is_err());
        assert!(audience_user_ids(&AudienceSelection::SELECTED, &Some(vec![])).is_err());
        assert_eq!(
            audience_user_ids(
                &AudienceSelection::SELECTED,
                &Some(vec!["voter-1".to_string()])
            )
            .expect("selected voters"),
            Some(vec!["voter-1".to_string()])
        );
    }

    #[test]
    fn non_selected_audiences_ignore_voter_ids() {
        for selection in [
            AudienceSelection::ALL_USERS,
            AudienceSelection::NOT_VOTED,
            AudienceSelection::VOTED,
        ] {
            assert_eq!(
                audience_user_ids(&selection, &Some(vec!["voter-1".to_string()]))
                    .expect("audience"),
                None
            );
        }
    }

    #[test]
    fn get_variables_exposes_dynamic_and_multivalued_user_attributes() {
        let user = User {
            username: Some("canonical-user".to_string()),
            attributes: Some(HashMap::from([
                (
                    "dateOfBirth".to_string(),
                    vec!["2000-01-01".to_string(), "ignored-first-value".to_string()],
                ),
                (
                    "username".to_string(),
                    vec!["untrusted-collision".to_string()],
                ),
                ("empty".to_string(), Vec::new()),
            ])),
            ..User::default()
        };

        let variables = get_variables(&user, None, "tenant-id".to_string(), AuthAction::Login)
            .expect("variables should be generated");

        assert_eq!(variables["user"]["username"], json!("canonical-user"));
        assert_eq!(variables["user"]["dateOfBirth"], json!("2000-01-01"));
        assert!(variables["user"].get("empty").is_none());
        assert_eq!(
            variables["user"]["attributes"]["dateOfBirth"],
            json!(["2000-01-01", "ignored-first-value"])
        );
        assert_eq!(
            variables["user"]["attributes"]["username"],
            json!(["untrusted-collision"])
        );
        assert_eq!(variables["user"]["attributes"]["empty"], json!([]));
    }

    #[test]
    fn delivery_audit_message_keeps_delivery_metadata_without_rendered_bodies() {
        let email = delivery_audit_message("email", "voter@example.com");

        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&email).expect("valid audit JSON"),
            json!({
                "channel": "email",
                "receiver": "voter@example.com",
            })
        );
        assert!(!email.contains("login_hint__"));

        let sms = delivery_audit_message("sms", "+34600000000");

        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&sms).expect("valid audit JSON"),
            json!({"channel": "sms", "receiver": "+34600000000"})
        );
    }
}
