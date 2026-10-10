// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Sending accounts, the message ledger and Messenger links.

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use deadpool_postgres::Transaction;
use messaging::sender::FailureKind;
use sequent_core::types::messaging::{
    AccountCheck, AccountIdentity, AccountLimits, AccountSender, BillingCategory, CredentialName,
    CredentialRecord, MessageAttemptState, MessageChannel, MessageDirection, MessagePurpose,
    MessageReason, MessagingProvider, MessengerLinkState, ProviderApproval, ProviderSupport,
    ReadinessPolicy,
};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::str::FromStr;
use strum::IntoEnumIterator;
use tokio_postgres::row::Row;
use tracing::{instrument, warn};
use uuid::Uuid;

fn parse_enum<T: FromStr>(row: &Row, column: &str) -> Result<T> {
    let value: String = row.try_get(column)?;
    T::from_str(&value).map_err(|_| anyhow!("unexpected {column} {value}"))
}

fn parse_optional_enum<T: FromStr>(row: &Row, column: &str) -> Result<Option<T>> {
    row.try_get::<_, Option<String>>(column)?
        .map(|value| T::from_str(&value).map_err(|_| anyhow!("unexpected {column} {value}")))
        .transpose()
}

fn parse_json<T: DeserializeOwned>(row: &Row, column: &str) -> Result<T> {
    let value: Value = row.try_get(column)?;
    serde_json::from_value(value).with_context(|| format!("unexpected {column}"))
}

#[derive(Debug, Clone, PartialEq)]
pub struct MessagingAccount {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub channel: MessageChannel,
    pub provider: MessagingProvider,
    pub name: String,
    pub sender: AccountSender,
    pub credentials: BTreeMap<CredentialName, CredentialRecord>,
    pub limits: AccountLimits,
    pub provider_approval: ProviderApproval,
    pub readiness: ReadinessPolicy,
    pub status: AccountCheck,
    pub webhook_key: String,
    pub is_default: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// An account's stored status. The column's default (`{}`), or no status
/// at all, is an account never checked: [`AccountCheck::default`].
pub fn parse_account_status(status: Value) -> Result<AccountCheck> {
    if status.is_null() || status.as_object().is_some_and(|o| o.is_empty()) {
        return Ok(AccountCheck::default());
    }
    serde_json::from_value(status).context("unexpected status")
}

impl TryFrom<Row> for MessagingAccount {
    type Error = anyhow::Error;

