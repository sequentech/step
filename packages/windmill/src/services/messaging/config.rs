// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The election event's messaging configuration, stored in
//! `messaging_event_config`. Only [`save_event_messaging_config`] writes it,
//! after validating it, so it can't be changed around its validation. The
//! first release's `messaging:config` annotation was copied there by the
//! migration and is no longer read; each save writes it again (R7-25) only
//! so a rolled-back binary reads the configuration saved last.

use super::accounts::{callback_blockers_with, webhooks_public_url};
use crate::postgres::messaging::{
    event_configs_using_account, get_messaging_event_config, list_messaging_accounts,
    upsert_messaging_event_config, MessagingAccount,
};
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::types::messaging::{
    AccountSummary, DeliveryFeedback, EventMessagingConfig, MessageChannel, MessagingConfigError,
    ProviderCapabilities, PublicMessagingChannels, ReadinessBlocker,
    EVENT_MESSAGING_CONFIG_VERSION, MESSAGING_CONFIG_ANNOTATION,
};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use tracing::warn;
use uuid::Uuid;

/// What the configuration knows of `account`, with the callback blockers
/// of `MESSAGING_WEBHOOKS_PUBLIC_URL`.
pub fn account_summary(account: &MessagingAccount) -> AccountSummary {
    account_summary_with(webhooks_public_url().as_deref(), account)
}

/// [`account_summary`] for a given public base URL of the webhook
/// component. While a callback blocker is set, receipts are unavailable.
pub fn account_summary_with(
    component_url: Option<&str>,
    account: &MessagingAccount,
) -> AccountSummary {
    let callback_blockers = callback_blockers_with(component_url, account);
    let mut summary = AccountSummary {
        id: account.id.to_string(),
        tenant_id: account.tenant_id.to_string(),
        channel: account.channel,
        provider: account.provider,
        capabilities: account
            .sender
            .capabilities(account.channel)
            .unwrap_or_else(|| {
                // Stored accounts were validated for their channel; an account
                // that no longer fits offers no purpose.
                ProviderCapabilities {
                    channel: account.channel,
                    recipient: account.channel.recipient_kind(),
                    purposes: vec![],
                    template_required_for: vec![],
                    delivery_feedback: DeliveryFeedback::UNAVAILABLE,
                    reconciliation: false,
                    conversation_window_hours: None,
                    requires_provider_approval: false,
                }
            }),
        provider_approval: account.provider_approval,
        readiness: account.readiness,
        check: account.status.clone(),
        identity: Some(account.sender.identity()),
        public_label: account.sender.public_label(),
        messenger_page: account.sender.messenger_page(),
        callback_blockers: vec![],
    };
    if !callback_blockers.is_empty() {
        summary.capabilities.delivery_feedback = DeliveryFeedback::UNAVAILABLE;
    }
    summary.callback_blockers = callback_blockers;
    summary
}

async fn event_exists(
    tx: &Transaction<'_>,
    tenant_id: &Uuid,
    election_event_id: &Uuid,
) -> Result<bool> {
    Ok(tx
        .query_one(
            r#"
            SELECT EXISTS (
                SELECT 1 FROM sequent_backend.election_event
                WHERE tenant_id = $1 AND id = $2
            )
            "#,
            &[tenant_id, election_event_id],
        )
        .await?
        .get(0))
}

/// The configuration as stored, without the send-time checks.
pub async fn get_stored_event_messaging_config(
    tx: &Transaction<'_>,
    tenant_id: &Uuid,
    election_event_id: &Uuid,
) -> Result<Option<EventMessagingConfig>> {
    get_messaging_event_config(tx, tenant_id, election_event_id)
        .await?
        .map(|stored| {
            serde_json::from_value(stored.config).context("invalid stored messaging configuration")
        })
        .transpose()
}

/// Why a stored channel can't be used to send any more. Its references were
/// valid when the configuration was saved; an account may have been
/// deleted, moved or its provider removed since.
pub fn reference_errors(
    config: &EventMessagingConfig,
    tenant_id: &str,
    accounts: &[AccountSummary],
) -> Vec<MessagingConfigError> {
    let Err(errors) = config.validate(tenant_id, accounts, &[]) else {
        return vec![];
    };
    errors
        .into_iter()
        .filter(|error| match error {
            MessagingConfigError::UNSUPPORTED_VERSION { .. }
            | MessagingConfigError::DUPLICATE_CHANNEL { .. }
            | MessagingConfigError::UNKNOWN_ACCOUNT { .. }
            | MessagingConfigError::ACCOUNT_OF_ANOTHER_TENANT { .. }
            | MessagingConfigError::ACCOUNT_CHANNEL_MISMATCH { .. } => true,
            MessagingConfigError::PURPOSE_NOT_READY { blockers, .. } => {
                blockers.contains(&ReadinessBlocker::UNSUPPORTED_PROVIDER)
            }
            _ => false,
        })
        .collect()
}

