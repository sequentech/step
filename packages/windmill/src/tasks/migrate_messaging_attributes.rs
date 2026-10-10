// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! One channel per voter (INFRA-MSG §2.5): moves event realms and their
//! voters from the first messaging release's shape to the ticket's.
//!
//! In every event realm whose user profile has the channel attribute:
//! - the profile gains `message-number`, and the channel is labelled
//!   "Contact channel";
//! - the contact change required action `message-channel-ra` is registered,
//!   and it and the old `messaging-app-otp-ra` require a sign-in in the last
//!   300 s (`max_auth_age`); the old one stays, since voters may have it
//!   pending;
//! - every message OTP step sends its code to the voter's one channel
//!   (`messageCourierAttribute` `CHOSEN`, whatever courier it named: R7-01);
//! - the reset code (`reset.send.otp.tlf`) page reveals nothing about the
//!   account (`code-page-disclosure` `NEUTRAL`, unless the realm chose its
//!   own); every other message OTP step states its code page policy too:
//!   `NEUTRAL` in the reset-credentials flow, `NAMED` at sign-in (the
//!   default became `NEUTRAL`), enrollment steps left alone;
//! - every voter with a first-release record (`verified-channels`) is
//!   rewritten to the ticket's attributes, and the first-release attributes
//!   are removed.
//!
//! Nothing is changed while Keycloak lacks `message-channel-ra` (an image
//! older than this release): the task waits for it, re-running with a wait
//! of up to 30 minutes.
//!
//! Already migrated realms and voters are unchanged, so every run is safe.
//! Every change and every voter's outcome is logged, never a number, email
//! or Page-scoped ID.

use crate::postgres::election_event::get_all_tenant_election_events;
use crate::postgres::tenant::get_tenant_ids;
use crate::services::database::get_hasura_pool;
use crate::tasks::migrate_realm_permissions::retry_countdown;
use anyhow::{anyhow, Context, Result};
use celery::error::TaskError;
use celery::task::{Task, TaskResult};
use deadpool_postgres::Client as DbClient;
use keycloak::types::{
    AuthenticationExecutionInfoRepresentation, AuthenticatorConfigInfoRepresentation,
    AuthenticatorConfigRepresentation, RequiredActionProviderRepresentation,
};
use keycloak::{KeycloakError, KeycloakTokenSupplier};
use sequent_core::services::keycloak::{get_event_realm, KeycloakAdminClient, PubKeycloakAdmin};
use sequent_core::types::keycloak::{
    MESSAGE_CHANNEL_ATTR_NAME, MESSAGE_CONSENT_ATTR_NAME, MESSAGE_NUMBER_ATTR_NAME,
    MESSENGER_ID_ATTR_NAME, MESSENGER_PAGE_ATTR_NAME, VERIFIED_CHANNELS_ATTR_NAME,
    VIBER_NUMBER_ATTR_NAME, WHATSAPP_NUMBER_ATTR_NAME,
};
use sequent_core::types::messaging::MessageChannel;
use sequent_core::types::voter_channel::{voter_channel, VoterChannelSource, CHANNEL_ORDER};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::str::FromStr;
use tracing::{error, info, instrument, warn};

/// The contact change required action ("Change how we contact you").
pub const MESSAGE_CHANNEL_RA: &str = "message-channel-ra";
const MESSAGE_CHANNEL_RA_NAME: &str = "Change how we contact you";
/// Its first-release id, still provided by the jar as an alias.
pub const MESSAGING_APP_OTP_RA: &str = "messaging-app-otp-ra";
/// Keycloak's required action config key for recent authentication.
pub const MAX_AUTH_AGE_KEY: &str = "max_auth_age";
pub const MAX_AUTH_AGE_SECONDS: &str = "300";
/// The authenticator config of the reset code step.
pub const RESET_CODE_CONFIG_ALIAS: &str = "reset.send.otp.tlf";
const COURIER_KEY: &str = "messageCourierAttribute";
const CHOSEN_COURIER: &str = "CHOSEN";
const DEFERRED_KEY: &str = "deferredUserAttribute";
/// What the reset code page says about the account (the message-otp
/// authenticator's config): `NAMED` names the voter's channel and masked
/// destination, `NEUTRAL` says only "if an account exists, we sent a code
/// to its contact". The reset code page is neutral (review R3-04).
pub const CODE_PAGE_DISCLOSURE_KEY: &str = "code-page-disclosure";
pub const NEUTRAL_DISCLOSURE: &str = "NEUTRAL";
const CONTACT_CHANNEL_LABEL: &str = "Contact channel";
/// The first release's label, replaced; any other label is the realm's own.
const NOTIFICATION_CHANNEL_LABEL: &str = "Notification channel";
const MESSAGE_NUMBER_LABEL: &str = "Messaging number";
/// Users per page when searching by attribute.
const SEARCH_PAGE: i32 = 100;

/// What the migration does with one voter. Logged with every voter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum_macros::Display)]
#[strum(serialize_all = "kebab-case")]
pub enum UserCase {
    /// A first-release record, rewritten to the ticket's attributes.
    Rewritten,
    /// A first-release record whose verified channels have no destination.
    /// Left untouched: the voter's channel can't be told from it.
    NoDestination,
    /// A record already in the ticket's shape, or a legacy one. Untouched.
    AlreadyRecorded,
}

/// The attribute writes for one voter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserPlan {
    pub case: UserCase,
    /// The voter's channel; `None` when it can't be told.
    pub channel: Option<MessageChannel>,
    /// Attributes to send. The user update merges, so an attribute is
    /// removed by sending it with no values, never by leaving it out.
    pub updates: HashMap<String, Vec<String>>,
}

fn first<'a>(attributes: Option<&'a HashMap<String, Vec<String>>>, name: &str) -> Option<&'a str> {
    attributes?.get(name)?.first().map(String::as_str)
}

/// Whether a consent record (`{at, version, organization, channel}`, or the
/// first release's `{time, wording_version, channel}`) is for `channel`.
fn consent_is_for(consent: &str, channel: MessageChannel) -> bool {
    serde_json::from_str::<Value>(consent)
        .ok()
        .and_then(|value| {
            value
                .get("channel")
                .and_then(Value::as_str)
                .and_then(|name| MessageChannel::from_str(name).ok())
        })
        == Some(channel)
}

/// Plans the rewrite of one voter's record, read with the same rules as
/// every other reader ([`voter_channel`]).
pub fn plan_user(
    email: Option<&str>,
    attributes: Option<&HashMap<String, Vec<String>>>,
) -> UserPlan {
    // As in [`voter_channel`], an empty list is no first-release record.
    let first_release = attributes
        .and_then(|attributes| attributes.get(VERIFIED_CHANNELS_ATTR_NAME))
        .is_some_and(|values| !values.is_empty());
    let read = match voter_channel(email, attributes) {
        Some(read) if read.source == VoterChannelSource::VERIFIED_CHANNELS => read,
        // A first-release record the reader gives no channel for has no
        // destination for any channel the voter verified.
        _ if first_release => {
            return UserPlan {
                case: UserCase::NoDestination,
                channel: None,
                updates: HashMap::new(),
            }
        }
        other => {
            return UserPlan {
                case: UserCase::AlreadyRecorded,
                channel: other.map(|read| read.channel),
                updates: HashMap::new(),
            }
        }
    };
    let channel = read.channel;
    let only_if = |condition: bool, value: Option<String>| -> Vec<String> {
        if condition {
            value.into_iter().collect()
        } else {
            vec![]
        }
    };
    let number = matches!(
        channel,
        MessageChannel::WHATSAPP | MessageChannel::VIBER | MessageChannel::SMS
    );
    let messenger = channel == MessageChannel::MESSENGER;
    let mut updates: HashMap<String, Vec<String>> = HashMap::from([
        (
            MESSAGE_CHANNEL_ATTR_NAME.to_string(),
            vec![channel.to_string()],
        ),
        (
            MESSAGE_NUMBER_ATTR_NAME.to_string(),
            only_if(number, Some(read.destination.clone())),
        ),
        (
            MESSENGER_ID_ATTR_NAME.to_string(),
            only_if(messenger, Some(read.destination.clone())),
        ),
        (
            MESSENGER_PAGE_ATTR_NAME.to_string(),
            only_if(messenger, read.messenger_page.clone()),
        ),
        (VERIFIED_CHANNELS_ATTR_NAME.to_string(), vec![]),
        (WHATSAPP_NUMBER_ATTR_NAME.to_string(), vec![]),
        (VIBER_NUMBER_ATTR_NAME.to_string(), vec![]),
    ]);
    if let Some(consent) = first(attributes, MESSAGE_CONSENT_ATTR_NAME) {
        if !consent_is_for(consent, channel) {
            updates.insert(MESSAGE_CONSENT_ATTR_NAME.to_string(), vec![]);
        }
    }
    UserPlan {
        case: UserCase::Rewritten,
        channel: Some(channel),
        updates,
    }
}

/// Merges `updates` into a user's attributes the way the user update does:
/// each named attribute is replaced, and the others are kept. An attribute
/// replaced with no values is removed by Keycloak.
pub fn apply_updates(
    attributes: &mut HashMap<String, Vec<String>>,
    updates: &HashMap<String, Vec<String>>,
) {
    for (name, values) in updates {
        attributes.insert(name.clone(), values.clone());
    }
}

fn profile_attribute_index(profile: &Value, name: &str) -> Option<usize> {
    profile
        .get("attributes")?
        .as_array()?
        .iter()
        .position(|attribute| attribute.get("name").and_then(Value::as_str) == Some(name))
}

/// A realm uses messaging when its user profile has the channel attribute.
pub fn uses_messaging(profile: &Value) -> bool {
    profile_attribute_index(profile, MESSAGE_CHANNEL_ATTR_NAME).is_some()
}

