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
//! - the reset code (`reset.send.otp.tlf`) goes to the voter's one channel;
//! - every voter with a first-release record (`verified-channels`) is
//!   rewritten to the ticket's attributes, and the first-release attributes
//!   are removed.
//!
//! Already migrated realms and voters are unchanged, so every run is safe.
//! Every change and every voter's outcome is logged, never a number, email
//! or Page-scoped ID.

use crate::postgres::election_event::get_all_tenant_election_events;
use crate::postgres::tenant::get_tenant_ids;
use crate::services::database::get_hasura_pool;
use crate::tasks::migrate_realm_permissions::retry_countdown;
use anyhow::{anyhow, Context, Result};
use celery::task::{Task, TaskResult};
use deadpool_postgres::Client as DbClient;
use keycloak::types::{AuthenticatorConfigRepresentation, RequiredActionProviderRepresentation};
use keycloak::KeycloakTokenSupplier;
use sequent_core::services::keycloak::{get_event_realm, KeycloakAdminClient, PubKeycloakAdmin};
use sequent_core::types::keycloak::{
    MESSAGE_CHANNEL_ATTR_NAME, MESSAGE_CONSENT_ATTR_NAME, MESSAGE_NUMBER_ATTR_NAME,
    MESSENGER_ID_ATTR_NAME, MESSENGER_PAGE_ATTR_NAME, VERIFIED_CHANNELS_ATTR_NAME,
    VIBER_NUMBER_ATTR_NAME, WHATSAPP_NUMBER_ATTR_NAME,
};
use sequent_core::types::messaging::MessageChannel;
use sequent_core::types::voter_channel::{voter_channel, VoterChannelSource, CHANNEL_ORDER};
use serde_json::{json, Value};
use std::collections::{BTreeSet, HashMap};
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
    let first_release =
        attributes.is_some_and(|attributes| attributes.contains_key(VERIFIED_CHANNELS_ATTR_NAME));
    let read = match voter_channel(email, attributes) {
        Some(read) if read.source == VoterChannelSource::VERIFIED_CHANNELS => read,
        // A first-release record read as legacy has no destination for any
        // channel the voter verified.
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

/// The reset code config sending to the voter's one channel, or `None` when
/// it isn't the reset code config or already does.
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
    changed.then_some(patched)
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

/// GETs a JSON document; `None` when Keycloak answers 404.
async fn get_json(pub_admin: &PubKeycloakAdmin, url: &str) -> Result<Option<Value>> {
    let token = pub_admin.token_supplier.get(&pub_admin.url).await?;
    let response = pub_admin.client.get(url).bearer_auth(token).send().await?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    if !response.status().is_success() {
        return Err(anyhow!(
            "Keycloak answered {} to GET {url}",
            response.status()
        ));
    }
    Ok(Some(response.json().await?))
}

async fn put_json(pub_admin: &PubKeycloakAdmin, url: &str, body: &Value) -> Result<()> {
    let token = pub_admin.token_supplier.get(&pub_admin.url).await?;
    let response = pub_admin
        .client
        .put(url)
        .bearer_auth(token)
        .json(body)
        .send()
        .await?;
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow!("Keycloak answered {status} to PUT {url}: {body}"));
    }
    Ok(())
}