/// What a send may use of a stored configuration: the channels whose
/// references still hold. A channel whose account was deleted, belongs to
/// another tenant, no longer fits the channel or has a removed provider is
/// left out, so the send fails with "No sending account for the channel"
/// instead of using it; the second copy of a duplicated channel is left
/// out too. A configuration of another version is refused whole.
pub fn sendable_config(
    config: EventMessagingConfig,
    tenant_id: &str,
    accounts: &[AccountSummary],
) -> Result<EventMessagingConfig> {
    if config.version != EVENT_MESSAGING_CONFIG_VERSION {
        return Err(anyhow!(
            "unsupported messaging configuration version {}",
            config.version
        ));
    }
    let mut dropped: BTreeSet<MessageChannel> = BTreeSet::new();
    for error in reference_errors(&config, tenant_id, accounts) {
        warn!("stored messaging configuration not used for sending: {error:?}");
        match error {
            MessagingConfigError::UNKNOWN_ACCOUNT { channel, .. }
            | MessagingConfigError::ACCOUNT_CHANNEL_MISMATCH { channel, .. }
            | MessagingConfigError::PURPOSE_NOT_READY { channel, .. } => {
                dropped.insert(channel);
            }
            MessagingConfigError::ACCOUNT_OF_ANOTHER_TENANT { account_id } => {
                dropped.extend(
                    config
                        .channels
                        .iter()
                        .filter(|c| c.account_id == account_id)
                        .map(|c| c.channel),
                );
            }
            _ => {}
        }
    }
    let mut seen = BTreeSet::new();
    let channels = config
        .channels
        .into_iter()
        .filter(|c| !dropped.contains(&c.channel) && seen.insert(c.channel))
        .collect();
    Ok(EventMessagingConfig { channels, ..config })
}

/// The event's configuration for sending, or `None` when it has none yet.
/// Events without one keep sending email and SMS as before. The references
/// it uses are checked again against the tenant's accounts now
/// ([`sendable_config`]).
pub async fn get_event_messaging_config(
    tx: &Transaction<'_>,
    tenant_id: &Uuid,
    election_event_id: &Uuid,
) -> Result<Option<EventMessagingConfig>> {
    let Some(config) = get_stored_event_messaging_config(tx, tenant_id, election_event_id).await?
    else {
        return Ok(None);
    };
    let accounts: Vec<AccountSummary> = list_messaging_accounts(tx, tenant_id)
        .await?
        .iter()
        .map(account_summary)
        .collect();
    sendable_config(config, &tenant_id.to_string(), &accounts).map(Some)
}

/// The bundle's configuration with its account ids as they were: the
/// import gives every id in the bundle a new one, but accounts belong to the
/// tenant, not the event. Election ids keep their new ids.
pub fn with_original_account_ids(
    config: &Value,
    replacement_map: &std::collections::HashMap<String, String>,
) -> Value {
    let original: std::collections::HashMap<&str, &str> = replacement_map
        .iter()
        .map(|(old, new)| (new.as_str(), old.as_str()))
        .collect();
    let mut config = config.clone();
    if let Some(channels) = config.get_mut("channels").and_then(Value::as_array_mut) {
        for channel in channels {
            if let Some(id) = channel.get("account_id").and_then(Value::as_str) {
                if let Some(old) = original.get(id) {
                    channel["account_id"] = Value::String(old.to_string());
                }
            }
        }
    }
    config
}

/// Whether the importer may write the event's messaging configuration
/// (`messaging-config-write`), as `update_event_messaging_config` requires.
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum MessagingConfigPermission {
    #[default]
    NOT_PERMITTED,
    PERMITTED,
}

/// What became of a bundle's messaging configuration.
#[derive(Debug, Clone, PartialEq)]
pub enum ImportOutcome {
    /// Stored; the projection is what the new event's realm publishes.
    Imported(PublicMessagingChannels),
    /// Not stored, with the one reason the import reports.
    NotImported(String),
}

/// Prefix of every [`ImportOutcome::NotImported`] reason.
pub const NOT_IMPORTED: &str = "Messaging configuration not imported";