/// Adds `message-number` (defined like `whatsapp-number`) after the channel
/// attribute, and relabels the channel "Contact channel" when it has the
/// first release's label. Returns the changes made; none on a migrated
/// profile.
pub fn patch_profile(profile: &mut Value) -> Vec<String> {
    let mut changes = vec![];
    let Some(channel_index) = profile_attribute_index(profile, MESSAGE_CHANNEL_ATTR_NAME) else {
        return changes;
    };
    let Some(attributes) = profile.get_mut("attributes").and_then(Value::as_array_mut) else {
        return changes;
    };
    let channel = &mut attributes[channel_index];
    let label = channel.get("displayName").and_then(Value::as_str);
    if label.is_none_or(|label| label.is_empty() || label == NOTIFICATION_CHANNEL_LABEL) {
        channel["displayName"] = json!(CONTACT_CHANNEL_LABEL);
        changes.push(format!(
            "labelled {MESSAGE_CHANNEL_ATTR_NAME} \"{CONTACT_CHANNEL_LABEL}\""
        ));
    }
    let has_number = attributes.iter().any(|attribute| {
        attribute.get("name").and_then(Value::as_str) == Some(MESSAGE_NUMBER_ATTR_NAME)
    });
    if !has_number {
        let template = attributes
            .iter()
            .find(|attribute| {
                attribute.get("name").and_then(Value::as_str) == Some(WHATSAPP_NUMBER_ATTR_NAME)
            })
            .cloned()
            .unwrap_or_else(|| {
                let mut channel = attributes[channel_index].clone();
                channel["multivalued"] = json!(false);
                channel
            });
        let mut number = template;
        number["name"] = json!(MESSAGE_NUMBER_ATTR_NAME);
        number["displayName"] = json!(MESSAGE_NUMBER_LABEL);
        attributes.insert(channel_index + 1, number);
        changes.push(format!(
            "added profile attribute {MESSAGE_NUMBER_ATTR_NAME}"
        ));
    }
    changes
}

/// The required action config with recent authentication, or `None` when
/// it already has a `max_auth_age` (a realm's own value is kept).
pub fn max_auth_age_patch(
    current: Option<&HashMap<String, String>>,
) -> Option<HashMap<String, String>> {
    if current
        .and_then(|config| config.get(MAX_AUTH_AGE_KEY))
        .is_some_and(|age| !age.trim().is_empty())
    {
        return None;
    }
    let mut config = current.cloned().unwrap_or_default();
    config.insert(
        MAX_AUTH_AGE_KEY.to_string(),
        MAX_AUTH_AGE_SECONDS.to_string(),
    );
    Some(config)
}

/// The reset code config sending to the voter's one channel with a neutral
/// code page, or `None` when it isn't the reset code config or already
/// does. A realm's own `code-page-disclosure` is kept.
pub fn reset_config_patch(
    config: &AuthenticatorConfigRepresentation,
) -> Option<AuthenticatorConfigRepresentation> {
    if config.alias.as_deref() != Some(RESET_CODE_CONFIG_ALIAS) {
        return None;
    }
    let mut patched = config.clone();
    let values = patched.config.get_or_insert_with(HashMap::new);
    let mut changed = false;
    for (key, value) in [(COURIER_KEY, CHOSEN_COURIER), (DEFERRED_KEY, "false")] {
        if values.get(key).map(String::as_str) != Some(value) {
            values.insert(key.to_string(), value.to_string());
            changed = true;
        }
    }
    if values
        .get(CODE_PAGE_DISCLOSURE_KEY)
        .is_none_or(|value| value.trim().is_empty())
    {
        values.insert(
            CODE_PAGE_DISCLOSURE_KEY.to_string(),
            NEUTRAL_DISCLOSURE.to_string(),
        );
        changed = true;
    }
    changed.then_some(patched)
}

pub const NAMED_DISCLOSURE: &str = "NAMED";

/// The authenticator whose code page the policy applies to.
pub const MESSAGE_OTP_PROVIDER: &str = "message-otp-authenticator";

/// Where a message OTP step is used, which decides its code page policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodeStepRole {
    /// In the realm's reset-credentials flow: anyone can start it with an
    /// identifier, so the page reveals nothing (`NEUTRAL`).
    Reset,
    /// Anywhere else: a sign-in step after the password, where the page keeps
    /// naming the voter's channel (`NAMED`), as it did before the default
    /// became `NEUTRAL`. Enrollment steps (`deferredUserAttribute`) always
    /// name the channel the voter entered and are left alone.
    SignIn,
}

/// The config of a message OTP step with its code page policy stated, or
/// `None` when it already states one (a realm's own choice is kept) or needs
/// none. The reset code config gets [`reset_config_patch`] wherever it is
/// used.
pub fn code_page_patch(
    config: &AuthenticatorConfigRepresentation,
    role: CodeStepRole,
) -> Option<AuthenticatorConfigRepresentation> {
    if config.alias.as_deref() == Some(RESET_CODE_CONFIG_ALIAS) {
        return reset_config_patch(config);
    }
    let values = config.config.clone().unwrap_or_default();
    if values
        .get(CODE_PAGE_DISCLOSURE_KEY)
        .is_some_and(|value| !value.trim().is_empty())
    {
        return None;
    }
    let policy = match role {
        CodeStepRole::Reset => NEUTRAL_DISCLOSURE,
        CodeStepRole::SignIn if values.get(DEFERRED_KEY).map(String::as_str) == Some("true") => {
            return None
        }
        CodeStepRole::SignIn => NAMED_DISCLOSURE,
    };
    let mut patched = config.clone();
    patched
        .config
        .get_or_insert_with(HashMap::new)
        .insert(CODE_PAGE_DISCLOSURE_KEY.to_string(), policy.to_string());
    Some(patched)
}

/// The courier of a step configured to send no code.
const NONE_COURIER: &str = "NONE";

/// Logged for each message OTP step the migration leaves on `NONE`.
pub const NONE_STEP_LEFT: &str = "left NONE: the step sends no code";

/// Whether `config` names the `NONE` courier, read as Keycloak reads it
/// (any case).
pub fn sends_no_code(config: &AuthenticatorConfigRepresentation) -> bool {
    config
        .config
        .as_ref()
        .and_then(|values| values.get(COURIER_KEY))
        .is_some_and(|courier| courier.eq_ignore_ascii_case(NONE_COURIER))
}

/// The config of a message OTP step of a messaging realm as the migration
/// leaves it: its code page policy stated ([`code_page_patch`]), and its
/// code sent to the voter's one channel (`messageCourierAttribute`
/// `CHOSEN`, R7-01), whatever courier it named. `None` when it needs
/// neither, so a migrated realm is left unchanged, and for a step on
/// `NONE` ([`sends_no_code`]): a step configured to send nothing never
/// starts sending.
pub fn code_step_patch(
    config: &AuthenticatorConfigRepresentation,
    role: CodeStepRole,
) -> Option<AuthenticatorConfigRepresentation> {
    if sends_no_code(config) {
        return None;
    }
    let mut patched = code_page_patch(config, role).unwrap_or_else(|| config.clone());
    let values = patched.config.get_or_insert_with(HashMap::new);
    if values.get(COURIER_KEY).map(String::as_str) != Some(CHOSEN_COURIER) {
        values.insert(COURIER_KEY.to_string(), CHOSEN_COURIER.to_string());
    }
    (patched != *config).then_some(patched)
}

/// The enabled message OTP steps of a realm, from the executions of each
/// top-level flow (nested flows included) with the flow's role.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CodeSteps {
    /// Each config a step uses, with its role; a config the reset flow uses
    /// is a reset one.
    pub configs: BTreeMap<String, CodeStepRole>,
    /// Each step without a config, by execution id: Keycloak gives it the
    /// factory defaults (courier `BOTH`), so it gets a config of its own.
    pub unconfigured: BTreeMap<String, CodeStepRole>,
}

/// The execution requirement Keycloak never runs.
const DISABLED_REQUIREMENT: &str = "DISABLED";

/// The message OTP steps the migration points at the voter's one channel:
/// every enabled one, in any flow or nested flow, configured or not, as
/// the contact change's gate counts them (R7-01).
pub fn code_steps(
    flows: &[(CodeStepRole, Vec<AuthenticationExecutionInfoRepresentation>)],
) -> CodeSteps {
    let mut steps = CodeSteps::default();
    for (role, executions) in flows {
        for execution in executions {
            if execution.provider_id.as_deref() != Some(MESSAGE_OTP_PROVIDER)
                || execution.requirement.as_deref() == Some(DISABLED_REQUIREMENT)
            {
                continue;
            }
            let entry = match (&execution.authentication_config, &execution.id) {
                (Some(config), _) => steps.configs.entry(config.clone()).or_insert(*role),
                (None, Some(execution)) => {
                    steps.unconfigured.entry(execution.clone()).or_insert(*role)
                }
                (None, None) => continue,
            };
            if *role == CodeStepRole::Reset {
                *entry = CodeStepRole::Reset;
            }
        }
    }
    steps
}

/// The config created for a message OTP step that has none: Keycloak's
/// factory defaults from the step's config description (what the step
/// used until now), then [`code_step_patch`]. Named after the execution.
pub fn new_code_step_config(
    execution_id: &str,
    description: &AuthenticatorConfigInfoRepresentation,
    role: CodeStepRole,
) -> AuthenticatorConfigRepresentation {
    let defaults: HashMap<String, String> = description
        .properties
        .iter()
        .flatten()
        .filter_map(|property| {
            let name = property.name.clone()?;
            let value = match &property.default_value {
                None | Some(Value::Null) => String::new(),
                Some(Value::String(value)) => value.clone(),
                Some(other) => other.to_string(),
            };
            Some((name, value))
        })
        .collect();
    let config = AuthenticatorConfigRepresentation {
        alias: Some(format!("{MESSAGE_OTP_PROVIDER}-{execution_id}")),
        config: Some(defaults),
        id: None,
    };
    code_step_patch(&config, role).unwrap_or(config)
}

/// The keys `patched` sets differently from `current`, as `key value`.
fn config_changes(
    current: &AuthenticatorConfigRepresentation,
    patched: &AuthenticatorConfigRepresentation,
) -> Vec<String> {
    let empty = HashMap::new();
    let before = current.config.as_ref().unwrap_or(&empty);
    let mut changes: Vec<String> = patched
        .config
        .as_ref()
        .unwrap_or(&empty)
        .iter()
        .filter(|(key, value)| before.get(*key) != Some(*value))
        .map(|(key, value)| format!("{key} {value}"))
        .collect();
    changes.sort();
    changes
}