/// Registers `message-channel-ra` when missing, and gives it and the old
/// `messaging-app-otp-ra` (when registered) recent authentication.
async fn migrate_required_actions(
    admin: &KeycloakAdminClient,
    pub_admin: &PubKeycloakAdmin,
    realm: &str,
) -> Result<Vec<String>> {
    let mut changes = vec![];
    let actions = admin
        .client
        .realm_authentication_required_actions_get(realm)
        .await
        .map_err(|err| anyhow!("Error reading required actions of {realm}: {err:?}"))?;
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
            .map_err(|err| anyhow!("Error registering {MESSAGE_CHANNEL_RA} in {realm}: {err:?}"))?;
        let mut action = admin
            .client
            .realm_authentication_required_actions_with_alias_get(realm, MESSAGE_CHANNEL_RA)
            .await
            .map_err(|err| anyhow!("Error reading {MESSAGE_CHANNEL_RA} in {realm}: {err:?}"))?;
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
                    anyhow!("Error enabling {MESSAGE_CHANNEL_RA} in {realm}: {err:?}")
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

/// Points every `reset.send.otp.tlf` config used by a flow at the voter's
/// one channel.
async fn migrate_reset_configs(
    admin: &KeycloakAdminClient,
    pub_admin: &PubKeycloakAdmin,
    realm: &str,
) -> Result<Vec<String>> {
    let mut changes = vec![];
    let flows = admin
        .client
        .realm_authentication_flows_get(realm)
        .await
        .map_err(|err| anyhow!("Error reading flows of {realm}: {err:?}"))?;
    let mut config_ids = BTreeSet::new();
    for flow in flows.iter().filter(|flow| flow.top_level == Some(true)) {
        let Some(alias) = flow.alias.as_deref() else {
            continue;
        };
        // Lists the executions of nested flows too.
        let executions = admin
            .get_flow_executions(pub_admin, realm, alias)
            .await
            .map_err(|err| anyhow!("Error reading flow {alias} of {realm}: {err:?}"))?;
        config_ids.extend(
            executions
                .into_iter()
                .filter_map(|execution| execution.authentication_config),
        );
    }
    for id in config_ids {
        let config = admin
            .client
            .realm_authentication_config_with_id_get(realm, &id)
            .await
            .map_err(|err| {
                anyhow!("Error reading authenticator config {id} of {realm}: {err:?}")
            })?;
        if let Some(patched) = reset_config_patch(&config) {
            admin
                .client
                .realm_authentication_config_with_id_put(realm, &id, patched)
                .await
                .map_err(|err| {
                    anyhow!("Error updating authenticator config {id} of {realm}: {err:?}")
                })?;
            changes.push(format!(
                "set {COURIER_KEY} {CHOSEN_COURIER} and {DEFERRED_KEY} false on authenticator config {RESET_CODE_CONFIG_ALIAS} ({id})"
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
                .map_err(|err| anyhow!("Error searching users of {realm}: {err:?}"))?;
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
        .map_err(|err| anyhow!("Error reading user {user_id} of {realm}: {err:?}"))?;
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
            .map_err(|err| anyhow!("Error updating user {user_id} of {realm}: {err:?}"))?;
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
                error!(
                    realm,
                    user_id, "Messaging attributes of voter not migrated: {err:?}"
                );
                counts.failed += 1;
            }
        }
    }
    Ok(counts)
}

/// Migrates one event realm and its voters (see the module docs).
#[instrument(skip(admin, pub_admin), err)]
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
    // The profile first: without `message-number` in it, Keycloak would
    // drop the voters' new attribute.
    let mut changes = patch_profile(&mut profile);
    if !changes.is_empty() {
        put_json(pub_admin, &profile_url, &profile).await?;
    }
    changes.extend(migrate_required_actions(admin, pub_admin, realm).await?);
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
/// others were migrated.
#[instrument(err)]
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
            Err(err) => {
                error!(realm, "Messaging realm not migrated: {err:?}");
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

/// Retried with a growing wait when the migration fails, as when Keycloak
/// is still starting with beat; every run leaves migrated realms unchanged.
#[instrument(skip(task))]
#[celery::task(bind = true, max_retries = 5)]
pub async fn migrate_messaging_attributes(task: &Self) -> TaskResult<()> {
    match migrate_messaging_attributes_now().await {
        Ok(_) => Ok(()),
        Err(err) => {
            let retries = task.request().retries;
            error!(retries, "Messaging migration failed: {err:?}");
            task.retry_with_countdown(retry_countdown(retries))
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
        // The reader falls back to the legacy channel (the email), which
        // is not what the voter verified: nothing is written.
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
}