/// Who stored an imported configuration.
pub const IMPORTED_BY: &str = "import";

/// Stores an imported event's configuration only as
/// `update_event_messaging_config` would: when the importer holds
/// `messaging-config-write` and it validates against the importing
/// tenant's accounts and the new event's elections. Otherwise nothing is
/// stored and the reason is returned for the import's log.
pub async fn import_event_messaging_config(
    tx: &Transaction<'_>,
    tenant_id: &Uuid,
    election_event_id: &Uuid,
    config: &Value,
    replacement_map: &std::collections::HashMap<String, String>,
    permission: MessagingConfigPermission,
) -> Result<ImportOutcome> {
    if permission != MessagingConfigPermission::PERMITTED {
        return Ok(ImportOutcome::NotImported(format!(
            "{NOT_IMPORTED}: importing it needs the messaging-config-write permission"
        )));
    }
    let config = with_original_account_ids(config, replacement_map);
    let Ok(parsed) = serde_json::from_value::<EventMessagingConfig>(config) else {
        return Ok(ImportOutcome::NotImported(format!(
            "{NOT_IMPORTED}: it can't be read"
        )));
    };
    match save_event_messaging_config_as(
        tx,
        tenant_id,
        election_event_id,
        &parsed,
        Some(IMPORTED_BY),
    )
    .await?
    {
        SaveOutcome::Saved(projection) => Ok(ImportOutcome::Imported(projection)),
        SaveOutcome::Invalid(errors) => Ok(ImportOutcome::NotImported(format!(
            "{NOT_IMPORTED}: {}",
            serde_json::to_string(&errors)?
        ))),
        SaveOutcome::UnknownEvent => Err(anyhow!("the imported election event isn't stored")),
    }
}

/// Who stored the configurations the migration copied from the
/// `messaging:config` annotation, never validated.
pub const MIGRATED_BY: &str = "migration";

/// [`MIGRATED_BY`] once validation found that the save would refuse it:
/// "Configuration needs saving again". Sends still re-check the references
/// they use ([`sendable_config`]).
pub const MIGRATED_INVALID_BY: &str = "migration: needs saving again";

/// Validates each configuration the migration copied, once, as the save
/// would, and marks those it would refuse ([`MIGRATED_INVALID_BY`]), with
/// their errors in the log. Returns the events marked.
pub async fn mark_migrated_configs(tx: &Transaction<'_>) -> Result<Vec<(Uuid, Uuid)>> {
    let rows = tx
        .query(
            r#"
            SELECT tenant_id, election_event_id, config
            FROM sequent_backend.messaging_event_config
            WHERE updated_by = $1
            ORDER BY tenant_id, election_event_id
            "#,
            &[&MIGRATED_BY],
        )
        .await?;
    let mut marked = vec![];
    for row in rows {
        let tenant_id: Uuid = row.get("tenant_id");
        let election_event_id: Uuid = row.get("election_event_id");
        let errors = match serde_json::from_value::<EventMessagingConfig>(row.get("config")) {
            Err(error) => vec![format!("it can't be read: {error}")],
            Ok(config) => {
                let accounts: Vec<AccountSummary> = list_messaging_accounts(tx, &tenant_id)
                    .await?
                    .iter()
                    .map(account_summary)
                    .collect();
                let elections: Vec<String> = election_labels(tx, &tenant_id, &election_event_id)
                    .await?
                    .into_keys()
                    .collect();
                match config.validate(&tenant_id.to_string(), &accounts, &elections) {
                    Ok(()) => vec![],
                    Err(errors) => errors.iter().map(|error| format!("{error:?}")).collect(),
                }
            }
        };
        let status = if errors.is_empty() {
            "valid"
        } else {
            "needs saving again"
        };
        if errors.is_empty() {
            tracing::info!(%tenant_id, %election_event_id, status, "Migrated messaging configuration validated");
            continue;
        }
        warn!(
            %tenant_id,
            %election_event_id,
            status,
            errors = ?errors,
            "Migrated messaging configuration would be refused by a save"
        );
        tx.execute(
            r#"
            UPDATE sequent_backend.messaging_event_config SET updated_by = $3
            WHERE tenant_id = $1 AND election_event_id = $2 AND updated_by = $4
            "#,
            &[
                &tenant_id,
                &election_event_id,
                &MIGRATED_INVALID_BY,
                &MIGRATED_BY,
            ],
        )
        .await?;
        marked.push((tenant_id, election_event_id));
    }
    Ok(marked)
}