/// Whether Keycloak has the contact change required action: the realm
/// already registered it, or Keycloak offers it for registration. Without
/// it (a Keycloak image older than this release) nothing is migrated: the
/// old jar would read rewritten voters as legacy ones.
pub fn channel_action_available(
    registered: &[RequiredActionProviderRepresentation],
    unregistered: &[RequiredActionProviderRepresentation],
) -> bool {
    registered
        .iter()
        .any(|action| action.alias.as_deref() == Some(MESSAGE_CHANNEL_RA))
        || unregistered
            .iter()
            .any(|action| action.provider_id.as_deref() == Some(MESSAGE_CHANNEL_RA))
}

/// Why a run failed, which decides when it runs again.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationFailure {
    /// Keycloak lacks `message-channel-ra` ([`KeycloakLacksChannelAction`]):
    /// waited for, without a limit, until the new Keycloak image is out.
    KeycloakLacksChannelAction,
    /// Anything else: retried [`OTHER_FAILURE_RETRIES`] times.
    Other,
}

/// When the task runs again after a failed run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NextAttempt {
    /// In this many seconds.
    RetryIn(u32),
    /// Not until beat restarts.
    GiveUp,
}

/// The longest wait between two runs.
pub const MAX_RETRY_COUNTDOWN_SECONDS: u32 = 30 * 60;
/// Retries for a failure other than a Keycloak without the action.
pub const OTHER_FAILURE_RETRIES: u32 = 5;

/// The next run after a failed one, `retries` being the retries so far.
/// The wait doubles from 30 s up to [`MAX_RETRY_COUNTDOWN_SECONDS`].
pub fn next_attempt(failure: MigrationFailure, retries: u32) -> NextAttempt {
    let countdown = retry_countdown(retries).min(MAX_RETRY_COUNTDOWN_SECONDS);
    match failure {
        MigrationFailure::KeycloakLacksChannelAction => NextAttempt::RetryIn(countdown),
        MigrationFailure::Other if retries < OTHER_FAILURE_RETRIES => {
            NextAttempt::RetryIn(countdown)
        }
        MigrationFailure::Other => NextAttempt::GiveUp,
    }
}

/// Keycloak lacks `message-channel-ra`: the migration waits for the new
/// Keycloak image (deploy Keycloak before windmill).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("Keycloak lacks {MESSAGE_CHANNEL_RA}; waiting for the new Keycloak image")]
pub struct KeycloakLacksChannelAction;

/// Keycloak's `errorMessage` when it is a plain message key or sentence
/// (letters, spaces, `-`, `_`, `.`), never a value it echoes back.
fn error_message_key(body: &str) -> Option<String> {
    let value: Value = serde_json::from_str(body).ok()?;
    let message = value.get("errorMessage")?.as_str()?.trim();
    let plain = !message.is_empty()
        && message.len() <= 100
        && message
            .chars()
            .all(|c| c.is_ascii_alphabetic() || matches!(c, ' ' | '-' | '_' | '.'));
    plain.then(|| message.to_string())
}

/// A log line for a Keycloak HTTP error: the status and Keycloak's
/// `errorMessage` key, never the body, which may echo voter data.
pub fn http_error_summary(status: u16, body: &str) -> String {
    match error_message_key(body) {
        Some(key) => format!("HTTP {status} ({key})"),
        None => format!("HTTP {status}"),
    }
}

/// A log line for a keycloak crate error, without its body.
pub fn keycloak_error_summary(err: &KeycloakError) -> String {
    match err {
        KeycloakError::HttpFailure { status, text, .. } => http_error_summary(*status, text),
        KeycloakError::ReqwestFailure(err) => request_error_summary(err),
    }
}

/// A log line for a request that got no usable answer.
fn request_error_summary(err: &reqwest::Error) -> String {
    if let Some(status) = err.status() {
        return format!("HTTP {}", status.as_u16());
    }
    let kind = if err.is_timeout() {
        "timed out"
    } else if err.is_connect() {
        "connection failed"
    } else if err.is_decode() {
        "unreadable answer"
    } else {
        "request failed"
    };
    kind.to_string()
}

/// Voters per outcome in one realm.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UserCounts {
    pub rewritten: usize,
    pub no_destination: usize,
    pub already_recorded: usize,
    pub failed: usize,
}

/// What happened to one realm.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RealmOutcome {
    /// The event has no realm (yet).
    NoRealm,
    /// The realm's profile has no channel attribute: nothing to migrate.
    NoMessaging,
    Migrated {
        changes: Vec<String>,
        users: UserCounts,
    },
}

fn admin_url(pub_admin: &PubKeycloakAdmin, realm: &str, path: &str) -> String {
    format!("{}/admin/realms/{realm}/{path}", pub_admin.url)
}

/// A log-safe error for a failed keycloak crate call (see
/// [`keycloak_error_summary`]).
fn keycloak_failure(what: String, err: &KeycloakError) -> anyhow::Error {
    anyhow!("{what}: {}", keycloak_error_summary(err))
}

async fn admin_token(pub_admin: &PubKeycloakAdmin) -> Result<String> {
    pub_admin
        .token_supplier
        .get(&pub_admin.url)
        .await
        .map_err(|err| keycloak_failure("Error getting the Keycloak admin token".into(), &err))
}

/// The error for a non-success answer: the status and Keycloak's
/// `errorMessage` key only, never the body.
async fn answer_error(method: &str, url: &str, response: reqwest::Response) -> anyhow::Error {
    let status = response.status().as_u16();
    let body = response.text().await.unwrap_or_default();
    anyhow!(
        "Keycloak answered {} to {method} {url}",
        http_error_summary(status, &body)
    )
}

/// GETs a JSON document; `None` when Keycloak answers 404.
async fn get_json(pub_admin: &PubKeycloakAdmin, url: &str) -> Result<Option<Value>> {
    let token = admin_token(pub_admin).await?;
    let response = pub_admin
        .client
        .get(url)
        .bearer_auth(token)
        .send()
        .await
        .map_err(|err| anyhow!("Error sending GET {url}: {}", request_error_summary(&err)))?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    if !response.status().is_success() {
        return Err(answer_error("GET", url, response).await);
    }
    let value = response
        .json()
        .await
        .map_err(|err| anyhow!("Error reading GET {url}: {}", request_error_summary(&err)))?;
    Ok(Some(value))
}

async fn put_json(pub_admin: &PubKeycloakAdmin, url: &str, body: &Value) -> Result<()> {
    let token = admin_token(pub_admin).await?;
    let response = pub_admin
        .client
        .put(url)
        .bearer_auth(token)
        .json(body)
        .send()
        .await
        .map_err(|err| anyhow!("Error sending PUT {url}: {}", request_error_summary(&err)))?;
    if !response.status().is_success() {
        return Err(answer_error("PUT", url, response).await);
    }
    Ok(())
}

/// The realm's registered required actions, and those Keycloak offers for
/// registration.
async fn required_actions(
    admin: &KeycloakAdminClient,
    realm: &str,
) -> Result<(
    Vec<RequiredActionProviderRepresentation>,
    Vec<RequiredActionProviderRepresentation>,
)> {
    let registered = admin
        .client
        .realm_authentication_required_actions_get(realm)
        .await
        .map_err(|err| {
            keycloak_failure(format!("Error reading required actions of {realm}"), &err)
        })?;
    let unregistered = admin
        .client
        .realm_authentication_unregistered_required_actions_get(realm)
        .await
        .map_err(|err| {
            keycloak_failure(
                format!("Error reading unregistered required actions of {realm}"),
                &err,
            )
        })?;
    Ok((registered, unregistered))
}

/// Registers `message-channel-ra` when missing, and gives it and the old
/// `messaging-app-otp-ra` (when registered) recent authentication.
async fn migrate_required_actions(
    admin: &KeycloakAdminClient,
    pub_admin: &PubKeycloakAdmin,
    realm: &str,
    actions: &[RequiredActionProviderRepresentation],
) -> Result<Vec<String>> {
    let mut changes = vec![];
    let registered = |alias: &str| {
        actions
            .iter()
            .any(|action| action.alias.as_deref() == Some(alias))
    };
    if !registered(MESSAGE_CHANNEL_RA) {
        admin
            .client
            .realm_authentication_register_required_action_post(
                realm,
                RequiredActionProviderRepresentation {
                    provider_id: Some(MESSAGE_CHANNEL_RA.to_string()),
                    name: Some(MESSAGE_CHANNEL_RA_NAME.to_string()),
                    ..Default::default()
                },
            )
            .await
            .map_err(|err| {
                keycloak_failure(
                    format!("Error registering {MESSAGE_CHANNEL_RA} in {realm}"),
                    &err,
                )
            })?;
        let mut action = admin
            .client
            .realm_authentication_required_actions_with_alias_get(realm, MESSAGE_CHANNEL_RA)
            .await
            .map_err(|err| {
                keycloak_failure(
                    format!("Error reading {MESSAGE_CHANNEL_RA} in {realm}"),
                    &err,
                )
            })?;
        // Enabled only when this run registers it: a realm that disabled
        // it later keeps its choice.
        if action.enabled != Some(true) {
            action.enabled = Some(true);
            admin
                .client
                .realm_authentication_required_actions_with_alias_put(
                    realm,
                    MESSAGE_CHANNEL_RA,
                    action,
                )
                .await
                .map_err(|err| {
                    keycloak_failure(
                        format!("Error enabling {MESSAGE_CHANNEL_RA} in {realm}"),
                        &err,
                    )
                })?;
        }
        changes.push(format!("registered required action {MESSAGE_CHANNEL_RA}"));
    }
    for alias in [MESSAGE_CHANNEL_RA, MESSAGING_APP_OTP_RA] {
        if alias == MESSAGING_APP_OTP_RA && !registered(alias) {
            continue;
        }
        let url = admin_url(
            pub_admin,
            realm,
            &format!("authentication/required-actions/{alias}/config"),
        );
        let current: Option<HashMap<String, String>> = get_json(pub_admin, &url)
            .await?
            .and_then(|value| value.get("config").cloned())
            .map(serde_json::from_value)
            .transpose()
            .with_context(|| format!("Unreadable config of {alias} in {realm}"))?;
        match max_auth_age_patch(current.as_ref()) {
            Some(config) => {
                put_json(pub_admin, &url, &json!({ "config": config })).await?;
                changes.push(format!(
                    "set {MAX_AUTH_AGE_KEY} {MAX_AUTH_AGE_SECONDS} on required action {alias}"
                ));
            }
            None => {
                let age = current
                    .as_ref()
                    .and_then(|config| config.get(MAX_AUTH_AGE_KEY))
                    .cloned()
                    .unwrap_or_default();
                if age != MAX_AUTH_AGE_SECONDS {
                    warn!(
                        realm,
                        alias,
                        max_auth_age = age,
                        "Required action keeps the realm's own {MAX_AUTH_AGE_KEY}"
                    );
                }
            }
        }
    }
    Ok(changes)
}