    fn try_from(row: Row) -> Result<Self> {
        let status: Value = row.try_get("status")?;
        Ok(MessagingAccount {
            id: row.try_get("id")?,
            tenant_id: row.try_get("tenant_id")?,
            channel: parse_enum(&row, "channel")?,
            provider: parse_enum(&row, "provider")?,
            name: row.try_get("name")?,
            sender: parse_json(&row, "sender")?,
            credentials: parse_json(&row, "credentials")?,
            limits: parse_json(&row, "limits")?,
            provider_approval: parse_enum(&row, "provider_approval")?,
            readiness: parse_enum(&row, "readiness")?,
            status: parse_account_status(status)?,
            webhook_key: row.try_get("webhook_key")?,
            is_default: row.try_get("is_default")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }
}

/// What an administrator sets on an account. Credentials are separate.
#[derive(Debug, Clone, PartialEq)]
pub struct AccountSettings {
    pub channel: MessageChannel,
    pub name: String,
    pub sender: AccountSender,
    pub limits: AccountLimits,
    pub provider_approval: ProviderApproval,
    pub readiness: ReadinessPolicy,
    pub is_default: bool,
}

/// Refusal of an address change that keeps the stored credentials: they
/// would be sent to the new address.
pub const ENDPOINT_CREDENTIALS_REASON: &str = "Enter the credentials again for the new address";
/// Refusal of an account API address that isn't `https`.
pub const ENDPOINT_HTTPS_REASON: &str = "The provider address must start with https://";

/// The address an account's credentials are sent to, when the account
/// names one: the Graph API base, the Infobip base or the SMTP server.
pub fn account_endpoint(sender: &AccountSender) -> Option<&str> {
    match sender {
        AccountSender::WHATSAPP_CLOUD_API { api_base_url, .. }
        | AccountSender::MESSENGER_SEND_API { api_base_url, .. } => api_base_url
            .as_deref()
            .map(str::trim)
            .filter(|url| !url.is_empty()),
        AccountSender::VIBER_INFOBIP { base_url, .. } => Some(base_url.trim()),
        AccountSender::SMTP { server_url, .. } => Some(server_url.trim()),
        _ => None,
    }
}

/// Whether moving an account from `current` to `next` sends its stored
/// credentials somewhere new without them being entered again. Every
/// credential the provider needs to send must be among `reentered`.
pub fn endpoint_change_refusal(
    current: &AccountSender,
    next: &AccountSender,
    reentered: &BTreeSet<CredentialName>,
) -> Option<&'static str> {
    let changed = account_endpoint(current).map(|url| url.trim_end_matches('/'))
        != account_endpoint(next).map(|url| url.trim_end_matches('/'));
    let all_reentered = next
        .provider()
        .required_credentials()
        .iter()
        .all(|name| reentered.contains(name));
    (changed && !all_reentered).then_some(ENDPOINT_CREDENTIALS_REASON)
}

/// Whether moving an account from `current` to `next` changes who it
/// sends as at the provider ([`AccountSender::identity`]): the WhatsApp
/// number or business account, the Messenger Page, the Viber sender, the
/// SNS or SES identity. Its last check named the old identity, so the
/// account isn't ready until it is checked again (AP-B3, R5-09).
pub fn identity_changed(current: &AccountSender, next: &AccountSender) -> bool {
    current.identity() != next.identity()
}

/// The reason logged when an account's identity changes.
pub const IDENTITY_CHANGED_REASON: &str =
    "The account's sending identity changed: it is checked again";

impl AccountSettings {
    fn validate(&self) -> Result<()> {
        let provider = self.sender.provider();
        if provider.support() == ProviderSupport::UNSUPPORTED {
            return Err(anyhow!(MessageReason::UNSUPPORTED_PROVIDER.as_str()));
        }
        if self.sender.capabilities(self.channel).is_none() {
            return Err(anyhow!("{provider} cannot send {}", self.channel));
        }
        if self.name.trim().is_empty() {
            return Err(anyhow!("the account needs a name"));
        }
        // API addresses carry the account's credentials: never in clear.
        // SMTP servers name their own scheme (smtp, smtps).
        if provider != MessagingProvider::SMTP {
            if let Some(url) = account_endpoint(&self.sender) {
                if !url.to_ascii_lowercase().starts_with("https://") {
                    return Err(anyhow!(ENDPOINT_HTTPS_REASON));
                }
            }
        }
        Ok(())
    }
}

async fn clear_other_defaults(
    tx: &Transaction<'_>,
    tenant_id: &Uuid,
    channel: MessageChannel,
    keep: &Uuid,
) -> Result<()> {
    tx.execute(
        r#"
        UPDATE sequent_backend.messaging_account
        SET is_default = false, updated_at = now()
        WHERE tenant_id = $1 AND channel = $2 AND id <> $3 AND is_default
        "#,
        &[tenant_id, &channel.to_string(), keep],
    )
    .await?;
    Ok(())
}

#[instrument(skip(tx, settings), err)]
pub async fn insert_messaging_account(
    tx: &Transaction<'_>,
    tenant_id: &Uuid,
    settings: &AccountSettings,
    webhook_key: &str,
) -> Result<MessagingAccount> {
    settings.validate()?;
    let id = Uuid::new_v4();
    if settings.is_default {
        clear_other_defaults(tx, tenant_id, settings.channel, &id).await?;
    }
    let row = tx
        .query_one(
            r#"
            INSERT INTO sequent_backend.messaging_account
                (id, tenant_id, channel, provider, name, sender, limits,
                 provider_approval, webhook_key, is_default, readiness)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING *
            "#,
            &[
                &id,
                tenant_id,
                &settings.channel.to_string(),
                &settings.sender.provider().to_string(),
                &settings.name,
                &serde_json::to_value(&settings.sender)?,
                &serde_json::to_value(&settings.limits)?,
                &settings.provider_approval.to_string(),
                &webhook_key,
                &settings.is_default,
                // The caller's readiness is parse-only: a new account is
                // checked by its provider.
                &ReadinessPolicy::PROVIDER_CHECK.to_string(),
            ],
        )
        .await?;
    row.try_into()
}

/// Updates an account. Changing the provider is refused: its credentials
/// would not apply. Changing the address its credentials are sent to is
/// refused too ([`ENDPOINT_CREDENTIALS_REASON`]): use
/// [`update_messaging_account_reentering`] with the new credentials.
pub async fn update_messaging_account(
    tx: &Transaction<'_>,
    tenant_id: &Uuid,
    id: &Uuid,
    settings: &AccountSettings,
) -> Result<Option<MessagingAccount>> {
    update_messaging_account_reentering(tx, tenant_id, id, settings, &BTreeSet::new()).await
}

/// [`update_messaging_account`] where the caller replaces `reentered`
/// credentials in the same transaction, so the address may change when
/// they cover every credential the provider sends with.
#[instrument(skip(tx, settings), err)]
pub async fn update_messaging_account_reentering(
    tx: &Transaction<'_>,
    tenant_id: &Uuid,
    id: &Uuid,
    settings: &AccountSettings,
    reentered: &BTreeSet<CredentialName>,
) -> Result<Option<MessagingAccount>> {
    let Some(current) = lock_messaging_account(tx, tenant_id, id).await? else {
        return Ok(None);
    };
    check_account_update(&current, settings, reentered)?;
    if settings.is_default {
        clear_other_defaults(tx, tenant_id, settings.channel, id).await?;
    }
    // "Needs a check" is the check's identity, not a stored readiness
    // (R5-09): after an identity change the stored check names the old
    // identity, so it stops counting. A check stored before checks named
    // their identity gets the old one now. The readiness column keeps the
    // values every binary reads.
    let legacy_identity = if identity_changed(&current.sender, &settings.sender) {
        warn!(account = %id, "{IDENTITY_CHANGED_REASON}");
        Some(serde_json::to_value(current.sender.identity())?)
    } else {
        None
    };
    let row = tx
        .query_opt(
            r#"
            UPDATE sequent_backend.messaging_account
            SET name = $3, sender = $4, limits = $5, provider_approval = $6,
                is_default = $7, readiness = $8,
                status = CASE
                    WHEN $9::jsonb IS NOT NULL
                         AND status->>'checked_at' IS NOT NULL
                         AND COALESCE(status->'details'->'identity', 'null'::jsonb)
                             = 'null'::jsonb
                    THEN jsonb_set(
                        status || jsonb_build_object(
                            'details', COALESCE(status->'details', '{}'::jsonb)),
                        '{details,identity}', $9::jsonb)
                    ELSE status END,
                updated_at = now()
            WHERE tenant_id = $1 AND id = $2
            RETURNING *
            "#,
            &[
                tenant_id,
                id,
                &settings.name,
                &serde_json::to_value(&settings.sender)?,
                &serde_json::to_value(&settings.limits)?,
                &settings.provider_approval.to_string(),
                &settings.is_default,
                &ReadinessPolicy::PROVIDER_CHECK.to_string(),
                &legacy_identity,
            ],
        )
        .await?;
    // Page-scoped IDs verified on an unfinished link must never be paired
    // with the replacement Page. The account lock is always taken before
    // link locks (also by creation, binding and confirmation).
    if current
        .sender
        .messenger_page()
        .map(|page| page.page_id.trim().to_owned())
        != settings
            .sender
            .messenger_page()
            .map(|page| page.page_id.trim().to_owned())
    {
        tx.execute(
            "UPDATE sequent_backend.messenger_link
             SET state = 'EXPIRED', encrypted_payload = NULL, updated_at = now()
             WHERE tenant_id = $1 AND account_id = $2 AND state IN ('PENDING', 'CODE_SENT')",
            &[tenant_id, id],
        )
        .await?;
    }
    row.map(TryInto::try_into).transpose()
}

/// Whether `settings` may replace `current`, with `reentered` credentials
/// replaced in the same save: valid settings (an `https` API address), the
/// same provider and channel, and credentials entered again for a new
/// address. Run before anything else, the candidate check's provider call
/// included (R5-04), so a refused save never reaches the provider.
pub fn check_account_update(
    current: &MessagingAccount,
    settings: &AccountSettings,
    reentered: &BTreeSet<CredentialName>,
) -> Result<()> {
    settings.validate()?;
    if current.provider != settings.sender.provider() || current.channel != settings.channel {
        return Err(anyhow!(
            "the provider and channel of an account cannot change; add another account"
        ));
    }
    if let Some(reason) = endpoint_change_refusal(&current.sender, &settings.sender, reentered) {
        return Err(anyhow!(reason));
    }
    Ok(())
}

#[instrument(skip(tx), err)]
pub async fn get_messaging_account(
    tx: &Transaction<'_>,
    tenant_id: &Uuid,
    id: &Uuid,
) -> Result<Option<MessagingAccount>> {
    tx.query_opt(
        "SELECT * FROM sequent_backend.messaging_account WHERE tenant_id = $1 AND id = $2",
        &[tenant_id, id],
    )
    .await?
    .map(TryInto::try_into)
    .transpose()
}

/// Locks an account before any of its Messenger links, so Page changes
/// serialize with link creation, binding and confirmation (R9-02).
#[instrument(skip(tx), err)]
pub async fn lock_messaging_account(
    tx: &Transaction<'_>,
    tenant_id: &Uuid,
    id: &Uuid,
) -> Result<Option<MessagingAccount>> {
    tx.query_opt(
        "SELECT * FROM sequent_backend.messaging_account WHERE tenant_id = $1 AND id = $2 FOR UPDATE",
        &[tenant_id, id],
    ).await?.map(TryInto::try_into).transpose()
}

#[instrument(skip_all, err)]
pub async fn get_messaging_account_by_webhook_key(
    tx: &Transaction<'_>,
    webhook_key: &str,
) -> Result<Option<MessagingAccount>> {
    tx.query_opt(
        "SELECT * FROM sequent_backend.messaging_account WHERE webhook_key = $1",
        &[&webhook_key],
    )
    .await?
    .map(TryInto::try_into)
    .transpose()
}

#[instrument(skip(tx), err)]
pub async fn list_messaging_accounts(
    tx: &Transaction<'_>,
    tenant_id: &Uuid,
) -> Result<Vec<MessagingAccount>> {
    tx.query(
        "SELECT * FROM sequent_backend.messaging_account WHERE tenant_id = $1 ORDER BY channel, name",
        &[tenant_id],
    )
    .await?
    .into_iter()
    .map(TryInto::try_into)
    .collect()
}

/// Stores a check's result, but only if the account still has the
/// identity and credentials the check started with (R5-03): a check that
/// started before an identity change or a credential replacement
/// describes what the account no longer is, and is dropped. The row is
/// locked while it is compared and written, so a concurrent save waits.
/// `checked_credentials` `None`: the check was of credentials being saved
/// in the same transaction (a candidate), compared by identity only. The
/// number-change signals are the row's, as it is now (R5-12): a signal
/// stored during the check stays. Returns whether the result was stored.
#[instrument(skip(tx, checked, checked_credentials, status), err)]
pub async fn store_messaging_account_check(
    tx: &Transaction<'_>,
    tenant_id: &Uuid,
    id: &Uuid,
    checked: &AccountIdentity,
    checked_credentials: Option<&BTreeMap<CredentialName, CredentialRecord>>,
    status: &AccountCheck,
) -> Result<bool> {
    let Some(row) = tx
        .query_opt(
            r#"
            SELECT sender, credentials FROM sequent_backend.messaging_account
            WHERE tenant_id = $1 AND id = $2
            FOR UPDATE
            "#,
            &[tenant_id, id],
        )
        .await?
    else {
        return Ok(false);
    };
    let sender: AccountSender = serde_json::from_value(row.try_get("sender")?)?;
    let credentials: BTreeMap<CredentialName, CredentialRecord> =
        serde_json::from_value(row.try_get("credentials")?)?;
    if &sender.identity() != checked
        || checked_credentials.is_some_and(|checked| checked != &credentials)
    {
        return Ok(false);
    }
    tx.execute(
        r#"
        UPDATE sequent_backend.messaging_account
        SET status = jsonb_set(
                $3::jsonb, '{details,number_changes}',
                COALESCE(status->'details'->'number_changes', '[]'::jsonb)),
            readiness = CASE WHEN readiness = 'UNCHECKED'
                             THEN 'PROVIDER_CHECK' ELSE readiness END,
            updated_at = now()
        WHERE tenant_id = $1 AND id = $2
        "#,
        &[tenant_id, id, &serde_json::to_value(status)?],
    )
    .await?;
    Ok(true)
}

/// Sets an account's status as given, number-change signals included.
/// Check results go through [`store_messaging_account_check`]; this is for
/// statuses set by hand (fixtures, tests).
#[instrument(skip(tx, status), err)]
pub async fn update_messaging_account_status(
    tx: &Transaction<'_>,
    tenant_id: &Uuid,
    id: &Uuid,
    status: &AccountCheck,
) -> Result<()> {
    tx.execute(
        r#"
        UPDATE sequent_backend.messaging_account
        SET status = $3::jsonb,
            readiness = CASE WHEN readiness = 'UNCHECKED'
                             THEN 'PROVIDER_CHECK' ELSE readiness END,
            updated_at = now()
        WHERE tenant_id = $1 AND id = $2
        "#,
        &[tenant_id, id, &serde_json::to_value(status)?],
    )
    .await?;
    Ok(())
}

/// The status of an account never checked, as every binary reads it: a
/// field written into an empty status (`'{}'`) starts from it, so the row
/// stays a whole check.
pub fn empty_status() -> Result<Value> {
    Ok(serde_json::to_value(AccountCheck::default())?)
}

/// Records that a check of the account was attempted at `at`, when its
/// result couldn't be stored or the check couldn't run: the daily pass
/// backs off from it instead of retrying it first on every pass (R5-14).
/// The stored result is untouched.
#[instrument(skip(tx), err)]
pub async fn record_messaging_account_check_attempt(
    tx: &Transaction<'_>,
    tenant_id: &Uuid,
    id: &Uuid,
    at: DateTime<Utc>,
) -> Result<()> {
    tx.execute(
        r#"
        UPDATE sequent_backend.messaging_account
        SET status = jsonb_set($4::jsonb || status, '{details,attempted_at}', to_jsonb($3::text))
        WHERE tenant_id = $1 AND id = $2
        "#,
        &[tenant_id, id, &at.to_rfc3339(), &empty_status()?],
    )
    .await?;
    Ok(())
}

/// Records when credentials were replaced. Values never reach this table.
#[instrument(skip(tx), err)]
pub async fn record_credentials_replaced(
    tx: &Transaction<'_>,
    tenant_id: &Uuid,
    id: &Uuid,
    names: &[CredentialName],
    at: DateTime<Utc>,
) -> Result<()> {
    let records: BTreeMap<String, CredentialRecord> = names
        .iter()
        .map(|name| {
            (
                name.to_string(),
                CredentialRecord {
                    replaced_at: at.to_rfc3339(),
                },
            )
        })
        .collect();
    tx.execute(
        r#"
        UPDATE sequent_backend.messaging_account
        SET credentials = credentials || $3, updated_at = now()
        WHERE tenant_id = $1 AND id = $2
        "#,
        &[tenant_id, id, &serde_json::to_value(records)?],
    )
    .await?;
    Ok(())
}

#[instrument(skip(tx), err)]
pub async fn delete_messaging_account(
    tx: &Transaction<'_>,
    tenant_id: &Uuid,
    id: &Uuid,
) -> Result<bool> {
    Ok(tx
        .execute(
            "DELETE FROM sequent_backend.messaging_account WHERE tenant_id = $1 AND id = $2",
            &[tenant_id, id],
        )
        .await?
        == 1)
}

/// The election events whose messaging configuration sends through the
/// account. An account in use isn't deleted.
#[instrument(skip(tx), err)]
pub async fn event_configs_using_account(
    tx: &Transaction<'_>,
    tenant_id: &Uuid,
    account_id: &Uuid,
) -> Result<Vec<Uuid>> {
    Ok(tx
        .query(
            r#"
            SELECT election_event_id
            FROM sequent_backend.messaging_event_config
            WHERE tenant_id = $1
              AND jsonb_path_exists(
                  config,
                  '$.channels[*] ? (@.account_id == $id)',
                  jsonb_build_object('id', $2::text)
              )
            ORDER BY election_event_id
            "#,
            &[tenant_id, &account_id.to_string()],
        )
        .await?
        .iter()
        .map(|row| row.try_get("election_event_id"))
        .collect::<std::result::Result<_, _>>()?)
}

/// An election event's stored messaging configuration. Only
/// `update_event_messaging_config` (after validation) and the event import
/// write it.
#[derive(Debug, Clone, PartialEq)]
pub struct StoredEventConfig {
    pub config: Value,
    pub version: i32,
    pub updated_at: DateTime<Utc>,
    pub updated_by: Option<String>,
}

#[instrument(skip(tx), err)]
pub async fn get_messaging_event_config(
    tx: &Transaction<'_>,
    tenant_id: &Uuid,
    election_event_id: &Uuid,
) -> Result<Option<StoredEventConfig>> {
    let row = tx
        .query_opt(
            r#"
            SELECT config, version, updated_at, updated_by
            FROM sequent_backend.messaging_event_config
            WHERE tenant_id = $1 AND election_event_id = $2
            "#,
            &[tenant_id, election_event_id],
        )
        .await?;
    Ok(row.map(|row| StoredEventConfig {
        config: row.get("config"),
        version: row.get("version"),
        updated_at: row.get("updated_at"),
        updated_by: row.get("updated_by"),
    }))
}

/// Stores the event's configuration, counting each save in `version`.
#[instrument(skip(tx, config), err)]
pub async fn upsert_messaging_event_config(
    tx: &Transaction<'_>,
    tenant_id: &Uuid,
    election_event_id: &Uuid,
    config: &Value,
    updated_by: Option<&str>,
) -> Result<()> {
    tx.execute(
        r#"
        INSERT INTO sequent_backend.messaging_event_config
            (tenant_id, election_event_id, config, updated_by)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (election_event_id) DO UPDATE
        SET config = EXCLUDED.config,
            updated_by = EXCLUDED.updated_by,
            version = sequent_backend.messaging_event_config.version + 1,
            updated_at = now()
        WHERE sequent_backend.messaging_event_config.tenant_id = EXCLUDED.tenant_id
        "#,
        &[tenant_id, election_event_id, config, &updated_by],
    )
    .await?;
    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
pub struct MessageRecord {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub election_event_id: Option<Uuid>,
    pub voter_id: Option<String>,
    pub account_id: Option<Uuid>,
    /// `None` for a message recorded without any channel ("No contact
    /// channel").
    pub channel: Option<MessageChannel>,
    pub direction: MessageDirection,
    pub purpose: Option<MessagePurpose>,
    pub template_alias: Option<String>,
    pub language: Option<String>,
    pub masked_destination: String,
    pub destination_digest: String,
    /// ISO 3166-1 alpha-2. Rows from the first release hold the calling
    /// code instead.
    pub destination_country: Option<String>,
    pub logical_key: Option<String>,
    pub attempt: i32,
    pub state: MessageAttemptState,
    pub provider_message_id: Option<String>,
    pub error: Option<String>,
    pub failure: Option<FailureKind>,
    pub billing: Option<Value>,
    pub billing_category: Option<String>,
    pub billable: Option<bool>,
    /// For a transient failure that will be retried: when.
    pub next_retry_at: Option<DateTime<Utc>>,
    /// When the attempt was handed to the provider. `None`: never sent.
    pub dispatched_at: Option<DateTime<Utc>>,
    pub last_reconciled_at: Option<DateTime<Utc>>,
    /// When the attempt's electoral log entry was handed to the log.
    /// `None` until then; the sweep writes the ones left behind.
    pub logged_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TryFrom<Row> for MessageRecord {
    type Error = anyhow::Error;

    fn try_from(row: Row) -> Result<Self> {
        Ok(MessageRecord {
            id: row.try_get("id")?,
            tenant_id: row.try_get("tenant_id")?,
            election_event_id: row.try_get("election_event_id")?,
            voter_id: row.try_get("voter_id")?,
            account_id: row.try_get("account_id")?,
            channel: parse_optional_enum(&row, "channel")?,
            direction: parse_enum(&row, "direction")?,
            purpose: parse_optional_enum(&row, "purpose")?,
            template_alias: row.try_get("template_alias")?,
            language: row.try_get("language")?,
            masked_destination: row.try_get("masked_destination")?,
            destination_digest: row.try_get("destination_digest")?,
            destination_country: row.try_get("destination_country")?,
            logical_key: row.try_get("logical_key")?,
            attempt: row.try_get("attempt")?,
            state: parse_enum(&row, "state")?,
            provider_message_id: row.try_get("provider_message_id")?,
            error: row.try_get("error")?,
            failure: parse_optional_enum(&row, "failure")?,
            billing: row.try_get("billing")?,
            billing_category: row.try_get("billing_category")?,
            billable: row.try_get("billable")?,
            next_retry_at: row.try_get("next_retry_at")?,
            dispatched_at: row.try_get("dispatched_at")?,
            last_reconciled_at: row.try_get("last_reconciled_at")?,
            logged_at: row.try_get("logged_at")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NewMessage {
    pub tenant_id: Uuid,
    pub election_event_id: Option<Uuid>,
    pub voter_id: Option<String>,
    pub account_id: Option<Uuid>,
    /// `None` only for "No contact channel".
    pub channel: Option<MessageChannel>,
    pub purpose: Option<MessagePurpose>,
    pub template_alias: Option<String>,
    pub language: Option<String>,
    pub masked_destination: String,
    pub destination_digest: String,
    /// ISO 3166-1 alpha-2, for phone numbers.
    pub destination_country: Option<String>,
    pub logical_key: Option<String>,
    pub attempt: i32,
    pub state: MessageAttemptState,
    pub provider_message_id: Option<String>,
    /// For attempts refused before reaching the provider: one
    /// [`sequent_core::types::messaging::MessageReason`] or preflight code.
    pub error: Option<String>,
    pub failure: Option<FailureKind>,
    pub billing_category: Option<BillingCategory>,
}

/// Inserts an outgoing attempt. With a logical key, a second insert of the
/// same attempt returns `None` instead of a duplicate. Earlier attempts of
/// the same message stop waiting for a retry: this is the retry.
#[instrument(skip(tx, message), err)]
pub async fn insert_message(
    tx: &Transaction<'_>,
    message: &NewMessage,
) -> Result<Option<MessageRecord>> {
    let inserted: Option<MessageRecord> = tx
        .query_opt(
            r#"
            INSERT INTO sequent_backend.message
                (tenant_id, election_event_id, voter_id, account_id, channel, direction,
                 purpose, template_alias, language, masked_destination, destination_digest,
                 destination_country, logical_key, attempt, state, provider_message_id,
                 error, failure, billing_category, failed_at)
            VALUES ($1, $2, $3, $4, $5, 'OUTBOUND', $6, $7, $8, $9, $10, $11, $12, $13, $14,
                    $15, $16, $17, $18, CASE WHEN $14 = 'FAILED' THEN now() END)
            ON CONFLICT (tenant_id, logical_key, attempt) DO NOTHING
            RETURNING *
            "#,
            &[
                &message.tenant_id,
                &message.election_event_id,
                &message.voter_id,
                &message.account_id,
                &message.channel.map(|c| c.to_string()),
                &message.purpose.map(|p| p.to_string()),
                &message.template_alias,
                &message.language,
                &message.masked_destination,
                &message.destination_digest,
                &message.destination_country,
                &message.logical_key,
                &message.attempt,
                &message.state.to_string(),
                &message.provider_message_id,
                &message.error,
                &message.failure.map(|f| f.to_string()),
                &message.billing_category.map(|c| c.to_string()),
            ],
        )
        .await?
        .map(TryInto::try_into)
        .transpose()?;
    if let (Some(_), Some(logical_key)) = (&inserted, &message.logical_key) {
        tx.execute(
            r#"
            UPDATE sequent_backend.message SET next_retry_at = NULL, updated_at = now()
            WHERE tenant_id = $1 AND logical_key = $2 AND attempt < $3
              AND next_retry_at IS NOT NULL
            "#,
            &[&message.tenant_id, logical_key, &message.attempt],
        )
        .await?;
    }
    Ok(inserted)
}

#[instrument(skip(tx), err)]
pub async fn list_attempts(
    tx: &Transaction<'_>,
    tenant_id: &Uuid,
    logical_key: &str,
) -> Result<Vec<MessageRecord>> {
    tx.query(
        r#"
        SELECT * FROM sequent_backend.message
        WHERE tenant_id = $1 AND logical_key = $2 AND direction = 'OUTBOUND'
        ORDER BY attempt
        "#,
        &[tenant_id, &logical_key],
    )
    .await?
    .into_iter()
    .map(TryInto::try_into)
    .collect()
}

#[instrument(skip(tx), err)]
pub async fn get_message(tx: &Transaction<'_>, id: &Uuid) -> Result<Option<MessageRecord>> {
    tx.query_opt("SELECT * FROM sequent_backend.message WHERE id = $1", &[id])
        .await?
        .map(TryInto::try_into)
        .transpose()
}

/// States an attempt may be in before moving to `to`.
fn predecessors(to: MessageAttemptState) -> Vec<String> {
    MessageAttemptState::iter()
        .filter(|from| from.can_transition_to(to))
        .map(|from| from.to_string())
        .collect()
}

/// What changes with a state transition.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StateChange {
    /// Kept when `None`: an existing provider ID is never replaced by
    /// Step's own reference.
    pub provider_message_id: Option<String>,
    pub error: Option<String>,
    pub failure: Option<FailureKind>,
    pub billing: Option<Value>,
    pub billing_category: Option<BillingCategory>,
    pub billable: Option<bool>,
    /// Moving to FAILED: when the retry on the same channel is due.
    pub next_retry_at: Option<DateTime<Utc>>,
}

/// Moves an attempt to `to` only if its current state allows it, so late or
/// repeated reports cannot regress it. Returns the updated record, or
/// `None` when the transition was not allowed.
#[instrument(skip(tx, change), err)]
pub async fn transition_message(
    tx: &Transaction<'_>,
    id: &Uuid,
    to: MessageAttemptState,
    change: &StateChange,
) -> Result<Option<MessageRecord>> {
    tx.query_opt(
        r#"
        UPDATE sequent_backend.message
        SET state = $2,
            provider_message_id = COALESCE(provider_message_id, $4),
            error = COALESCE($5, error),
            billing = COALESCE($6, billing),
            failure = COALESCE($7, failure),
            billing_category = COALESCE($8, billing_category),
            billable = COALESCE($9::boolean, billable),
            next_retry_at = CASE WHEN $2 = 'FAILED' THEN $10::timestamptz END,
            updated_at = now(),
            accepted_at = CASE WHEN $2 = 'ACCEPTED' THEN now() ELSE accepted_at END,
            delivered_at = CASE WHEN $2 = 'DELIVERED' THEN now() ELSE delivered_at END,
            failed_at = CASE WHEN $2 = 'FAILED' THEN now() ELSE failed_at END
        WHERE id = $1 AND state = ANY($3)
        RETURNING *
        "#,
        &[
            id,
            &to.to_string(),
            &predecessors(to),
            &change.provider_message_id,
            &change.error,
            &change.billing,
            &change.failure.map(|f| f.to_string()),
            &change.billing_category.map(|c| c.to_string()),
            &change.billable,
            &change.next_retry_at,
        ],
    )
    .await?
    .map(TryInto::try_into)
    .transpose()
}

/// Records that a queued attempt is being handed to the provider. `false`
/// when it is no longer queued (the sweep found it stale): then it must
/// not be sent.
#[instrument(skip(tx), err)]
pub async fn mark_dispatched(tx: &Transaction<'_>, id: &Uuid) -> Result<bool> {
    Ok(tx
        .execute(
            r#"
            UPDATE sequent_backend.message SET dispatched_at = now(), updated_at = now()
            WHERE id = $1 AND state = 'QUEUED' AND dispatched_at IS NULL
            "#,
            &[id],
        )
        .await?
        == 1)
}

#[instrument(skip(tx), err)]
pub async fn find_message_by_provider_id(
    tx: &Transaction<'_>,
    account_id: &Uuid,
    provider_message_id: &str,
) -> Result<Option<MessageRecord>> {
    tx.query_opt(
        r#"
        SELECT * FROM sequent_backend.message
        WHERE account_id = $1 AND provider_message_id = $2 AND direction = 'OUTBOUND'
        ORDER BY created_at DESC
        LIMIT 1
        "#,
        &[account_id, &provider_message_id],
    )
    .await?
    .map(TryInto::try_into)
    .transpose()
}

/// The attempt a provider report refers to by Step's own reference
/// (`<logical key>:<attempt>`), for providers that echo it back.
#[instrument(skip(tx), err)]
pub async fn find_message_by_reference(
    tx: &Transaction<'_>,
    account_id: &Uuid,
    reference: &str,
) -> Result<Option<MessageRecord>> {
    let Some((logical_key, attempt)) = reference.rsplit_once(':') else {
        return Ok(None);
    };
    let Ok(attempt) = attempt.parse::<i32>() else {
        return Ok(None);
    };
    tx.query_opt(
        r#"
        SELECT * FROM sequent_backend.message
        WHERE account_id = $1 AND logical_key = $2 AND attempt = $3
          AND direction = 'OUTBOUND'
        "#,
        &[account_id, &logical_key, &attempt],
    )
    .await?
    .map(TryInto::try_into)
    .transpose()
}

/// Marks as delivered every message the account had accepted for this
/// recipient up to `up_to`, for providers that report a watermark instead
/// of message IDs.
#[instrument(skip(tx), err)]
pub async fn deliver_messages_up_to(
    tx: &Transaction<'_>,
    account_id: &Uuid,
    destination_digest: &str,
    up_to: DateTime<Utc>,
) -> Result<u64> {
    Ok(tx
        .execute(
            r#"
            UPDATE sequent_backend.message
            SET state = 'DELIVERED', delivered_at = now(), updated_at = now()
            WHERE account_id = $1 AND destination_digest = $2
              AND direction = 'OUTBOUND' AND state = 'ACCEPTED'
              AND accepted_at <= $3
            "#,
            &[account_id, &destination_digest, &up_to],
        )
        .await?)
}

/// Records a Messenger recipient's Get Started or referral, which opens the
/// 24-hour window. Written before the code that follows it is sent.
#[instrument(skip(tx), err)]
pub async fn record_messenger_contact(
    tx: &Transaction<'_>,
    tenant_id: &Uuid,
    account_id: &Uuid,
    destination_digest: &str,
    at: DateTime<Utc>,
) -> Result<()> {
    tx.execute(
        r#"
        INSERT INTO sequent_backend.messenger_contact
            (tenant_id, account_id, destination_digest, last_get_started_at)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (account_id, destination_digest) DO UPDATE
        SET last_get_started_at = GREATEST(
                messenger_contact.last_get_started_at, EXCLUDED.last_get_started_at),
            updated_at = now()
        "#,
        &[tenant_id, account_id, &destination_digest, &at],
    )
    .await?;
    Ok(())
}

/// When the recipient last pressed Get Started or opened a referral for the
/// account. Text the recipient writes doesn't count: it isn't kept.
#[instrument(skip(tx), err)]
pub async fn last_get_started_at(
    tx: &Transaction<'_>,
    account_id: &Uuid,
    destination_digest: &str,
) -> Result<Option<DateTime<Utc>>> {
    Ok(tx
        .query_opt(
            r#"
            SELECT last_get_started_at FROM sequent_backend.messenger_contact
            WHERE account_id = $1 AND destination_digest = $2
            "#,
            &[account_id, &destination_digest],
        )
        .await?
        .map(|row| row.try_get("last_get_started_at"))
        .transpose()?)
}

/// Where a keyset page of the ledger ends: the last row's `(created_at,
/// id)`, or of `(next_retry_at, id)` for due retries.
pub type LedgerKey = (DateTime<Utc>, Uuid);

/// One keyset page of the ledger. Rows that can't be read (such as a value
/// this release doesn't know) are logged and left out, but still move the
/// page on, so one bad row never stops a sweep.
#[derive(Debug, Default)]
pub struct LedgerPage {
    pub records: Vec<MessageRecord>,
    /// The key of the page's last row, read or not.
    pub last: Option<LedgerKey>,
    /// How many rows the page held, read or not.
    pub rows: usize,
}

fn ledger_page(rows: Vec<Row>, key_column: &str) -> Result<LedgerPage> {
    let mut page = LedgerPage {
        rows: rows.len(),
        ..Default::default()
    };
    for row in rows {
        let id: Uuid = row.try_get("id")?;
        let at: DateTime<Utc> = row.try_get(key_column)?;
        page.last = Some((at, id));
        match MessageRecord::try_from(row) {
            Ok(record) => page.records.push(record),
            Err(error) => warn!(message_id = %id, "skipped an unreadable ledger row: {error:#}"),
        }
    }
    Ok(page)
}

/// Queued attempts whose worker stopped before recording the outcome, a
/// page at a time: never handed to the provider and created before
/// `queued_before`, or handed to it before `dispatched_before` (an attempt
/// that waited for its turn is in flight from when it was handed over).
#[instrument(skip(tx), err)]
pub async fn list_stale_queued(
    tx: &Transaction<'_>,
    queued_before: DateTime<Utc>,
    dispatched_before: DateTime<Utc>,
    after: Option<LedgerKey>,
    limit: i64,
) -> Result<LedgerPage> {
    let (after_at, after_id) = after.unzip();
    let rows = tx
        .query(
            r#"
            SELECT * FROM sequent_backend.message
            WHERE state = 'QUEUED' AND direction = 'OUTBOUND'
              AND ((dispatched_at IS NULL AND created_at < $1) OR dispatched_at < $5)
              AND ($2::timestamptz IS NULL OR (created_at, id) > ($2, $3))
            ORDER BY created_at, id
            LIMIT $4
            "#,
            &[
                &queued_before,
                &after_at,
                &after_id,
                &limit,
                &dispatched_before,
            ],
        )
        .await?;
    ledger_page(rows, "created_at")
}

/// Unknown outcomes worth asking the provider about, a page at a time:
/// through an account of one of `providers` (those that can reconcile),
/// newer than `newer_than` (older ones are Unresolved), and not asked about
/// since `asked_before`. Rows that can't be resolved are never selected, so
/// they can't starve the ones that can.
#[instrument(skip(tx), err)]
pub async fn list_reconcilable_unknown(
    tx: &Transaction<'_>,
    providers: &[String],
    newer_than: DateTime<Utc>,
    asked_before: DateTime<Utc>,
    after: Option<LedgerKey>,
    limit: i64,
) -> Result<LedgerPage> {
    let (after_at, after_id) = after.unzip();
    let rows = tx
        .query(
            r#"
            SELECT message.* FROM sequent_backend.message AS message
            JOIN sequent_backend.messaging_account AS account
              ON account.id = message.account_id
            WHERE message.state = 'UNKNOWN' AND message.direction = 'OUTBOUND'
              AND account.provider = ANY($1)
              AND message.created_at >= $2
              AND (message.last_reconciled_at IS NULL OR message.last_reconciled_at < $3)
              AND ($4::timestamptz IS NULL OR (message.created_at, message.id) > ($4, $5))
            ORDER BY message.created_at, message.id
            LIMIT $6
            "#,
            &[
                &providers,
                &newer_than,
                &asked_before,
                &after_at,
                &after_id,
                &limit,
            ],
        )
        .await?;
    ledger_page(rows, "created_at")
}

/// Notes that the provider was asked about an attempt, for the backoff.
#[instrument(skip(tx), err)]
pub async fn mark_reconciled(tx: &Transaction<'_>, id: &Uuid) -> Result<()> {
    tx.execute(
        "UPDATE sequent_backend.message SET last_reconciled_at = now() WHERE id = $1",
        &[id],
    )
    .await?;
    Ok(())
}

/// Failed attempts whose same-channel retry is due at `now`, a page at a
/// time.
#[instrument(skip(tx), err)]
pub async fn list_due_retries(
    tx: &Transaction<'_>,
    now: DateTime<Utc>,
    after: Option<LedgerKey>,
    limit: i64,
) -> Result<LedgerPage> {
    let (after_at, after_id) = after.unzip();
    let rows = tx
        .query(
            r#"
            SELECT * FROM sequent_backend.message
            WHERE next_retry_at IS NOT NULL AND next_retry_at <= $1 AND state = 'FAILED'
              AND ($2::timestamptz IS NULL OR (next_retry_at, id) > ($2, $3))
            ORDER BY next_retry_at, id
            LIMIT $4
            "#,
            &[&now, &after_at, &after_id, &limit],
        )
        .await?;
    ledger_page(rows, "next_retry_at")
}

/// Gives up the scheduled retry of a failed attempt. `reason` (why it
/// isn't retried) is added next to the failure's own reason, never in its
/// place ([`MessageReason::not_retried_after`]). `None` when the attempt
/// isn't a failure any more.
#[instrument(skip(tx), err)]
pub async fn cancel_retry(
    tx: &Transaction<'_>,
    id: &Uuid,
    reason: MessageReason,
) -> Result<Option<MessageRecord>> {
    tx.query_opt(
        r#"
        UPDATE sequent_backend.message
        SET next_retry_at = NULL,
            error = CASE WHEN COALESCE(btrim(error), '') = '' THEN $2
                         ELSE error || $3 || $2 END,
            updated_at = now()
        WHERE id = $1 AND state = 'FAILED'
        RETURNING *
        "#,
        &[
            id,
            &reason.as_str(),
            &sequent_core::types::messaging::REASON_SEPARATOR,
        ],
    )
    .await?
    .map(TryInto::try_into)
    .transpose()
}

/// Attempts with an outcome whose electoral log entry wasn't handed to the
/// log, recorded before `recorded_before` (so a sender still posting its
/// own isn't raced), a page at a time. Only attempts of an event: there is
/// no log without one.
#[instrument(skip(tx), err)]
pub async fn list_unlogged(
    tx: &Transaction<'_>,
    recorded_before: DateTime<Utc>,
    after: Option<LedgerKey>,
    limit: i64,
) -> Result<LedgerPage> {
    let (after_at, after_id) = after.unzip();
    let rows = tx
        .query(
            r#"
            SELECT * FROM sequent_backend.message
            WHERE logged_at IS NULL AND state <> 'QUEUED' AND direction = 'OUTBOUND'
              AND election_event_id IS NOT NULL AND updated_at < $1
              AND ($2::timestamptz IS NULL OR (created_at, id) > ($2, $3))
            ORDER BY created_at, id
            LIMIT $4
            "#,
            &[&recorded_before, &after_at, &after_id, &limit],
        )
        .await?;
    ledger_page(rows, "created_at")
}

/// Records that the attempts' electoral log entries were handed to the log.
#[instrument(skip(tx), err)]
pub async fn mark_logged(tx: &Transaction<'_>, ids: &[Uuid]) -> Result<()> {
    if ids.is_empty() {
        return Ok(());
    }
    tx.execute(
        r#"
        UPDATE sequent_backend.message SET logged_at = now()
        WHERE id = ANY($1) AND logged_at IS NULL
        "#,
        &[&ids],
    )
    .await?;
    Ok(())
}

/// Which channel a ledger filter asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelFilter {
    Any,
    /// Messages recorded without a channel ("No contact channel").
    NoChannel,
    Channel(MessageChannel),
}

/// The ID of the latest attempt of each message of an event, by its
/// current channel and outcome: the same messages [`count_event_messages`]
/// counts, so a filtered list agrees with the counts. `outcome` is a
/// [`MessageAttemptState`] or `UNRESOLVED`; `UNKNOWN` then means unknown and
/// not yet Unresolved.
#[instrument(skip(tx), err)]
pub async fn latest_attempt_ids(
    tx: &Transaction<'_>,
    tenant_id: &Uuid,
    election_event_id: &Uuid,
    channel: ChannelFilter,
    outcome: Option<&str>,
    unresolved_before: DateTime<Utc>,
) -> Result<Vec<Uuid>> {
    let (any_channel, no_channel, wanted_channel) = match channel {
        ChannelFilter::Any => (true, false, None),
        ChannelFilter::NoChannel => (false, true, None),
        ChannelFilter::Channel(channel) => (false, false, Some(channel.to_string())),
    };
    let outcome = outcome.map(str::to_uppercase);
    tx.query(
        r#"
        SELECT id FROM (
            SELECT DISTINCT ON (COALESCE(logical_key, id::text))
                id, channel,
                CASE WHEN state = 'UNKNOWN' AND created_at < $3 THEN 'UNRESOLVED'
                     ELSE state END AS outcome
            FROM sequent_backend.message
            WHERE tenant_id = $1 AND election_event_id = $2 AND direction = 'OUTBOUND'
            ORDER BY COALESCE(logical_key, id::text), attempt DESC
        ) AS latest
        WHERE ($4 OR ($5 AND channel IS NULL) OR channel = $6)
          AND ($7::text IS NULL OR outcome = $7)
        "#,
        &[
            tenant_id,
            election_event_id,
            &unresolved_before,
            &any_channel,
            &no_channel,
            &wanted_channel,
            &outcome,
        ],
    )
    .await?
    .into_iter()
    .map(|row| Ok(row.try_get("id")?))
    .collect()
}

/// Stores what an administrator sent in a bulk send (the send's template,
/// never per-voter content) as a stopped, archived `SEND_TEMPLATE` scheduled
/// event, with its own ID as the payload's `send_id`. A retry renders the
/// notice again from it. Returns the ID.
#[instrument(skip(tx, payload), err)]
pub async fn record_bulk_send(
    tx: &Transaction<'_>,
    tenant_id: &Uuid,
    election_event_id: Option<&Uuid>,
    created_by: Option<&str>,
    payload: &Value,
) -> Result<Uuid> {
    let id = Uuid::new_v4();
    let mut payload = payload.clone();
    if let Some(object) = payload.as_object_mut() {
        object.insert("send_id".to_string(), Value::String(id.to_string()));
    }
    tx.execute(
        r#"
        INSERT INTO sequent_backend.scheduled_event
            (id, tenant_id, election_event_id, created_at, event_processor, event_payload,
             created_by, stopped_at, archived_at)
        VALUES ($1, $2, $3, now(), 'SEND_TEMPLATE', $4, $5, now(), now())
        "#,
        &[&id, tenant_id, &election_event_id, &payload, &created_by],
    )
    .await?;
    Ok(id)
}

/// The payload a bulk send was sent with ([`record_bulk_send`]).
#[instrument(skip(tx), err)]
pub async fn get_bulk_send_payload(
    tx: &Transaction<'_>,
    tenant_id: &Uuid,
    send_id: &Uuid,
) -> Result<Option<Value>> {
    Ok(tx
        .query_opt(
            r#"
            SELECT event_payload FROM sequent_backend.scheduled_event
            WHERE id = $1 AND tenant_id = $2 AND event_processor = 'SEND_TEMPLATE'
            "#,
            &[send_id, tenant_id],
        )
        .await?
        .map(|row| row.try_get::<_, Option<Value>>("event_payload"))
        .transpose()?
        .flatten())
}

/// Messages (not attempts) of an event by channel and outcome, from the
/// ledger, so replayed jobs never count twice. Each logical message counts
/// once, by its latest attempt; an unknown outcome older than
/// `unresolved_before` is counted as `UNRESOLVED`, never as failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerCount {
    /// `None`: messages recorded without a channel ("No contact channel").
    pub channel: Option<MessageChannel>,
    /// A [`MessageAttemptState`], or `UNRESOLVED`.
    pub outcome: String,
    pub messages: i64,
}

#[instrument(skip(tx), err)]
pub async fn count_event_messages(
    tx: &Transaction<'_>,
    tenant_id: &Uuid,
    election_event_id: &Uuid,
    since: Option<DateTime<Utc>>,
    unresolved_before: DateTime<Utc>,
) -> Result<Vec<LedgerCount>> {
    tx.query(
        r#"
        SELECT channel, outcome, count(*) AS messages FROM (
            SELECT DISTINCT ON (COALESCE(logical_key, id::text))
                channel,
                CASE WHEN state = 'UNKNOWN' AND created_at < $4 THEN 'UNRESOLVED'
                     ELSE state END AS outcome
            FROM sequent_backend.message
            WHERE tenant_id = $1 AND election_event_id = $2 AND direction = 'OUTBOUND'
              AND ($3::timestamptz IS NULL OR created_at >= $3)
            ORDER BY COALESCE(logical_key, id::text), attempt DESC
        ) AS latest
        GROUP BY channel, outcome
        ORDER BY channel, outcome
        "#,
        &[tenant_id, election_event_id, &since, &unresolved_before],
    )
    .await?
    .into_iter()
    .map(|row| {
        Ok(LedgerCount {
            channel: parse_optional_enum(&row, "channel")?,
            outcome: row.try_get("outcome")?,
            messages: row.try_get("messages")?,
        })
    })
    .collect()
}

#[derive(Debug, Clone, PartialEq)]
pub struct MessengerLink {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub election_event_id: Option<Uuid>,
    pub account_id: Uuid,
    pub reference_digest: String,
    pub auth_session_digest: String,
    pub challenge_digest: String,
    pub encrypted_payload: Option<Vec<u8>>,
    pub language: Option<String>,
    pub state: MessengerLinkState,
    pub page_scoped_id: Option<String>,
    pub message_id: Option<Uuid>,
    pub expires_at: DateTime<Utc>,
}

impl TryFrom<Row> for MessengerLink {
    type Error = anyhow::Error;

    fn try_from(row: Row) -> Result<Self> {
        Ok(MessengerLink {
            id: row.try_get("id")?,
            tenant_id: row.try_get("tenant_id")?,
            election_event_id: row.try_get("election_event_id")?,
            account_id: row.try_get("account_id")?,
            reference_digest: row.try_get("reference_digest")?,
            auth_session_digest: row.try_get("auth_session_digest")?,
            challenge_digest: row.try_get("challenge_digest")?,
            encrypted_payload: row.try_get("encrypted_payload")?,
            language: row.try_get("language")?,
            state: parse_enum(&row, "state")?,
            page_scoped_id: row.try_get("page_scoped_id")?,
            message_id: row.try_get("message_id")?,
            expires_at: row.try_get("expires_at")?,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NewMessengerLink {
    pub tenant_id: Uuid,
    pub election_event_id: Option<Uuid>,
    pub account_id: Uuid,
    pub reference_digest: String,
    pub auth_session_digest: String,
    pub challenge_digest: String,
    pub encrypted_payload: Vec<u8>,
    pub language: Option<String>,
    pub expires_at: DateTime<Utc>,
}

/// Creates a link and replaces every unfinished link of the same session:
/// their codes are deleted and their references stop working.
#[instrument(skip_all, err)]
pub async fn insert_messenger_link(
    tx: &Transaction<'_>,
    link: &NewMessengerLink,
) -> Result<MessengerLink> {
    tx.execute(
        r#"
        UPDATE sequent_backend.messenger_link
        SET state = 'REPLACED', encrypted_payload = NULL, updated_at = now()
        WHERE tenant_id = $1 AND auth_session_digest = $2
          AND state IN ('PENDING', 'CODE_SENT')
        "#,
        &[&link.tenant_id, &link.auth_session_digest],
    )
    .await?;
    tx.query_one(
        r#"
        INSERT INTO sequent_backend.messenger_link
            (tenant_id, election_event_id, account_id, reference_digest,
             auth_session_digest, challenge_digest, encrypted_payload, language, state,
             expires_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'PENDING', $9)
        RETURNING *
        "#,
        &[
            &link.tenant_id,
            &link.election_event_id,
            &link.account_id,
            &link.reference_digest,
            &link.auth_session_digest,
            &link.challenge_digest,
            &link.encrypted_payload,
            &link.language,
            &link.expires_at,
        ],
    )
    .await?
    .try_into()
}

#[instrument(skip_all, err)]
pub async fn get_messenger_link_by_reference(
    tx: &Transaction<'_>,
    tenant_id: &Uuid,
    reference_digest: &str,
) -> Result<Option<MessengerLink>> {
    tx.query_opt(
        r#"
        SELECT * FROM sequent_backend.messenger_link
        WHERE tenant_id = $1 AND reference_digest = $2
        FOR UPDATE
        "#,
        &[tenant_id, &reference_digest],
    )
    .await?
    .map(TryInto::try_into)
    .transpose()
}

/// The pending link of this account a voter opened from the chat, by its
/// reference.
#[instrument(skip_all, err)]
pub async fn get_pending_messenger_link(
    tx: &Transaction<'_>,
    account_id: &Uuid,
    reference_digest: &str,
) -> Result<Option<MessengerLink>> {
    tx.query_opt(
        r#"
        SELECT * FROM sequent_backend.messenger_link
        WHERE account_id = $1 AND state = 'PENDING' AND reference_digest = $2
        FOR UPDATE
        "#,
        &[account_id, &reference_digest],
    )
    .await?
    .map(TryInto::try_into)
    .transpose()
}

/// Moves a link on. Leaving PENDING deletes its code: once handed to
/// Messenger, replaced or expired, Step no longer holds it.
#[instrument(skip(tx), err)]
pub async fn update_messenger_link(
    tx: &Transaction<'_>,
    id: &Uuid,
    state: MessengerLinkState,
    page_scoped_id: Option<&str>,
    message_id: Option<&Uuid>,
) -> Result<()> {
    tx.execute(
        r#"
        UPDATE sequent_backend.messenger_link
        SET state = $2,
            page_scoped_id = COALESCE($3, page_scoped_id),
            message_id = COALESCE($4, message_id),
            encrypted_payload = CASE WHEN $2 = 'PENDING'
                                     THEN encrypted_payload ELSE NULL END,
            updated_at = now()
        WHERE id = $1 AND ($2 <> 'CODE_SENT' OR state = 'PENDING')
        "#,
        &[id, &state.to_string(), &page_scoped_id, &message_id],
    )
    .await?;
    Ok(())
}

/// Expires unfinished links past their lifetime and deletes their codes.
#[instrument(skip(tx), err)]
pub async fn expire_messenger_links(tx: &Transaction<'_>, now: DateTime<Utc>) -> Result<u64> {
    Ok(tx
        .execute(
            r#"
            UPDATE sequent_backend.messenger_link
            SET state = 'EXPIRED', encrypted_payload = NULL, updated_at = now()
            WHERE state IN ('PENDING', 'CODE_SENT') AND expires_at <= $1
            "#,
            &[&now],
        )
        .await?)
}