/// Columns of an election that may name it in a voter record. Read from
/// the row as JSON, so a schema without one of them still works.
const ELECTION_LABEL_COLUMNS: &[&str] = &["name", "alias", "external_id", "permission_label"];

/// Each election of the event with the names a voter record may hold for
/// it.
async fn election_labels(
    tx: &Transaction<'_>,
    tenant_id: &Uuid,
    election_event_id: &Uuid,
) -> Result<BTreeMap<String, Vec<String>>> {
    Ok(tx
        .query(
            r#"
            SELECT id, to_jsonb(election) AS election FROM sequent_backend.election
            WHERE tenant_id = $1 AND election_event_id = $2
            "#,
            &[tenant_id, election_event_id],
        )
        .await?
        .into_iter()
        .map(|row| {
            let election: Value = row.get("election");
            let mut labels: Vec<String> = ELECTION_LABEL_COLUMNS
                .iter()
                .filter_map(|column| election[*column].as_str())
                .map(|label| label.trim().to_string())
                .filter(|label| !label.is_empty())
                .collect();
            labels.dedup();
            (row.get::<_, Uuid>("id").to_string(), labels)
        })
        .collect())
}

/// Share-locks the accounts the configuration names until the save commits
/// (R5-05): a delete of one of them locks its row before reading the
/// references, so it waits for the save and then finds the reference, or
/// the save waits for the delete and then finds no account. Either way
/// the loser gets its reason, and no configuration names a deleted account.
async fn lock_referenced_accounts(
    tx: &Transaction<'_>,
    tenant_id: &Uuid,
    config: &EventMessagingConfig,
) -> Result<()> {
    let referenced: Vec<Uuid> = config
        .channels
        .iter()
        .filter_map(|channel| Uuid::parse_str(channel.account_id.trim()).ok())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    if !referenced.is_empty() {
        tx.query(
            r#"
            SELECT id FROM sequent_backend.messaging_account
            WHERE tenant_id = $1 AND id = ANY($2)
            ORDER BY id
            FOR SHARE
            "#,
            &[tenant_id, &referenced],
        )
        .await?;
    }
    Ok(())
}

/// The public projection of every election event whose configuration
/// sends through the account, as its realm should publish it now. After a
/// change of the account's identity, Page or public label commits, each
/// is published again, so voters aren't sent to a Page that no longer
/// sends (R5-10). A stored configuration that no longer reads is skipped
/// and logged.
pub async fn projections_using_account(
    tx: &Transaction<'_>,
    tenant_id: &Uuid,
    account_id: &Uuid,
) -> Result<Vec<(Uuid, PublicMessagingChannels)>> {
    let events = event_configs_using_account(tx, tenant_id, account_id).await?;
    if events.is_empty() {
        return Ok(vec![]);
    }
    let component_url = webhooks_public_url();
    let accounts: Vec<AccountSummary> = list_messaging_accounts(tx, tenant_id)
        .await?
        .iter()
        .map(|account| account_summary_with(component_url.as_deref(), account))
        .collect();
    let mut projections = vec![];
    for event in events {
        let config = match get_stored_event_messaging_config(tx, tenant_id, &event).await {
            Ok(Some(config)) => config,
            Ok(None) => continue,
            Err(error) => {
                warn!(event = %event, "messaging projection not refreshed: {error:#}");
                continue;
            }
        };
        let labels = election_labels(tx, tenant_id, &event).await?;
        projections.push((event, config.public_projection(&accounts, &labels)));
    }
    Ok(projections)
}

/// Writes the saved configuration to the first release's
/// `messaging:config` annotation too (R7-25). This release never reads it:
/// it is there so a binary rolled back before `down.sql` runs (or after,
/// since `down.sql` drops the table) sends with the configuration saved
/// last, not the one from before the upgrade. The event's other
/// annotations are kept. To be removed with the release after this one.
async fn write_rollback_annotation(
    tx: &Transaction<'_>,
    tenant_id: &Uuid,
    election_event_id: &Uuid,
    config: &Value,
) -> Result<()> {
    tx.execute(
        r#"
        UPDATE sequent_backend.election_event
        SET annotations = COALESCE(annotations, '{}'::jsonb)
            || jsonb_build_object($3::text, $4::jsonb::text)
        WHERE tenant_id = $1 AND id = $2
        "#,
        &[
            tenant_id,
            election_event_id,
            &MESSAGING_CONFIG_ANNOTATION,
            config,
        ],
    )
    .await?;
    Ok(())
}