/// States the code page policy of every message OTP step: `NEUTRAL` in the
/// realm's reset-credentials flow, `NAMED` at sign-in ([`code_page_patch`]),
/// and points every enabled message OTP step ([`code_steps`]) at the
/// voter's one channel ([`code_step_patch`], R7-01); a step without a
/// config gets one ([`new_code_step_config`]). Each change is logged by the
/// caller.
async fn migrate_reset_configs(
    admin: &KeycloakAdminClient,
    pub_admin: &PubKeycloakAdmin,
    realm: &str,
) -> Result<Vec<String>> {
    let mut changes = vec![];
    let reset_flow = admin
        .client
        .realm_get(realm)
        .await
        .map_err(|err| keycloak_failure(format!("Error reading realm {realm}"), &err))?
        .reset_credentials_flow;
    let flows = admin
        .client
        .realm_authentication_flows_get(realm)
        .await
        .map_err(|err| keycloak_failure(format!("Error reading flows of {realm}"), &err))?;
    let mut found = vec![];
    for flow in flows.iter().filter(|flow| flow.top_level == Some(true)) {
        let Some(alias) = flow.alias.as_deref() else {
            continue;
        };
        let role = if reset_flow.as_deref() == Some(alias) {
            CodeStepRole::Reset
        } else {
            CodeStepRole::SignIn
        };
        // Lists the executions of nested flows too.
        let executions = admin
            .get_flow_executions(pub_admin, realm, alias)
            .await
            .map_err(|err| {
                keycloak_failure(format!("Error reading flow {alias} of {realm}"), &err)
            })?;
        found.push((role, executions));
    }
    let steps = code_steps(&found);
    if !steps.unconfigured.is_empty() {
        let description = admin
            .client
            .realm_authentication_config_description_with_provider_id_get(
                realm,
                MESSAGE_OTP_PROVIDER,
            )
            .await
            .map_err(|err| {
                keycloak_failure(
                    format!(
                        "Error reading the {MESSAGE_OTP_PROVIDER} config description of {realm}"
                    ),
                    &err,
                )
            })?;
        for (execution_id, role) in &steps.unconfigured {
            let config = new_code_step_config(execution_id, &description, *role);
            let alias = config.alias.clone().unwrap_or_default();
            let set = config_changes(&AuthenticatorConfigRepresentation::default(), &config)
                .into_iter()
                .filter(|change| {
                    change.starts_with(COURIER_KEY) || change.starts_with(CODE_PAGE_DISCLOSURE_KEY)
                })
                .collect::<Vec<_>>()
                .join(", ");
            admin
                .client
                .realm_authentication_executions_with_execution_id_config_post(
                    realm,
                    execution_id,
                    config,
                )
                .await
                .map_err(|err| {
                    keycloak_failure(
                        format!("Error creating the config of execution {execution_id} of {realm}"),
                        &err,
                    )
                })?;
            changes.push(format!(
                "created authenticator config {alias} with the factory defaults and {set} for execution {execution_id} ({role:?}), which had none"
            ));
        }
    }
    for (id, role) in steps.configs {
        let config = admin
            .client
            .realm_authentication_config_with_id_get(realm, &id)
            .await
            .map_err(|err| {
                keycloak_failure(
                    format!("Error reading authenticator config {id} of {realm}"),
                    &err,
                )
            })?;
        if config.alias.as_deref() == Some(RESET_CODE_CONFIG_ALIAS) {
            let disclosure = config
                .config
                .as_ref()
                .and_then(|values| values.get(CODE_PAGE_DISCLOSURE_KEY))
                .filter(|value| !value.trim().is_empty());
            if let Some(disclosure) = disclosure.filter(|value| *value != NEUTRAL_DISCLOSURE) {
                warn!(
                    realm,
                    config_id = id,
                    code_page_disclosure = disclosure,
                    "Reset code config keeps the realm's own {CODE_PAGE_DISCLOSURE_KEY}"
                );
            }
        }
        if sends_no_code(&config) {
            let alias = config.alias.as_deref().unwrap_or("");
            // Not a change: logged on its own, so a migrated realm still
            // reports nothing to change.
            info!(realm, config_id = id, alias, role = ?role, "{NONE_STEP_LEFT}");
            continue;
        }
        if let Some(patched) = code_step_patch(&config, role) {
            let set = config_changes(&config, &patched).join(", ");
            admin
                .client
                .realm_authentication_config_with_id_put(realm, &id, patched)
                .await
                .map_err(|err| {
                    keycloak_failure(
                        format!("Error updating authenticator config {id} of {realm}"),
                        &err,
                    )
                })?;
            let alias = config.alias.as_deref().unwrap_or("");
            changes.push(format!(
                "set {set} on authenticator config {alias} ({id}, {role:?})"
            ));
        }
    }
    Ok(changes)
}

/// The ids of the voters with a first-release record, found by an exact
/// attribute search for each channel rather than by paging every voter.
async fn first_release_user_ids(
    admin: &KeycloakAdminClient,
    realm: &str,
) -> Result<BTreeSet<String>> {
    let mut ids = BTreeSet::new();
    for channel in CHANNEL_ORDER {
        let query = format!("{VERIFIED_CHANNELS_ATTR_NAME}:{channel}");
        let mut offset = 0;
        loop {
            // All ids are collected before any rewrite, which removes the
            // searched attribute and would shift later pages.
            let page = admin
                .client
                .realm_users_get(
                    realm,
                    Some(true),
                    None,
                    None,
                    None,
                    Some(true),
                    Some(offset),
                    None,
                    None,
                    None,
                    None,
                    Some(SEARCH_PAGE),
                    Some(query.clone()),
                    None,
                    None,
                )
                .await
                .map_err(|err| {
                    keycloak_failure(format!("Error searching users of {realm}"), &err)
                })?;
            let count = page.len();
            ids.extend(page.into_iter().filter_map(|user| user.id));
            if count < SEARCH_PAGE as usize {
                break;
            }
            offset += SEARCH_PAGE;
        }
    }
    Ok(ids)
}

/// Rewrites one voter, planned from their current record.
async fn migrate_user(admin: &KeycloakAdminClient, realm: &str, user_id: &str) -> Result<UserPlan> {
    let mut user = admin
        .client
        .realm_users_with_user_id_get(realm, user_id, None)
        .await
        .map_err(|err| {
            keycloak_failure(format!("Error reading user {user_id} of {realm}"), &err)
        })?;
    let plan = plan_user(user.email.as_deref(), user.attributes.as_ref());
    if plan.case == UserCase::Rewritten {
        apply_updates(
            user.attributes.get_or_insert_with(HashMap::new),
            &plan.updates,
        );
        admin
            .client
            .realm_users_with_user_id_put(realm, user_id, user)
            .await
            .map_err(|err| {
                keycloak_failure(format!("Error updating user {user_id} of {realm}"), &err)
            })?;
    }
    Ok(plan)
}

async fn migrate_users(admin: &KeycloakAdminClient, realm: &str) -> Result<UserCounts> {
    let mut counts = UserCounts::default();
    for user_id in first_release_user_ids(admin, realm).await? {
        match migrate_user(admin, realm, &user_id).await {
            Ok(plan) => {
                let channel = plan
                    .channel
                    .map(|channel| channel.to_string())
                    .unwrap_or_default();
                info!(
                    realm,
                    user_id,
                    case = %plan.case,
                    channel,
                    "Messaging attributes of voter migrated"
                );
                match plan.case {
                    UserCase::Rewritten => counts.rewritten += 1,
                    UserCase::NoDestination => counts.no_destination += 1,
                    UserCase::AlreadyRecorded => counts.already_recorded += 1,
                }
            }
            Err(err) => {
                // Sanitized: a status and an error key, never a body.
                error!(
                    realm,
                    user_id, "Messaging attributes of voter not migrated: {err:#}"
                );
                counts.failed += 1;
            }
        }
    }
    Ok(counts)
}

/// Migrates one event realm and its voters (see the module docs). Fails
/// with [`KeycloakLacksChannelAction`], changing nothing, on a Keycloak
/// without `message-channel-ra`.
#[instrument(skip(admin, pub_admin))]
pub async fn migrate_realm(
    admin: &KeycloakAdminClient,
    pub_admin: &PubKeycloakAdmin,
    realm: &str,
) -> Result<RealmOutcome> {
    let profile_url = admin_url(pub_admin, realm, "users/profile");
    let Some(mut profile) = get_json(pub_admin, &profile_url).await? else {
        return Ok(RealmOutcome::NoRealm);
    };
    if !uses_messaging(&profile) {
        return Ok(RealmOutcome::NoMessaging);
    }
    // Before any change: the old jar would read rewritten voters as legacy.
    let (registered, unregistered) = required_actions(admin, realm).await?;
    if !channel_action_available(&registered, &unregistered) {
        return Err(KeycloakLacksChannelAction.into());
    }
    // The profile first: without `message-number` in it, Keycloak would
    // drop the voters' new attribute.
    let mut changes = patch_profile(&mut profile);
    if !changes.is_empty() {
        put_json(pub_admin, &profile_url, &profile).await?;
    }
    changes.extend(migrate_required_actions(admin, pub_admin, realm, &registered).await?);
    changes.extend(migrate_reset_configs(admin, pub_admin, realm).await?);
    for change in &changes {
        info!(realm, change, "Messaging realm config migrated");
    }
    let users = migrate_users(admin, realm).await?;
    info!(
        realm,
        rewritten = users.rewritten,
        no_destination = users.no_destination,
        already_recorded = users.already_recorded,
        failed = users.failed,
        "Messaging attributes of voters migrated"
    );
    if users.failed > 0 {
        return Err(anyhow!(
            "{} voters of {realm} were not migrated",
            users.failed
        ));
    }
    Ok(RealmOutcome::Migrated { changes, users })
}

