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
use crate::services::reports::template_time::{
    insert_template_time_variables, NotificationTimeContext,
};
use crate::services::users::{list_users, list_users_with_vote_info, ListUsersFilter};
use crate::services::voter_secret_attributes::{
    decrypt_user_attributes, get_secret_attribute_config, strip_undeclared_secret_attributes,
};
use crate::types::error::Result;

use crate::services::database::{get_hasura_pool, get_keycloak_pool, PgConfig};
use crate::types::error::Error;
use deadpool_postgres::{Client as DbClient, Transaction};

use crate::postgres::messaging::{mark_logged, MessageRecord};
use crate::services::application::application_retry;
use crate::services::messaging::audit::{AttemptOutcome, MessageKind};
use crate::services::messaging::dispatch::{
    bulk_send_body, deliver, record_without_channel, retry_refusal, Delivery, DeliveryResult,
    Dispatcher, ProviderTemplate, Recipient,
};
use crate::services::messaging::reconcile::{RetryContent, RetryRenderer};
use anyhow::{anyhow, Context};
use async_trait::async_trait;
use aws_sdk_sesv2::types::{Body, Content, Destination, EmailContent, Message as AwsMessage};
use aws_sdk_sesv2::Client as AwsSesClient;
use aws_sdk_sns::{types::MessageAttributeValue, Client as AwsSnsClient};
use celery::error::TaskError;
use deadpool_postgres::Pool;
use lettre::message::MultiPart;
use lettre::Message;
use sequent_core::serialization::deserialize_with_path::*;
use sequent_core::services::generate_urls::get_auth_url;
use sequent_core::services::generate_urls::AuthAction;
use sequent_core::services::keycloak::{get_event_realm, get_tenant_realm};
use sequent_core::services::translations::Name;
use sequent_core::services::{keycloak, reports};
use sequent_core::types::hasura::core::ElectionEvent;
use sequent_core::types::keycloak::{User, UserArea, AREA_ID_ATTR_NAME, LOCALE_ATTR_NAME};
use sequent_core::types::messaging::{
    MessageChannel, MessageContent, MessagePurpose, MessageReason,
};
use sequent_core::types::templates::{
    AudienceSelection, EmailConfig, SendTemplateBody, SmsConfig, TemplateMethod,
};
use sequent_core::types::voter_channel::voter_channel;
use sequent_core::util::aws::get_from_env_aws_config;
use serde_json::json;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::default::Default;
use std::str::FromStr;
use std::sync::Arc;
use strand::info;
use tracing::{event, info, instrument, Level};
use uuid::Uuid;

#[instrument(skip_all, err)]
pub(crate) fn get_variables(
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
/// voter: an empty selection would otherwise reach the whole realm. Harvest
/// refuses it when the send is scheduled; this is the same check.
fn audience_user_ids(
    audience_selection: &AudienceSelection,
    audience_voter_ids: &Option<Vec<String>>,
) -> Result<Option<Vec<String>>> {
    match audience_selection {
        AudienceSelection::SELECTED => match audience_voter_ids {
            Some(ids) if ids.iter().any(|id| !id.trim().is_empty()) => Ok(Some(ids.clone())),
            _ => Err(Error::String(
                MessageReason::SELECTED_WITHOUT_VOTERS.to_string(),
            )),
        },
        _ => Ok(None),
    }
}

pub(crate) fn first_attribute<'a>(user: &'a User, name: &str) -> Option<&'a String> {
    user.attributes
        .as_ref()
        .and_then(|attributes| attributes.get(name))
        .and_then(|values| values.first())
        .filter(|value| !value.is_empty())
}

/// The voter's elections, for per-election (Post) channel restrictions.
fn voter_elections(user: &User, elections_by_area: &HashMap<String, Vec<String>>) -> Vec<String> {
    user.area
        .as_ref()
        .and_then(|area| area.id.as_ref())
        .and_then(|area_id| elections_by_area.get(area_id))
        .cloned()
        .unwrap_or_default()
}

/// The voter's one channel and destination (`voter_channel`), or `None`
/// when the record has none. The send's own channel choice, if any, is
/// ignored: there is no override.
fn voter_recipient(
    user: &User,
    elections_by_area: &HashMap<String, Vec<String>>,
) -> Option<Recipient> {
    let found = voter_channel(user.email.as_deref(), user.attributes.as_ref())?;
    Some(Recipient::of_voter(
        user.id.clone(),
        found,
        voter_elections(user, elections_by_area),
    ))
}