#[derive(Debug, PartialEq)]
pub enum SaveOutcome {
    /// Saved. The projection is what the event's realm should publish.
    Saved(PublicMessagingChannels),
    Invalid(Vec<MessagingConfigError>),
    UnknownEvent,
}

/// Validates the configuration against the tenant's accounts and the
/// event's elections, then stores it. Nothing is written when invalid.
pub async fn save_event_messaging_config(
    tx: &Transaction<'_>,
    tenant_id: &Uuid,
    election_event_id: &Uuid,
    config: &EventMessagingConfig,
) -> Result<SaveOutcome> {
    save_event_messaging_config_as(tx, tenant_id, election_event_id, config, None).await
}

/// [`save_event_messaging_config`], recording who saved it.
pub async fn save_event_messaging_config_as(
    tx: &Transaction<'_>,
    tenant_id: &Uuid,
    election_event_id: &Uuid,
    config: &EventMessagingConfig,
    updated_by: Option<&str>,
) -> Result<SaveOutcome> {
    save_event_messaging_config_with(
        tx,
        tenant_id,
        election_event_id,
        config,
        updated_by,
        webhooks_public_url().as_deref(),
    )
    .await
}

/// [`save_event_messaging_config_as`] for a given public base URL of the
/// webhook component (`MESSAGING_WEBHOOKS_PUBLIC_URL`): without one,
/// Messenger isn't ready for codes.
pub async fn save_event_messaging_config_with(
    tx: &Transaction<'_>,
    tenant_id: &Uuid,
    election_event_id: &Uuid,
    config: &EventMessagingConfig,
    updated_by: Option<&str>,
    component_url: Option<&str>,
) -> Result<SaveOutcome> {
    if !event_exists(tx, tenant_id, election_event_id).await? {
        return Ok(SaveOutcome::UnknownEvent);
    }
    lock_referenced_accounts(tx, tenant_id, config).await?;
    let accounts: Vec<AccountSummary> = list_messaging_accounts(tx, tenant_id)
        .await?
        .iter()
        .map(|account| account_summary_with(component_url, account))
        .collect();
    let labels = election_labels(tx, tenant_id, election_event_id).await?;
    let elections: Vec<String> = labels.keys().cloned().collect();
    if let Err(errors) = config.validate(&tenant_id.to_string(), &accounts, &elections) {
        return Ok(SaveOutcome::Invalid(errors));
    }
    let stored = serde_json::to_value(config)?;
    upsert_messaging_event_config(tx, tenant_id, election_event_id, &stored, updated_by).await?;
    write_rollback_annotation(tx, tenant_id, election_event_id, &stored).await?;
    Ok(SaveOutcome::Saved(
        config.public_projection(&accounts, &labels),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sequent_core::types::messaging::{
        AccountCheck, MessagePurpose, MessagingProvider, ProviderApproval, ReadinessPolicy,
    };
    use serde_json::json;

    const TENANT: &str = "tenant-1";

    fn account(id: &str, channel: MessageChannel, provider: MessagingProvider) -> AccountSummary {
        AccountSummary {
            id: id.to_string(),
            tenant_id: TENANT.to_string(),
            channel,
            provider,
            capabilities: provider.capabilities(channel).unwrap(),
            provider_approval: ProviderApproval::PENDING,
            readiness: ReadinessPolicy::PROVIDER_CHECK,
            // Not connected: these tests don't send.
            check: AccountCheck::default(),
            identity: None,
            public_label: None,
            messenger_page: None,
            callback_blockers: vec![],
        }
    }

    fn config(channels: serde_json::Value) -> EventMessagingConfig {
        serde_json::from_value(json!({"version": 1, "channels": channels})).unwrap()
    }

    fn channels(config: &EventMessagingConfig) -> Vec<(MessageChannel, String)> {
        config
            .channels
            .iter()
            .map(|c| (c.channel, c.account_id.clone()))
            .collect()
    }

    #[test]
    fn sends_use_only_channels_whose_references_still_hold() {
        let mut foreign = account(
            "foreign",
            MessageChannel::VIBER,
            MessagingProvider::VIBER_INFOBIP,
        );
        foreign.tenant_id = "tenant-2".to_string();
        let accounts = vec![
            account("sms", MessageChannel::SMS, MessagingProvider::AWS_SNS),
            account("console", MessageChannel::EMAIL, MessagingProvider::CONSOLE),
            account("mail", MessageChannel::EMAIL, MessagingProvider::AWS_SES),
            foreign,
        ];
        let stored = config(json!([
            {"channel": "SMS", "account_id": "sms", "purposes": ["NOTICE"]},
            {"channel": "SMS", "account_id": "deleted", "purposes": ["NOTICE"]},
            {"channel": "EMAIL", "account_id": "console", "purposes": ["NOTICE"]},
            {"channel": "WHATSAPP", "account_id": "deleted", "purposes": ["OTP"]},
            {"channel": "VIBER", "account_id": "foreign", "purposes": ["OTP"]},
            {"channel": "MESSENGER", "account_id": "mail", "purposes": ["OTP"]},
        ]));
        let sendable = sendable_config(stored, TENANT, &accounts).unwrap();
        assert_eq!(
            channels(&sendable),
            vec![(MessageChannel::SMS, "sms".to_string())]
        );
        assert_eq!(sendable.channels[0].purposes, vec![MessagePurpose::NOTICE]);
    }

    // The account rule of `postgres::messaging`, kept with the other
    // configuration-protection checks.
    #[test]
    fn credentials_are_entered_again_when_their_address_changes() {
        use crate::postgres::messaging::{endpoint_change_refusal, ENDPOINT_CREDENTIALS_REASON};
        use sequent_core::types::messaging::{AccountSender, CredentialName};
        let viber = |base_url: &str| AccountSender::VIBER_INFOBIP {
            base_url: base_url.to_string(),
            sender: "Elections".to_string(),
            approved_templates: Default::default(),
        };
        let whatsapp = |api_base_url: Option<&str>| AccountSender::WHATSAPP_CLOUD_API {
            business_account_id: "waba".to_string(),
            phone_number_id: "pn".to_string(),
            display_phone_number: "+63".to_string(),
            display_name: None,
            api_version: "v23.0".to_string(),
            api_base_url: api_base_url.map(str::to_string),
        };
        let none = BTreeSet::new();
        let api_key = BTreeSet::from([CredentialName::API_KEY]);
        let token = BTreeSet::from([CredentialName::ACCESS_TOKEN]);
        let a = "https://a.api.infobip.com";
        let b = "https://b.example.org";
        for (current, next, reentered, refused) in [
            (viber(a), viber(a), &none, false),
            (viber(a), viber(&format!("{a}/")), &none, false),
            (viber(a), viber(b), &none, true),
            (viber(a), viber(b), &api_key, false),
            (whatsapp(None), whatsapp(Some(b)), &none, true),
            (whatsapp(Some(b)), whatsapp(None), &none, true),
            (whatsapp(Some(b)), whatsapp(Some(a)), &api_key, true),
            (whatsapp(Some(b)), whatsapp(Some(a)), &token, false),
            (whatsapp(None), whatsapp(None), &none, false),
        ] {
            assert_eq!(
                endpoint_change_refusal(&current, &next, reentered),
                refused.then_some(ENDPOINT_CREDENTIALS_REASON),
                "{current:?} -> {next:?}"
            );
        }
    }

    #[test]
    fn imported_configurations_keep_their_account_ids_and_take_new_election_ids() {
        let account = "11111111-1111-4111-8111-111111111111";
        let election = "22222222-2222-4222-8222-222222222222";
        let replaced = std::collections::HashMap::from([
            (
                account.to_string(),
                "aaaaaaaa-1111-4111-8111-111111111111".to_string(),
            ),
            (
                election.to_string(),
                "bbbbbbbb-2222-4222-8222-222222222222".to_string(),
            ),
        ]);
        // As `replace_ids` leaves the bundle: every id is new.
        let bundle = json!({
            "version": 1,
            "channels": [{"channel": "SMS", "account_id": "aaaaaaaa-1111-4111-8111-111111111111"}],
            "election_channels": {"bbbbbbbb-2222-4222-8222-222222222222": ["SMS"]},
        });
        let imported = with_original_account_ids(&bundle, &replaced);
        assert_eq!(imported["channels"][0]["account_id"], account);
        assert_eq!(
            imported["election_channels"],
            json!({"bbbbbbbb-2222-4222-8222-222222222222": ["SMS"]})
        );
    }

    #[test]
    fn a_configuration_of_another_version_is_not_sent_with() {
        let stored: EventMessagingConfig =
            serde_json::from_value(json!({"version": 2, "channels": []})).unwrap();
        assert!(sendable_config(stored, TENANT, &[]).is_err());
    }
}