/// [`migrate_realm`] for the realm of every election event. Every realm is
/// tried; if any failed the result is an error naming them, after the
/// others were migrated. On a Keycloak without `message-channel-ra` it
/// stops at the first realm that uses messaging, with
/// [`KeycloakLacksChannelAction`]: every realm would wait for the same
/// image.
#[instrument]
pub async fn migrate_messaging_attributes_now() -> Result<Vec<(String, RealmOutcome)>> {
    let realms = {
        let mut hasura_db_client: DbClient = get_hasura_pool()
            .await
            .get()
            .await
            .context("Error getting hasura client")?;
        let hasura_transaction = hasura_db_client
            .transaction()
            .await
            .context("Error starting hasura transaction")?;
        let mut realms = vec![];
        for tenant_id in get_tenant_ids(&hasura_transaction).await? {
            for event in get_all_tenant_election_events(&hasura_transaction, &tenant_id).await? {
                realms.push(get_event_realm(&tenant_id, &event.0.id));
            }
        }
        realms
    };
    let admin = KeycloakAdminClient::new().await?;
    let pub_admin = KeycloakAdminClient::pub_new().await?;
    let mut migrated = vec![];
    let mut failed = vec![];
    for realm in realms {
        match migrate_realm(&admin, &pub_admin, &realm).await {
            Ok(outcome) => {
                info!(realm, ?outcome, "Messaging realm migrated");
                migrated.push((realm, outcome));
            }
            Err(err) if err.downcast_ref::<KeycloakLacksChannelAction>().is_some() => {
                return Err(err);
            }
            Err(err) => {
                error!(realm, "Messaging realm not migrated: {err:#}");
                failed.push(realm);
            }
        }
    }
    if failed.is_empty() {
        Ok(migrated)
    } else {
        Err(anyhow!(
            "Messaging realm not migrated in {}",
            failed.join(", ")
        ))
    }
}