/// The approved template the send itself names for `channel`.
fn provider_template(body: &SendTemplateBody, channel: MessageChannel) -> Option<ProviderTemplate> {
    provider_templates(body).remove(&channel)
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
pub(crate) fn render_contents(
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

/// Writes the attempts a send recorded to the electoral log, signed for the
/// administrator who sent it. Only attempts recorded now: a replayed job
/// logs nothing new.
#[allow(clippy::too_many_arguments)]
async fn log_recorded_attempts(
    hasura_transaction: &Transaction<'_>,
    ledger: &mut DbClient,
    election_event: &Option<ElectionEvent>,
    recorded: &[AttemptOutcome],
    username: Option<String>,
    tenant_id: &str,
    admin_id: &str,
    area_id: Option<String>,
) {
    for outcome in recorded {
        let posted = on_success_send_message(
            hasura_transaction,
            election_event.clone(),
            outcome.voter_id.clone(),
            username.clone(),
            &outcome.log_body(),
            tenant_id,
            admin_id,
            area_id.clone(),
        )
        .await;
        match posted {
            // Handed to the log: the sweep won't write it again.
            Ok(()) if election_event.is_some() => {
                let marked = async {
                    let tx = ledger.transaction().await?;
                    mark_logged(&tx, &[outcome.message_id]).await?;
                    tx.commit().await?;
                    anyhow::Ok(())
                }
                .await;
                if let Err(error) = marked {
                    event!(Level::WARN, "could not mark the attempt logged: {error:?}");
                }
            }
            Ok(()) => {}
            // Left unlogged: the messaging sweep writes it later.
            Err(error) => event!(
                Level::ERROR,
                "Error writing the message to the electoral log: {error:?}"
            ),
        }
    }
}

/// The logical key of a voter's notice in a bulk send.
fn bulk_logical_key(send_id: &str, voter_id: &str) -> String {
    format!("send-template:{send_id}:{voter_id}")
}

/// The voter's notice of a bulk send, rendered on their channel.
fn voter_delivery(
    body: &SendTemplateBody,
    tenant_id: Uuid,
    election_event_id: Option<Uuid>,
    recipient: Recipient,
    contents: &BTreeMap<MessageChannel, MessageContent>,
    language: Option<String>,
    logical_key: String,
) -> Delivery {
    let channel = recipient.channel;
    Delivery {
        tenant_id,
        election_event_id,
        purpose: MessagePurpose::NOTICE,
        recipient,
        content: contents.get(&channel).cloned(),
        language,
        template_alias: body.alias.clone(),
        logical_key,
        expires_at: None,
        account_id: None,
        template_key: body.alias.clone(),
        provider_template: provider_template(body, channel),
    }
}

/// The secret attributes a send's template uses: those named inside a
/// Handlebars expression such as `{{user.reference}}` or
/// `{{lookup user.attributes "reference"}}`. A name that only appears in
/// prose isn't used. The same rule as the Admin Portal's
/// `getReferencedSecretAttributeNames`.
pub fn referenced_secret_attribute_names(
    body: &SendTemplateBody,
    secret_names: &HashSet<String>,
) -> Vec<String> {
    let mut texts: Vec<&str> = vec![];
    if let Some(email) = &body.email {
        texts.extend([email.subject.as_str(), email.plaintext_body.as_str()]);
        texts.extend(email.html_body.as_deref());
    }
    if let Some(sms) = &body.sms {
        texts.push(sms.message.as_str());
    }
    for config in [&body.whatsapp, &body.viber, &body.messenger]
        .into_iter()
        .flatten()
    {
        texts.push(config.message.as_str());
        texts.extend(config.parameters.iter().map(String::as_str));
    }
    let expressions: Vec<&str> = texts
        .iter()
        .flat_map(|text| {
            text.split("{{")
                .skip(1)
                .filter_map(|rest| rest.split_once("}}").map(|(inside, _)| inside))
        })
        .collect();
    let is_name_char = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '-';
    let mut used: Vec<String> = secret_names
        .iter()
        .filter(|name| {
            !name.is_empty()
                && expressions.iter().any(|expression| {
                    expression.match_indices(name.as_str()).any(|(at, _)| {
                        let before = expression[..at].chars().next_back();
                        let after = expression[at + name.len()..].chars().next();
                        !before.is_some_and(is_name_char) && !after.is_some_and(is_name_char)
                    })
                })
        })
        .cloned()
        .collect();
    used.sort();
    used
}

/// Where a retry reads the voter's current record and the event's
/// encrypted attribute names.
#[async_trait]
pub trait RetryDirectory: Send + Sync {
    async fn voter(
        &self,
        hasura_transaction: &Transaction<'_>,
        tenant_id: &str,
        election_event_id: Option<&str>,
        voter_id: &str,
    ) -> anyhow::Result<Option<User>>;

    async fn secret_attribute_names(
        &self,
        tenant_id: &str,
        election_event_id: &str,
    ) -> anyhow::Result<HashSet<String>>;
}

/// A voter's current record, from Keycloak's database.
pub async fn find_voter(
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: Option<&str>,
    voter_id: &str,
) -> anyhow::Result<Option<User>> {
    let realm = match election_event_id {
        Some(event_id) => get_event_realm(tenant_id, event_id),
        None => get_tenant_realm(tenant_id),
    };
    let (users, _) = list_users(
        hasura_transaction,
        keycloak_transaction,
        ListUsersFilter {
            tenant_id: tenant_id.to_string(),
            election_event_id: election_event_id.map(str::to_string),
            election_id: None,
            area_id: None,
            realm,
            search: None,
            first_name: None,
            last_name: None,
            username: None,
            email: None,
            limit: Some(1),
            offset: None,
            user_ids: Some(vec![voter_id.to_string()]),
            attributes: None,
            enabled: None,
            email_verified: None,
            sort: None,
            has_voted: None,
            authorized_to_election_alias: None,
        },
    )
    .await?;
    Ok(users.into_iter().next())
}

/// The production directory: Keycloak's database and user profile.
pub struct KeycloakDirectory;

#[async_trait]
impl RetryDirectory for KeycloakDirectory {
    async fn voter(
        &self,
        hasura_transaction: &Transaction<'_>,
        tenant_id: &str,
        election_event_id: Option<&str>,
        voter_id: &str,
    ) -> anyhow::Result<Option<User>> {
        let mut keycloak_client: DbClient = get_keycloak_pool().await.get().await?;
        let keycloak_transaction = keycloak_client.transaction().await?;
        let voter = find_voter(
            hasura_transaction,
            &keycloak_transaction,
            tenant_id,
            election_event_id,
            voter_id,
        )
        .await?;
        keycloak_transaction.commit().await?;
        Ok(voter)
    }

    async fn secret_attribute_names(
        &self,
        tenant_id: &str,
        election_event_id: &str,
    ) -> anyhow::Result<HashSet<String>> {
        Ok(get_secret_attribute_config(tenant_id, election_event_id)
            .await
            .map_err(|error| anyhow!("{error:#}"))?
            .redacted_names()
            .clone())
    }
}

/// Renders a due retry again by where the message comes from
/// ([`retry_refusal`]): a bulk notice from what the administrator sent
/// (the stored send), an application result from the application, both
/// for the voter's current record. Everything else is refused with its
/// reason. The ledger keeps no content.
pub struct RetryByOrigin<D: RetryDirectory> {
    pub pool: Arc<Pool>,
    pub directory: D,
}

impl RetryByOrigin<KeycloakDirectory> {
    pub async fn production() -> Self {
        RetryByOrigin {
            pool: get_hasura_pool().await,
            directory: KeycloakDirectory,
        }
    }
}

impl<D: RetryDirectory> RetryByOrigin<D> {
    async fn bulk_notice(
        &self,
        hasura_transaction: &Transaction<'_>,
        record: &MessageRecord,
        logical_key: &str,
    ) -> anyhow::Result<RetryContent> {
        let refused = |reason| Ok(RetryContent::Refused(reason));
        let Some(body) = bulk_send_body(hasura_transaction, &record.tenant_id, logical_key).await?
        else {
            return refused(MessageReason::NO_STORED_TEMPLATE);
        };
        let Some(voter_id) = record.voter_id.as_deref() else {
            return refused(MessageReason::NO_CONTACT_CHANNEL);
        };
        let tenant_id = record.tenant_id.to_string();
        let election_event_id = record.election_event_id.map(|id| id.to_string());
        // Ciphertexts of encrypted attributes never reach a template, and a
        // send that uses them isn't sent again.
        let secret_names = match &election_event_id {
            Some(event_id) => {
                self.directory
                    .secret_attribute_names(&tenant_id, event_id)
                    .await?
            }
            None => HashSet::new(),
        };
        if !referenced_secret_attribute_names(&body, &secret_names).is_empty() {
            return refused(MessageReason::PROTECTED_FIELDS);
        }
        let Some(user) = self
            .directory
            .voter(
                hasura_transaction,
                &tenant_id,
                election_event_id.as_deref(),
                voter_id,
            )
            .await?
        else {
            return refused(MessageReason::NO_CONTACT_CHANNEL);
        };
        let election_event = match &election_event_id {
            Some(event_id) => {
                get_election_event_by_id_if_exist(hasura_transaction, &tenant_id, event_id).await?
            }
            None => None,
        };
        let elections_by_area = match &election_event_id {
            Some(event_id) => {
                get_elections_by_area(hasura_transaction, &tenant_id, event_id).await?
            }
            None => HashMap::new(),
        };
        let Some(recipient) = voter_recipient(&user, &elections_by_area) else {
            return refused(MessageReason::NO_CONTACT_CHANNEL);
        };
        let time_context = match &election_event {
            Some(election_event) => {
                NotificationTimeContext::load(hasura_transaction, &tenant_id, election_event)
                    .await?
            }
            None => NotificationTimeContext::default(),
        };
        let mut render_user = user.clone();
        strip_undeclared_secret_attributes(&mut render_user, &secret_names, &HashSet::new());
        let mut variables = get_variables(
            &render_user,
            election_event,
            tenant_id.clone(),
            AuthAction::Login,
        )
        .map_err(|error| anyhow!("{error:?}"))?;
        insert_template_time_variables(
            &mut variables,
            time_context
                .variables_for(first_attribute(&user, AREA_ID_ATTR_NAME).map(String::as_str)),
        );
        let contents = render_contents(&body, &variables).map_err(|error| anyhow!("{error:?}"))?;
        Ok(RetryContent::Message(Box::new(voter_delivery(
            &body,
            record.tenant_id,
            record.election_event_id,
            recipient,
            &contents,
            first_attribute(&user, LOCALE_ATTR_NAME).cloned(),
            logical_key.to_string(),
        ))))
    }
}

#[async_trait]
impl<D: RetryDirectory> RetryRenderer for RetryByOrigin<D> {
    async fn render(&self, record: &MessageRecord) -> anyhow::Result<RetryContent> {
        let Some(logical_key) = record.logical_key.as_deref() else {
            return Ok(RetryContent::Refused(MessageReason::NO_STORED_TEMPLATE));
        };
        let mut client: DbClient = self.pool.get().await?;
        let hasura_transaction = client.transaction().await?;
        let content = match retry_refusal(
            &hasura_transaction,
            &record.tenant_id,
            Some(logical_key),
            record.template_alias.as_deref(),
        )
        .await?
        {
            Some(reason) => RetryContent::Refused(reason),
            None => match MessageKind::of(logical_key) {
                MessageKind::BULK_NOTICE => {
                    self.bulk_notice(&hasura_transaction, record, logical_key)
                        .await?
                }
                MessageKind::APPLICATION_RESULT => {
                    application_retry(&hasura_transaction, record, &self.directory).await?
                }
                _ => RetryContent::Refused(MessageReason::NO_STORED_TEMPLATE),
            },
        };
        hasura_transaction.commit().await?;
        Ok(content)
    }
}

/// What one voter's notice of a bulk send adds to the statistics and the
/// electoral log.
#[derive(Debug, Clone, PartialEq)]
pub struct VoterNotice {
    /// The channel to count the notice under: only when this call recorded
    /// it as sent. A replay counts nothing.
    pub counted: Option<MessageChannel>,
    /// The attempts this call recorded, to log. A replay logs nothing.
    pub to_log: Vec<AttemptOutcome>,
}

impl From<DeliveryResult> for VoterNotice {
    fn from(result: DeliveryResult) -> Self {
        VoterNotice {
            counted: result.newly_sent().then_some(result.channel).flatten(),
            to_log: result.recorded,
        }
    }
}

/// Sends one voter's notice of a bulk send, as `send_template` does.
pub async fn send_voter_notice(
    client: &mut DbClient,
    dispatcher: &Dispatcher,
    delivery: &Delivery,
) -> anyhow::Result<VoterNotice> {
    Ok(deliver(client, dispatcher, delivery).await?.into())
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
    // Each voter's election, its opening and close, and the zones (design §7).
    let time_context = match election_event {
        Some(ref election_event) => {
            NotificationTimeContext::load(&hasura_transaction, &tenant_id, election_event)
                .await
                .with_context(|| "Error loading the notification times")?
        }
        None => NotificationTimeContext::default(),
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
            let Some(voter_id) = user.id.clone() else {
                continue;
            };
            let logical_key = bulk_logical_key(&send_id, &voter_id);
            let area_id = user.area.as_ref().and_then(|area| area.id.clone());
            let language = first_attribute(user, LOCALE_ATTR_NAME).cloned();
            let Some(recipient) = voter_recipient(user, &elections_by_area) else {
                // One FAILED row, so staff see the voter wasn't reached.
                match record_without_channel(
                    &mut delivery_client,
                    tenant_uuid,
                    event_uuid,
                    Some(voter_id.clone()),
                    MessagePurpose::NOTICE,
                    body.alias.clone(),
                    language,
                    &logical_key,
                )
                .await
                {
                    Ok(result) => {
                        log_recorded_attempts(
                            &hasura_transaction,
                            &mut delivery_client,
                            &election_event,
                            &result.recorded,
                            user.username.clone(),
                            &tenant_id,
                            &admin_id,
                            area_id,
                        )
                        .await
                    }
                    Err(error) => {
                        event!(Level::ERROR, "voter {voter_id}: {error:?}, continuing..")
                    }
                }
                continue;
            };
            let mut variables = get_variables(
                &render_user,
                election_event.clone(),
                tenant_id.clone(),
                AuthAction::Login,
            )?;
            insert_template_time_variables(
                &mut variables,
                time_context
                    .variables_for(first_attribute(user, AREA_ID_ATTR_NAME).map(String::as_str)),
            );
            let contents = match render_contents(&body, &variables) {
                Ok(contents) => contents,
                Err(error) => {
                    event!(Level::ERROR, "voter {voter_id}: {error:?}, continuing..");
                    continue;
                }
            };
            let delivery = voter_delivery(
                &body,
                tenant_uuid,
                event_uuid,
                recipient,
                &contents,
                language,
                logical_key,
            );
            let notice = match send_voter_notice(&mut delivery_client, dispatcher, &delivery).await
            {
                Ok(notice) => notice,
                Err(error) => {
                    event!(Level::ERROR, "voter {voter_id}: {error:?}, continuing..");
                    continue;
                }
            };
            // Only what this run recorded counts: a replayed job neither
            // counts a voter again nor logs them again. A transient failure
            // is retried on the same channel by the messaging sweep.
            if let Some(channel) = notice.counted {
                update_metrics(&mut metrics, &elections_by_area, user, channel, true);
            }
            log_recorded_attempts(
                &hasura_transaction,
                &mut delivery_client,
                &election_event,
                &notice.to_log,
                user.username.clone(),
                &tenant_id,
                &admin_id,
                area_id,
            )
            .await;
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

#[cfg(test)]
mod tests {
    use super::{
        audience_user_ids, bulk_logical_key, election_statistics_increments, get_variables,
        referenced_secret_attribute_names, render_contents, voter_delivery, voter_recipient,
        Metrics, MetricsUnit,
    };
    use sequent_core::services::generate_urls::AuthAction;
    use sequent_core::types::keycloak::{User, UserArea};
    use sequent_core::types::messaging::{MessageChannel, MessageContent};
    use sequent_core::types::templates::{AudienceSelection, SendTemplateBody};
    use serde_json::{json, Map};
    use std::collections::BTreeMap;
    use std::collections::HashMap;
    use uuid::Uuid;

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
    fn each_voter_gets_their_one_recorded_channel() {
        let user = voter(&[
            ("sequent.read-only.mobile-number", &["+639171234567"]),
            ("sequent.read-only.message-channel", &["WHATSAPP"]),
            ("sequent.read-only.message-number", &["+639181234567"]),
        ]);
        let elections = HashMap::from([("area-1".to_string(), vec!["election-1".to_string()])]);
        let recipient = voter_recipient(&user, &elections).expect("a channel");
        assert_eq!(recipient.channel, MessageChannel::WHATSAPP);
        assert_eq!(recipient.destination, "+639181234567");
        assert_eq!(recipient.election_ids, vec!["election-1".to_string()]);
    }

    /// R7-02: a Messenger voter's notice (and its retry, rendered again
    /// from their record) carries their Page, so a send through an
    /// account of another Page is refused.
    #[test]
    fn a_messenger_voter_carries_their_page() {
        let user = voter(&[
            ("sequent.read-only.message-channel", &["MESSENGER"]),
            ("sequent.read-only.messenger-id", &["6543210"]),
            ("sequent.read-only.messenger-page", &["page-old"]),
        ]);
        let recipient = voter_recipient(&user, &HashMap::new()).expect("a channel");
        assert_eq!(recipient.channel, MessageChannel::MESSENGER);
        assert_eq!(recipient.destination, "6543210");
        assert_eq!(recipient.messenger_page.as_deref(), Some("page-old"));
    }

    #[test]
    fn legacy_voters_have_email_else_sms_and_some_have_no_channel() {
        let user = voter(&[("sequent.read-only.mobile-number", &["+639171234567"])]);
        assert_eq!(
            voter_recipient(&user, &HashMap::new()).unwrap().channel,
            MessageChannel::EMAIL
        );
        let no_email = User {
            email: None,
            ..user.clone()
        };
        assert_eq!(
            voter_recipient(&no_email, &HashMap::new()).unwrap().channel,
            MessageChannel::SMS
        );
        let nothing = User {
            email: None,
            attributes: None,
            ..user
        };
        assert_eq!(voter_recipient(&nothing, &HashMap::new()), None);
    }

    #[test]
    fn the_send_channel_choice_is_ignored() {
        // The first release's override: every voter on SMS.
        let body: SendTemplateBody = serde_json::from_value(json!({
            "audience_selection": "ALL_USERS",
            "communication_method": "SMS",
            "channel_selection": "SINGLE_CHANNEL",
            "sms": {"message": "Vote"},
            "whatsapp": {"message": "Vote", "provider_template": "reminder"},
        }))
        .unwrap();
        let user = voter(&[
            ("sequent.read-only.message-channel", &["WHATSAPP"]),
            ("sequent.read-only.message-number", &["+639181234567"]),
        ]);
        let recipient = voter_recipient(&user, &HashMap::new()).unwrap();
        let contents = BTreeMap::from([
            (MessageChannel::SMS, content("Vote")),
            (MessageChannel::WHATSAPP, content("Vote")),
        ]);
        let delivery = voter_delivery(
            &body,
            Uuid::nil(),
            None,
            recipient,
            &contents,
            None,
            bulk_logical_key("send-1", "voter-1"),
        );
        assert_eq!(delivery.recipient.channel, MessageChannel::WHATSAPP);
        assert_eq!(delivery.content, Some(content("Vote")));
        assert_eq!(
            delivery.provider_template.map(|t| t.name),
            Some("reminder".to_string())
        );
        assert_eq!(delivery.logical_key, "send-template:send-1:voter-1");
    }

    #[test]
    fn a_voter_without_content_for_their_channel_gets_none() {
        let body: SendTemplateBody = serde_json::from_value(json!({
            "audience_selection": "ALL_USERS",
            "sms": {"message": "Vote"},
        }))
        .unwrap();
        let user = voter(&[
            ("sequent.read-only.message-channel", &["VIBER"]),
            ("sequent.read-only.message-number", &["+639181234567"]),
        ]);
        let delivery = voter_delivery(
            &body,
            Uuid::nil(),
            None,
            voter_recipient(&user, &HashMap::new()).unwrap(),
            &BTreeMap::from([(MessageChannel::SMS, content("Vote"))]),
            None,
            bulk_logical_key("send-1", "voter-1"),
        );
        assert_eq!(delivery.content, None);
    }

    fn content(text: &str) -> MessageContent {
        MessageContent {
            subject: None,
            text: text.to_string(),
            html: None,
            template_parameters: vec![],
            code: None,
        }
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
    fn secret_attributes_count_only_inside_template_expressions() {
        let body: SendTemplateBody = serde_json::from_value(json!({
            "audience_selection": "ALL_USERS",
            "sms": {"message": "Your pin-code: {{user.pin}}"},
            "whatsapp": {"message": "x", "parameters": ["{{lookup user.attributes \"ref\"}}"]},
            "email": {"subject": "About your pin", "plaintext_body": "{{user.pinned}}",
                      "html_body": null},
        }))
        .unwrap();
        let names = |list: &[&str]| list.iter().map(|n| n.to_string()).collect();
        assert_eq!(
            referenced_secret_attribute_names(&body, &names(&["pin", "ref", "other"])),
            vec!["pin".to_string(), "ref".to_string()]
        );
        // Prose and longer names don't count.
        assert!(referenced_secret_attribute_names(&body, &names(&["code", "pinn"])).is_empty());
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
}