/// Runs again after a failure ([`next_attempt`]): while Keycloak lacks
/// `message-channel-ra`, with no limit (the retry limit here only bounds
/// that wait, at 30 minutes a run); after any other failure, as when
/// Keycloak is still starting with beat, up to [`OTHER_FAILURE_RETRIES`]
/// times. Every run leaves migrated realms unchanged.
#[instrument(skip(task))]
#[celery::task(bind = true, max_retries = 4294967295, retry_for_unexpected = false)]
pub async fn migrate_messaging_attributes(task: &Self) -> TaskResult<()> {
    let Err(err) = migrate_messaging_attributes_now().await else {
        return Ok(());
    };
    let retries = task.request().retries;
    let failure = if err.downcast_ref::<KeycloakLacksChannelAction>().is_some() {
        MigrationFailure::KeycloakLacksChannelAction
    } else {
        MigrationFailure::Other
    };
    match (failure, next_attempt(failure, retries)) {
        (MigrationFailure::KeycloakLacksChannelAction, NextAttempt::RetryIn(countdown)) => {
            warn!(retries, countdown, "Messaging migration not run: {err}");
            task.retry_with_countdown(countdown)
        }
        (_, NextAttempt::RetryIn(countdown)) => {
            error!(
                retries,
                countdown, "Messaging migration failed, retrying: {err:#}"
            );
            task.retry_with_countdown(countdown)
        }
        (_, NextAttempt::GiveUp) => {
            error!(
                retries,
                "Messaging migration failed, not retried until beat restarts: {err:#}"
            );
            Err(TaskError::UnexpectedError(
                "Messaging migration failed".to_string(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sequent_core::types::keycloak::{
        MESSAGE_CHANNEL_ATTR_NAME, MESSAGE_CONSENT_ATTR_NAME, MESSAGE_NUMBER_ATTR_NAME,
        MESSENGER_ID_ATTR_NAME, MESSENGER_PAGE_ATTR_NAME, MOBILE_PHONE_ATTR_NAME,
        VERIFIED_CHANNELS_ATTR_NAME, VIBER_NUMBER_ATTR_NAME, WHATSAPP_NUMBER_ATTR_NAME,
    };
    use sequent_core::types::voter_channel::{voter_channel, VoterChannelSource};
    use serde_json::json;

    const REMOVED: [&str; 3] = [
        VERIFIED_CHANNELS_ATTR_NAME,
        WHATSAPP_NUMBER_ATTR_NAME,
        VIBER_NUMBER_ATTR_NAME,
    ];

    fn attrs(pairs: &[(&str, &[&str])]) -> HashMap<String, Vec<String>> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.iter().map(|s| s.to_string()).collect()))
            .collect()
    }

    fn value<'a>(plan: &'a UserPlan, name: &str) -> Option<&'a [String]> {
        plan.updates.get(name).map(Vec::as_slice)
    }

    /// What Keycloak keeps after a user PUT: an attribute sent with no
    /// values is removed.
    fn stored_after_put(
        mut current: HashMap<String, Vec<String>>,
        plan: &UserPlan,
    ) -> HashMap<String, Vec<String>> {
        apply_updates(&mut current, &plan.updates);
        current.retain(|_, values| !values.is_empty());
        current
    }

    #[test]
    fn a_first_release_whatsapp_record_is_rewritten_to_the_ticket_attributes() {
        let current = attrs(&[
            (VERIFIED_CHANNELS_ATTR_NAME, &["SMS", "WHATSAPP"]),
            (WHATSAPP_NUMBER_ATTR_NAME, &["+34600000001"]),
            (MOBILE_PHONE_ATTR_NAME, &["+34600000002"]),
            (MESSAGE_CHANNEL_ATTR_NAME, &["WHATSAPP"]),
        ]);
        let plan = plan_user(Some("v@example.org"), Some(&current));
        assert_eq!(plan.case, UserCase::Rewritten);
        assert_eq!(plan.channel, Some(MessageChannel::WHATSAPP));
        assert_eq!(
            value(&plan, MESSAGE_CHANNEL_ATTR_NAME),
            Some(&["WHATSAPP".to_string()][..])
        );
        assert_eq!(
            value(&plan, MESSAGE_NUMBER_ATTR_NAME),
            Some(&["+34600000001".to_string()][..])
        );
        assert_eq!(value(&plan, MESSENGER_ID_ATTR_NAME), Some(&[][..]));
        assert_eq!(value(&plan, MESSENGER_PAGE_ATTR_NAME), Some(&[][..]));
        // The mobile number is the voter's own attribute: never touched.
        assert_eq!(value(&plan, MOBILE_PHONE_ATTR_NAME), None);
    }

    #[test]
    fn the_first_release_attributes_are_removed_explicitly_not_left_out() {
        let current = attrs(&[
            (VERIFIED_CHANNELS_ATTR_NAME, &["VIBER"]),
            (VIBER_NUMBER_ATTR_NAME, &["+34600000003"]),
            (WHATSAPP_NUMBER_ATTR_NAME, &["+34600000004"]),
        ]);
        let plan = plan_user(None, Some(&current));
        assert_eq!(plan.case, UserCase::Rewritten);
        // The user update merges: an attribute left out of the update
        // would be kept. Each one is sent with no values, which removes it.
        for name in REMOVED {
            assert_eq!(value(&plan, name), Some(&[][..]), "{name}");
        }
        let mut merged = current.clone();
        apply_updates(&mut merged, &plan.updates);
        for name in REMOVED {
            assert_eq!(merged.get(name), Some(&vec![]), "{name}");
        }
        let stored = stored_after_put(current, &plan);
        for name in REMOVED {
            assert!(!stored.contains_key(name), "{name}");
        }
        assert_eq!(stored[MESSAGE_NUMBER_ATTR_NAME], vec!["+34600000003"]);
    }

    #[test]
    fn a_rewritten_record_reads_as_recorded_with_the_same_channel_and_is_then_left_alone() {
        for (current, channel, destination) in [
            (
                attrs(&[
                    (VERIFIED_CHANNELS_ATTR_NAME, &["WHATSAPP"]),
                    (WHATSAPP_NUMBER_ATTR_NAME, &["+34600000001"]),
                ]),
                MessageChannel::WHATSAPP,
                "+34600000001",
            ),
            (
                attrs(&[
                    (VERIFIED_CHANNELS_ATTR_NAME, &["MESSENGER", "SMS"]),
                    (MESSENGER_ID_ATTR_NAME, &["psid-1"]),
                    (MESSENGER_PAGE_ATTR_NAME, &["page-1"]),
                    (MOBILE_PHONE_ATTR_NAME, &["+34600000002"]),
                ]),
                MessageChannel::MESSENGER,
                "psid-1",
            ),
            (
                attrs(&[
                    (VERIFIED_CHANNELS_ATTR_NAME, &["SMS"]),
                    (MOBILE_PHONE_ATTR_NAME, &["+34600000002"]),
                ]),
                MessageChannel::SMS,
                "+34600000002",
            ),
            (
                attrs(&[(VERIFIED_CHANNELS_ATTR_NAME, &["EMAIL"])]),
                MessageChannel::EMAIL,
                "v@example.org",
            ),
        ] {
            let plan = plan_user(Some("v@example.org"), Some(&current));
            assert_eq!(plan.case, UserCase::Rewritten);
            assert_eq!(plan.channel, Some(channel));
            let stored = stored_after_put(current, &plan);
            let read = voter_channel(Some("v@example.org"), Some(&stored)).unwrap();
            assert_eq!(read.source, VoterChannelSource::RECORDED);
            assert_eq!(read.channel, channel);
            assert_eq!(read.destination, destination);
            let again = plan_user(Some("v@example.org"), Some(&stored));
            assert_eq!(again.case, UserCase::AlreadyRecorded);
            assert!(again.updates.is_empty());
        }
    }

    #[test]
    fn messenger_gets_its_id_and_page_and_no_number() {
        let current = attrs(&[
            (VERIFIED_CHANNELS_ATTR_NAME, &["MESSENGER"]),
            (MESSENGER_ID_ATTR_NAME, &["psid-1"]),
            (MESSENGER_PAGE_ATTR_NAME, &["page-1"]),
        ]);
        let plan = plan_user(None, Some(&current));
        assert_eq!(value(&plan, MESSAGE_NUMBER_ATTR_NAME), Some(&[][..]));
        assert_eq!(
            value(&plan, MESSENGER_ID_ATTR_NAME),
            Some(&["psid-1".to_string()][..])
        );
        assert_eq!(
            value(&plan, MESSENGER_PAGE_ATTR_NAME),
            Some(&["page-1".to_string()][..])
        );
    }

    #[test]
    fn consent_is_kept_only_when_it_was_given_for_the_resolved_channel() {
        for (consent, kept) in [
            (
                r#"{"at":"2026-01-01T00:00:00Z","version":"1","organization":"Org","channel":"WHATSAPP"}"#,
                true,
            ),
            (
                r#"{"time":"2026-01-01T00:00:00Z","wording_version":"1","channel":"WHATSAPP"}"#,
                true,
            ),
            (
                r#"{"time":"2026-01-01T00:00:00Z","wording_version":"1","channel":"VIBER"}"#,
                false,
            ),
            ("not json", false),
        ] {
            let current = attrs(&[
                (VERIFIED_CHANNELS_ATTR_NAME, &["WHATSAPP"]),
                (WHATSAPP_NUMBER_ATTR_NAME, &["+34600000001"]),
                (MESSAGE_CONSENT_ATTR_NAME, &[consent]),
            ]);
            let plan = plan_user(None, Some(&current));
            assert_eq!(plan.case, UserCase::Rewritten);
            let consent_update = value(&plan, MESSAGE_CONSENT_ATTR_NAME);
            if kept {
                assert_eq!(consent_update, None, "{consent}");
            } else {
                assert_eq!(consent_update, Some(&[][..]), "{consent}");
            }
        }
    }

    #[test]
    fn a_first_release_record_without_a_usable_destination_is_untouched() {
        let current = attrs(&[
            (VERIFIED_CHANNELS_ATTR_NAME, &["WHATSAPP"]),
            (MOBILE_PHONE_ATTR_NAME, &["+34600000002"]),
        ]);
        // The reader gives no channel (never the email or the mobile
        // number, which the voter didn't choose): nothing is written.
        let plan = plan_user(Some("v@example.org"), Some(&current));
        assert_eq!(plan.case, UserCase::NoDestination);
        assert_eq!(plan.channel, None);
        assert!(plan.updates.is_empty());
    }

    #[test]
    fn recorded_and_legacy_records_are_untouched() {
        let recorded = attrs(&[
            (MESSAGE_CHANNEL_ATTR_NAME, &["VIBER"]),
            (MESSAGE_NUMBER_ATTR_NAME, &["+34600000003"]),
        ]);
        let plan = plan_user(None, Some(&recorded));
        assert_eq!(plan.case, UserCase::AlreadyRecorded);
        assert_eq!(plan.channel, Some(MessageChannel::VIBER));
        assert!(plan.updates.is_empty());
        let legacy = attrs(&[(MOBILE_PHONE_ATTR_NAME, &["+34600000002"])]);
        let plan = plan_user(None, Some(&legacy));
        assert_eq!(plan.case, UserCase::AlreadyRecorded);
        assert!(plan.updates.is_empty());
        assert_eq!(plan_user(None, None).case, UserCase::AlreadyRecorded);
    }

    fn profile() -> Value {
        json!({
            "attributes": [
                {"name": "username", "displayName": "${username}"},
                {"name": "sequent.read-only.message-channel", "displayName": "Notification channel", "validations": {}, "annotations": {}, "permissions": {"view": ["admin"], "edit": ["admin"]}, "multivalued": false},
                {"name": "sequent.read-only.whatsapp-number", "displayName": "WhatsApp number", "validations": {}, "annotations": {}, "permissions": {"view": ["admin"], "edit": ["admin"]}, "multivalued": false},
                {"name": "sequent.read-only.verified-channels", "displayName": "Verified channels", "permissions": {"view": ["admin"], "edit": ["admin"]}, "multivalued": true}
            ],
            "groups": [],
            "unmanagedAttributePolicy": "ADMIN_VIEW"
        })
    }

    #[test]
    fn a_realm_uses_messaging_when_its_profile_has_the_channel_attribute() {
        assert!(uses_messaging(&profile()));
        assert!(!uses_messaging(&json!({
            "attributes": [{"name": "username"}]
        })));
        assert!(!uses_messaging(&json!({})));
    }

    #[test]
    fn the_profile_gains_the_message_number_and_the_new_channel_label_once() {
        let mut patched = profile();
        let changes = patch_profile(&mut patched);
        assert_eq!(changes.len(), 2, "{changes:?}");
        let attributes = patched["attributes"].as_array().unwrap();
        let names: Vec<&str> = attributes
            .iter()
            .map(|a| a["name"].as_str().unwrap())
            .collect();
        assert_eq!(
            names,
            [
                "username",
                MESSAGE_CHANNEL_ATTR_NAME,
                MESSAGE_NUMBER_ATTR_NAME,
                WHATSAPP_NUMBER_ATTR_NAME,
                VERIFIED_CHANNELS_ATTR_NAME,
            ]
        );
        assert_eq!(attributes[1]["displayName"], "Contact channel");
        let mut expected = attributes[3].clone();
        expected["name"] = json!(MESSAGE_NUMBER_ATTR_NAME);
        expected["displayName"] = json!("Messaging number");
        assert_eq!(attributes[2], expected);
        // Everything else is kept, including the first-release attributes
        // the migration still reads.
        assert_eq!(patched["unmanagedAttributePolicy"], "ADMIN_VIEW");
        let again = patched.clone();
        assert!(patch_profile(&mut patched).is_empty());
        assert_eq!(patched, again);
    }

    #[test]
    fn a_custom_channel_label_is_kept() {
        let mut patched = profile();
        patched["attributes"][1]["displayName"] = json!("${channelLabel}");
        let changes = patch_profile(&mut patched);
        assert_eq!(changes.len(), 1, "{changes:?}");
        assert_eq!(patched["attributes"][1]["displayName"], "${channelLabel}");
    }

    #[test]
    fn the_contact_change_requires_recent_authentication() {
        let patched = max_auth_age_patch(None).unwrap();
        assert_eq!(patched["max_auth_age"], "300");
        let mut other = HashMap::new();
        other.insert("ttl".to_string(), "120".to_string());
        let patched = max_auth_age_patch(Some(&other)).unwrap();
        assert_eq!(patched["max_auth_age"], "300");
        assert_eq!(patched["ttl"], "120");
        assert_eq!(max_auth_age_patch(Some(&patched)), None);
        let mut chosen = HashMap::new();
        chosen.insert("max_auth_age".to_string(), "600".to_string());
        assert_eq!(max_auth_age_patch(Some(&chosen)), None);
    }

    fn reset_config(
        alias: &str,
        courier: &str,
        deferred: &str,
    ) -> AuthenticatorConfigRepresentation {
        AuthenticatorConfigRepresentation {
            alias: Some(alias.into()),
            id: Some("id-1".into()),
            config: Some(
                [
                    ("messageCourierAttribute", courier),
                    ("deferredUserAttribute", deferred),
                    ("ttl", "300"),
                ]
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            ),
        }
    }

    #[test]
    fn the_reset_code_goes_to_the_voters_one_channel() {
        let patched =
            reset_config_patch(&reset_config("reset.send.otp.tlf", "BOTH", "true")).unwrap();
        let config = patched.config.as_ref().unwrap();
        assert_eq!(config["messageCourierAttribute"], "CHOSEN");
        assert_eq!(config["deferredUserAttribute"], "false");
        assert_eq!(config["ttl"], "300");
        assert_eq!(patched.id.as_deref(), Some("id-1"));
        assert_eq!(reset_config_patch(&patched), None);
        assert_eq!(
            reset_config_patch(&reset_config("otp", "BOTH", "true")),
            None
        );
    }

    #[test]
    fn the_reset_code_page_reveals_nothing_extra() {
        // A first-release config gets the neutral page with the channel.
        let patched =
            reset_config_patch(&reset_config("reset.send.otp.tlf", "BOTH", "true")).unwrap();
        let config = patched.config.as_ref().unwrap();
        assert_eq!(config["code-page-disclosure"], "NEUTRAL");
        assert_eq!(reset_config_patch(&patched), None);
        // A config already on the channel gets only the neutral page.
        let chosen = reset_config("reset.send.otp.tlf", "CHOSEN", "false");
        let patched = reset_config_patch(&chosen).unwrap();
        let config = patched.config.as_ref().unwrap();
        assert_eq!(config["code-page-disclosure"], "NEUTRAL");
        assert_eq!(config["messageCourierAttribute"], "CHOSEN");
        assert_eq!(config["ttl"], "300");
        // A realm's own choice is kept.
        let mut named = chosen.clone();
        named
            .config
            .as_mut()
            .unwrap()
            .insert("code-page-disclosure".into(), "NAMED".into());
        assert_eq!(reset_config_patch(&named), None);
    }

    #[test]
    fn an_empty_verified_channels_list_is_not_a_first_release_record() {
        // Same reading as voter_channel (e1679754): an empty list is
        // absent, so the record is legacy and left alone, not reported as
        // a first-release record without a destination.
        let current = attrs(&[(VERIFIED_CHANNELS_ATTR_NAME, &[])]);
        let plan = plan_user(Some("v@example.org"), Some(&current));
        assert_eq!(plan.case, UserCase::AlreadyRecorded);
        assert_eq!(plan.channel, Some(MessageChannel::EMAIL));
        assert!(plan.updates.is_empty());
    }

    #[test]
    fn a_recorded_channel_without_its_destination_gets_no_other_channel() {
        let current = attrs(&[
            (MESSAGE_CHANNEL_ATTR_NAME, &["WHATSAPP"]),
            (MOBILE_PHONE_ATTR_NAME, &["+34600000002"]),
        ]);
        let plan = plan_user(Some("v@example.org"), Some(&current));
        assert_eq!(plan.case, UserCase::AlreadyRecorded);
        assert_eq!(plan.channel, None);
        assert!(plan.updates.is_empty());
    }

    fn action(alias: &str) -> RequiredActionProviderRepresentation {
        RequiredActionProviderRepresentation {
            alias: Some(alias.into()),
            provider_id: Some(alias.into()),
            ..Default::default()
        }
    }

    fn unregistered(provider_id: &str) -> RequiredActionProviderRepresentation {
        RequiredActionProviderRepresentation {
            provider_id: Some(provider_id.into()),
            name: Some("name".into()),
            ..Default::default()
        }
    }

    #[test]
    fn a_keycloak_without_the_contact_change_action_is_waited_for() {
        let old = [action("messaging-app-otp-ra"), action("VERIFY_EMAIL")];
        assert!(!channel_action_available(&old, &[]));
        assert!(!channel_action_available(
            &old,
            &[unregistered("verify-email-otp-ra")]
        ));
        // The provider is there: not yet registered in the realm, or
        // already registered.
        assert!(channel_action_available(
            &old,
            &[unregistered("message-channel-ra")]
        ));
        assert!(channel_action_available(
            &[action("message-channel-ra")],
            &[]
        ));
    }

    #[test]
    fn waiting_for_keycloak_retries_with_a_capped_backoff_and_never_gives_up() {
        let waits: Vec<NextAttempt> = (0..8)
            .map(|retries| next_attempt(MigrationFailure::KeycloakLacksChannelAction, retries))
            .collect();
        assert_eq!(
            waits,
            [30, 60, 120, 240, 480, 960, 1800, 1800].map(NextAttempt::RetryIn)
        );
        for retries in [50, 1_000, u32::MAX - 1] {
            assert_eq!(
                next_attempt(MigrationFailure::KeycloakLacksChannelAction, retries),
                NextAttempt::RetryIn(1800)
            );
        }
    }

    #[test]
    fn other_failures_give_up_after_five_retries() {
        let attempts: Vec<NextAttempt> = (0..7)
            .map(|retries| next_attempt(MigrationFailure::Other, retries))
            .collect();
        assert_eq!(
            attempts,
            [
                NextAttempt::RetryIn(30),
                NextAttempt::RetryIn(60),
                NextAttempt::RetryIn(120),
                NextAttempt::RetryIn(240),
                NextAttempt::RetryIn(480),
                NextAttempt::GiveUp,
                NextAttempt::GiveUp,
            ]
        );
    }

    #[test]
    fn http_errors_log_the_status_and_keycloak_error_key_only() {
        let echoed = r#"{"field":"email","errorMessage":"error-invalid-email","params":["email","v@example.org","+34600000001"]}"#;
        let summary = http_error_summary(400, echoed);
        assert_eq!(summary, "HTTP 400 (error-invalid-email)");
        assert_eq!(http_error_summary(500, ""), "HTTP 500");
        assert_eq!(
            http_error_summary(502, "<html>v@example.org</html>"),
            "HTTP 502"
        );
        // A message that isn't a plain key is dropped, not logged.
        for message in [
            "v@example.org",
            "+34600000001",
            "user 34600000001",
            &"a".repeat(200),
        ] {
            let body = json!({ "errorMessage": message }).to_string();
            assert_eq!(http_error_summary(409, &body), "HTTP 409", "{message}");
        }
        assert_eq!(
            http_error_summary(409, r#"{"errorMessage":"User exists with same email"}"#),
            "HTTP 409 (User exists with same email)"
        );
    }

    #[test]
    fn keycloak_crate_errors_log_no_body() {
        let text =
            r#"{"errorMessage":"error-invalid-length","params":["firstName","Juan Dela Cruz"]}"#;
        let err = keycloak::KeycloakError::HttpFailure {
            status: 400,
            body: serde_json::from_str(text).ok(),
            text: text.to_string(),
        };
        let summary = keycloak_error_summary(&err);
        assert_eq!(summary, "HTTP 400 (error-invalid-length)");
        assert!(!summary.contains("Juan"));
    }

    const JANITOR_REALM: &str =
        include_str!("../../external-bin/janitor/templates/COMELEC/keycloak.hbs");
    const DEV_EVENT_REALM: &str = include_str!(
        "../../../../.devcontainer/keycloak/import/tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5-event-33f18502-a67c-4853-8333-a58630663559.json"
    );

    fn realm(source: &str) -> Value {
        // The janitor template's handlebars expressions are all inside
        // JSON strings, so it parses as it is.
        serde_json::from_str(source).expect("realm parses as JSON")
    }

    fn contact_change_max_auth_age(realm: &Value) -> Option<&str> {
        let actions = realm["requiredActions"].as_array()?;
        let found: Vec<&Value> = actions
            .iter()
            .filter(|action| action["alias"] == MESSAGE_CHANNEL_RA)
            .collect();
        assert_eq!(found.len(), 1, "one {MESSAGE_CHANNEL_RA} entry");
        assert_eq!(found[0]["providerId"], MESSAGE_CHANNEL_RA);
        found[0]["config"][MAX_AUTH_AGE_KEY].as_str()
    }

    fn reset_code_configs(realm: &Value) -> Vec<&serde_json::Map<String, Value>> {
        realm["authenticatorConfig"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|config| config["alias"] == RESET_CODE_CONFIG_ALIAS)
            .map(|config| config["config"].as_object().unwrap())
            .collect()
    }

    fn assert_reset_code_configs(realm: &Value, name: &str) -> usize {
        let configs = reset_code_configs(realm);
        for config in &configs {
            assert_eq!(config[COURIER_KEY], CHOSEN_COURIER, "{name}");
            assert_eq!(config[DEFERRED_KEY], "false", "{name}");
            assert_eq!(
                config[CODE_PAGE_DISCLOSURE_KEY], NEUTRAL_DISCLOSURE,
                "{name}"
            );
        }
        configs.len()
    }

    #[test]
    fn the_janitor_realm_requires_a_recent_sign_in_and_a_neutral_reset_code_page() {
        let realm = realm(JANITOR_REALM);
        assert_eq!(
            contact_change_max_auth_age(&realm),
            Some(MAX_AUTH_AGE_SECONDS)
        );
        assert_eq!(MAX_AUTH_AGE_SECONDS, "300");
        assert!(assert_reset_code_configs(&realm, "janitor") >= 1);
    }

    #[test]
    fn the_dev_event_realm_requires_a_recent_sign_in_and_a_neutral_reset_code_page() {
        let realm = realm(DEV_EVENT_REALM);
        assert_eq!(
            contact_change_max_auth_age(&realm),
            Some(MAX_AUTH_AGE_SECONDS)
        );
        // Every reset code config there is (the dev realm may have none).
        assert_reset_code_configs(&realm, "dev event realm");
    }

    #[test]
    fn the_migration_gives_existing_realms_what_the_janitor_realm_has() {
        let janitor = realm(JANITOR_REALM);
        let template = reset_code_configs(&janitor)[0];
        let patched = reset_config_patch(&reset_config("reset.send.otp.tlf", "BOTH", "true"))
            .unwrap()
            .config
            .unwrap();
        for key in [COURIER_KEY, DEFERRED_KEY, CODE_PAGE_DISCLOSURE_KEY] {
            assert_eq!(
                Some(&patched[key]).map(String::as_str),
                template[key].as_str(),
                "{key}"
            );
        }
        let age = max_auth_age_patch(None).unwrap();
        assert_eq!(
            Some(age[MAX_AUTH_AGE_KEY].as_str()),
            contact_change_max_auth_age(&janitor)
        );
    }

    /// Lead decision: the code page reveals nothing by default, so after the
    /// upgrade a sign-in step must say NAMED to keep naming the channel.
    #[test]
    fn sign_in_code_steps_keep_naming_the_channel_and_reset_steps_reveal_nothing() {
        let sign_in = reset_config("email-sms", "CHOSEN", "false");
        let patched = code_page_patch(&sign_in, CodeStepRole::SignIn).unwrap();
        assert_eq!(
            patched.config.unwrap()[CODE_PAGE_DISCLOSURE_KEY],
            NAMED_DISCLOSURE
        );
        // Enrollment names the channel the voter entered whatever the config.
        assert!(
            code_page_patch(&reset_config("otp", "CHOSEN", "true"), CodeStepRole::SignIn).is_none()
        );
        // Another message OTP step of the reset flow is neutral too.
        let other_reset = reset_config("reset.other", "CHOSEN", "false");
        let patched = code_page_patch(&other_reset, CodeStepRole::Reset).unwrap();
        assert_eq!(
            patched.config.unwrap()[CODE_PAGE_DISCLOSURE_KEY],
            NEUTRAL_DISCLOSURE
        );
        // The reset code config keeps its own patch wherever it is used.
        assert_eq!(
            code_page_patch(
                &reset_config("reset.send.otp.tlf", "BOTH", "true"),
                CodeStepRole::SignIn
            ),
            reset_config_patch(&reset_config("reset.send.otp.tlf", "BOTH", "true"))
        );
    }

    /// Regression (R7-01): only the reset code step was switched to the
    /// voter's one channel; a sign-in step on `BOTH` (the default preset),
    /// `EMAIL` or `SMS` kept sending to the email and mobile number of a
    /// voter whose channel is WhatsApp. Every message OTP step of a
    /// messaging realm is set to `CHOSEN`, once.
    #[test]
    fn every_code_step_of_a_messaging_realm_sends_to_the_voters_one_channel() {
        for (alias, courier, deferred, role) in [
            ("email-sms", "BOTH", "false", CodeStepRole::SignIn),
            ("email-only", "EMAIL", "false", CodeStepRole::SignIn),
            ("enrollment", "SMS", "true", CodeStepRole::SignIn),
            ("enrollment-both", "BOTH", "true", CodeStepRole::SignIn),
            ("reset.other", "BOTH", "false", CodeStepRole::Reset),
        ] {
            let patched = code_step_patch(&reset_config(alias, courier, deferred), role)
                .unwrap_or_else(|| panic!("{alias} is patched"));
            assert_eq!(
                patched.config.unwrap()[COURIER_KEY],
                CHOSEN_COURIER,
                "{alias}"
            );
        }
        // Lead decision: a step configured to send nothing (`NONE`, in any
        // case, as Keycloak reads it) never starts sending; it is left as
        // it is, in either role.
        for courier in ["NONE", "none"] {
            for role in [CodeStepRole::SignIn, CodeStepRole::Reset] {
                assert_eq!(
                    code_step_patch(&reset_config("silent", courier, "false"), role),
                    None,
                    "{courier} {role:?}"
                );
            }
        }
        // A realm's own code page policy is kept while the courier changes.
        let mut own = reset_config("email-sms", "BOTH", "false");
        own.config
            .as_mut()
            .unwrap()
            .insert(CODE_PAGE_DISCLOSURE_KEY.into(), NEUTRAL_DISCLOSURE.into());
        let patched = code_step_patch(&own, CodeStepRole::SignIn)
            .unwrap()
            .config
            .unwrap();
        assert_eq!(patched[COURIER_KEY], CHOSEN_COURIER);
        assert_eq!(patched[CODE_PAGE_DISCLOSURE_KEY], NEUTRAL_DISCLOSURE);
        // Idempotent: a migrated step needs nothing.
        let mut migrated = reset_config("email-sms", "CHOSEN", "false");
        migrated
            .config
            .as_mut()
            .unwrap()
            .insert(CODE_PAGE_DISCLOSURE_KEY.into(), NAMED_DISCLOSURE.into());
        assert_eq!(code_step_patch(&migrated, CodeStepRole::SignIn), None);
        let reset =
            reset_config_patch(&reset_config("reset.send.otp.tlf", "BOTH", "true")).unwrap();
        assert_eq!(code_step_patch(&reset, CodeStepRole::Reset), None);
    }

    fn execution(
        id: &str,
        provider: &str,
        config: Option<&str>,
        requirement: &str,
        level: i32,
    ) -> AuthenticationExecutionInfoRepresentation {
        AuthenticationExecutionInfoRepresentation {
            id: Some(id.into()),
            provider_id: Some(provider.into()),
            authentication_config: config.map(Into::into),
            requirement: Some(requirement.into()),
            level: Some(level),
            ..Default::default()
        }
    }

    /// Regression (R7-01, FX3-KC): the contact change's gate counts every
    /// enabled message OTP step, a step without a config as `BOTH`, but the
    /// migration skipped steps without a config, so the change stayed
    /// refused on those realms. Every enabled step is found: configured or
    /// not, in a nested flow, enrolling, on `NONE`; a disabled one is left.
    #[test]
    fn every_enabled_code_step_is_found_with_or_without_a_config() {
        let sign_in = vec![
            execution(
                "e-1",
                MESSAGE_OTP_PROVIDER,
                Some("c-sign-in"),
                "REQUIRED",
                0,
            ),
            // In a nested flow (Keycloak lists those with a level).
            execution("e-2", MESSAGE_OTP_PROVIDER, None, "REQUIRED", 2),
            execution(
                "e-3",
                MESSAGE_OTP_PROVIDER,
                Some("c-none"),
                "ALTERNATIVE",
                1,
            ),
            execution("e-4", MESSAGE_OTP_PROVIDER, None, "DISABLED", 1),
            execution(
                "e-5",
                MESSAGE_OTP_PROVIDER,
                Some("c-disabled"),
                "DISABLED",
                0,
            ),
            execution("e-6", "auth-username-password-form", None, "REQUIRED", 0),
        ];
        let enrollment = vec![
            execution(
                "e-7",
                MESSAGE_OTP_PROVIDER,
                Some("c-enrollment"),
                "REQUIRED",
                0,
            ),
            execution("e-8", MESSAGE_OTP_PROVIDER, None, "CONDITIONAL", 1),
        ];
        let reset = vec![
            execution(
                "e-9",
                MESSAGE_OTP_PROVIDER,
                Some("c-sign-in"),
                "REQUIRED",
                0,
            ),
            execution("e-10", MESSAGE_OTP_PROVIDER, None, "REQUIRED", 0),
        ];
        let steps = code_steps(&[
            (CodeStepRole::SignIn, sign_in),
            (CodeStepRole::SignIn, enrollment),
            (CodeStepRole::Reset, reset),
        ]);
        let configs: Vec<(&str, CodeStepRole)> = steps
            .configs
            .iter()
            .map(|(id, role)| (id.as_str(), *role))
            .collect();
        assert_eq!(
            configs,
            vec![
                ("c-enrollment", CodeStepRole::SignIn),
                ("c-none", CodeStepRole::SignIn),
                // Used by the reset flow too.
                ("c-sign-in", CodeStepRole::Reset),
            ]
        );
        let unconfigured: Vec<(&str, CodeStepRole)> = steps
            .unconfigured
            .iter()
            .map(|(id, role)| (id.as_str(), *role))
            .collect();
        assert_eq!(
            unconfigured,
            vec![
                ("e-10", CodeStepRole::Reset),
                ("e-2", CodeStepRole::SignIn),
                ("e-8", CodeStepRole::SignIn),
            ]
        );
    }

    /// Regression (R7-01, FX3-KC): a step without a config sent with the
    /// factory defaults (courier `BOTH`). Its new config keeps every default
    /// and sends to the voter's one channel, with the code page policy of
    /// its role.
    #[test]
    fn a_code_step_without_a_config_gets_the_defaults_on_the_voters_channel() {
        use keycloak::types::ConfigPropertyRepresentation;
        let property = |name: &str, default: Option<Value>| ConfigPropertyRepresentation {
            name: Some(name.into()),
            default_value: default,
            ..Default::default()
        };
        let description = AuthenticatorConfigInfoRepresentation {
            properties: Some(vec![
                property(COURIER_KEY, Some(json!("BOTH"))),
                property(DEFERRED_KEY, Some(json!(false))),
                property("length", Some(json!(6))),
                property("ttl", Some(json!("300"))),
                property("sender", None),
            ]),
            ..Default::default()
        };
        let config = new_code_step_config("e-2", &description, CodeStepRole::SignIn);
        assert_eq!(
            config.alias.as_deref(),
            Some("message-otp-authenticator-e-2")
        );
        assert_eq!(config.id, None);
        let values = config.config.unwrap();
        assert_eq!(values[COURIER_KEY], CHOSEN_COURIER);
        assert_eq!(values[DEFERRED_KEY], "false");
        assert_eq!(values["length"], "6");
        assert_eq!(values["ttl"], "300");
        assert_eq!(values["sender"], "");
        assert_eq!(values[CODE_PAGE_DISCLOSURE_KEY], NAMED_DISCLOSURE);
        let reset = new_code_step_config("e-10", &description, CodeStepRole::Reset)
            .config
            .unwrap();
        assert_eq!(reset[COURIER_KEY], CHOSEN_COURIER);
        assert_eq!(reset[CODE_PAGE_DISCLOSURE_KEY], NEUTRAL_DISCLOSURE);
    }

    #[test]
    fn a_realms_own_code_page_policy_is_kept() {
        let mut own = reset_config("email-sms", "CHOSEN", "false");
        own.config
            .as_mut()
            .unwrap()
            .insert(CODE_PAGE_DISCLOSURE_KEY.into(), NEUTRAL_DISCLOSURE.into());
        assert!(code_page_patch(&own, CodeStepRole::SignIn).is_none());
        assert!(code_page_patch(&own, CodeStepRole::Reset).is_none());
    }

    /// Every message OTP step of the templates states its policy: NEUTRAL in
    /// the reset flow, NAMED at sign-in; enrollment steps may omit it.
    fn assert_code_step_policies(realm: &Value, name: &str) {
        let configs: HashMap<&str, &Value> = realm["authenticatorConfig"]
            .as_array()
            .unwrap()
            .iter()
            .map(|config| (config["alias"].as_str().unwrap(), &config["config"]))
            .collect();
        let flows: HashMap<&str, &Value> = realm["authenticationFlows"]
            .as_array()
            .unwrap()
            .iter()
            .map(|flow| (flow["alias"].as_str().unwrap(), flow))
            .collect();
        fn steps<'a>(flows: &HashMap<&str, &'a Value>, alias: &str, found: &mut Vec<&'a Value>) {
            for execution in flows[alias]["authenticationExecutions"].as_array().unwrap() {
                if let Some(sub) = execution["flowAlias"].as_str() {
                    steps(flows, sub, found);
                } else {
                    found.push(execution);
                }
            }
        }
        let reset_flow = realm["resetCredentialsFlow"].as_str().unwrap_or("");
        for (alias, flow) in &flows {
            if flow["topLevel"] != true {
                continue;
            }
            let mut found = vec![];
            steps(&flows, alias, &mut found);
            for execution in found {
                if execution["authenticator"] != MESSAGE_OTP_PROVIDER {
                    continue;
                }
                let config = configs[execution["authenticatorConfig"].as_str().unwrap()];
                let expected = if *alias == reset_flow {
                    Some(NEUTRAL_DISCLOSURE)
                } else if config[DEFERRED_KEY] == "true" {
                    None
                } else {
                    Some(NAMED_DISCLOSURE)
                };
                if let Some(expected) = expected {
                    assert_eq!(
                        config[CODE_PAGE_DISCLOSURE_KEY].as_str(),
                        Some(expected),
                        "{name}: {alias} / {}",
                        execution["authenticatorConfig"]
                    );
                }
            }
        }
    }

    #[test]
    fn every_template_code_step_states_its_code_page_policy() {
        assert_code_step_policies(&realm(JANITOR_REALM), "janitor");
        assert_code_step_policies(&realm(DEV_EVENT_REALM), "dev event realm");
    }
}
