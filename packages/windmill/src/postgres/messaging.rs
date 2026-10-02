// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Sending accounts, the message ledger and Messenger links.

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use deadpool_postgres::Transaction;
use sequent_core::types::messaging::{
    AccountCheck, AccountLimits, AccountSender, CredentialName, CredentialRecord,
    MessageAttemptState, MessageChannel, MessageDirection, MessagePurpose, MessagingProvider,
    MessengerLinkState, ProviderApproval,
};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::collections::BTreeMap;
use std::str::FromStr;
use strum::IntoEnumIterator;
use tokio_postgres::row::Row;
use tracing::instrument;
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
    pub status: AccountCheck,
    pub webhook_key: String,
    pub is_default: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
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
            status: if status.as_object().is_some_and(|o| o.is_empty()) {
                AccountCheck::default()
            } else {
                serde_json::from_value(status).context("unexpected status")?
            },
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
    pub is_default: bool,
}

impl AccountSettings {
    fn validate(&self) -> Result<()> {
        let provider = self.sender.provider();
        if provider.capabilities(self.channel).is_none() {
            return Err(anyhow!("{provider} cannot send {}", self.channel));
        }
        if self.name.trim().is_empty() {
            return Err(anyhow!("the account needs a name"));
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
                 provider_approval, webhook_key, is_default)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
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
            ],
        )
        .await?;
    row.try_into()
}

/// Updates an account. Changing the provider is refused: its credentials
/// would not apply.
#[instrument(skip(tx, settings), err)]
pub async fn update_messaging_account(
    tx: &Transaction<'_>,
    tenant_id: &Uuid,
    id: &Uuid,
    settings: &AccountSettings,
) -> Result<Option<MessagingAccount>> {
    settings.validate()?;
    let Some(current) = get_messaging_account(tx, tenant_id, id).await? else {
        return Ok(None);
    };
    if current.provider != settings.sender.provider() || current.channel != settings.channel {
        return Err(anyhow!(
            "the provider and channel of an account cannot change; add another account"
        ));
    }
    if settings.is_default {
        clear_other_defaults(tx, tenant_id, settings.channel, id).await?;
    }
    let row = tx
        .query_opt(
            r#"
            UPDATE sequent_backend.messaging_account
            SET name = $3, sender = $4, limits = $5, provider_approval = $6,
                is_default = $7, updated_at = now()
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
            ],
        )
        .await?;
    row.map(TryInto::try_into).transpose()
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
        SET status = $3, updated_at = now()
        WHERE tenant_id = $1 AND id = $2
        "#,
        &[tenant_id, id, &serde_json::to_value(status)?],
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

#[derive(Debug, Clone, PartialEq)]
pub struct MessageRecord {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub election_event_id: Option<Uuid>,
    pub voter_id: Option<String>,
    pub account_id: Option<Uuid>,
    pub channel: MessageChannel,
    pub direction: MessageDirection,
    pub purpose: Option<MessagePurpose>,
    pub template_alias: Option<String>,
    pub language: Option<String>,
    pub masked_destination: String,
    pub destination_digest: String,
    pub destination_country: Option<String>,
    pub logical_key: Option<String>,
    pub attempt: i32,
    pub state: MessageAttemptState,
    pub provider_message_id: Option<String>,
    pub error: Option<String>,
    pub billing: Option<Value>,
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
            channel: parse_enum(&row, "channel")?,
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
            billing: row.try_get("billing")?,
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
    pub channel: MessageChannel,
    pub direction: MessageDirection,
    pub purpose: Option<MessagePurpose>,
    pub template_alias: Option<String>,
    pub language: Option<String>,
    pub masked_destination: String,
    pub destination_digest: String,
    pub destination_country: Option<String>,
    pub logical_key: Option<String>,
    pub attempt: i32,
    pub state: MessageAttemptState,
    pub provider_message_id: Option<String>,
}

/// Inserts a message. With a logical key, a second insert of the same
/// attempt returns `None` instead of a duplicate.
#[instrument(skip(tx, message), err)]
pub async fn insert_message(
    tx: &Transaction<'_>,
    message: &NewMessage,
) -> Result<Option<MessageRecord>> {
    tx.query_opt(
        r#"
        INSERT INTO sequent_backend.message
            (tenant_id, election_event_id, voter_id, account_id, channel, direction,
             purpose, template_alias, language, masked_destination, destination_digest,
             destination_country, logical_key, attempt, state, provider_message_id)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
        ON CONFLICT (tenant_id, logical_key, attempt) DO NOTHING
        RETURNING *
        "#,
        &[
            &message.tenant_id,
            &message.election_event_id,
            &message.voter_id,
            &message.account_id,
            &message.channel.to_string(),
            &message.direction.to_string(),
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
        ],
    )
    .await?
    .map(TryInto::try_into)
    .transpose()
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
    pub provider_message_id: Option<String>,
    pub error: Option<String>,
    pub billing: Option<Value>,
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
            provider_message_id = COALESCE($4, provider_message_id),
            error = COALESCE($5, error),
            billing = COALESCE($6, billing),
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
        ],
    )
    .await?
    .map(TryInto::try_into)
    .transpose()
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

/// Whether an inbound message with this provider ID was already recorded.
#[instrument(skip(tx), err)]
pub async fn inbound_exists(
    tx: &Transaction<'_>,
    account_id: &Uuid,
    provider_message_id: &str,
) -> Result<bool> {
    Ok(tx
        .query_opt(
            r#"
            SELECT 1 FROM sequent_backend.message
            WHERE account_id = $1 AND provider_message_id = $2 AND direction = 'INBOUND'
            "#,
            &[account_id, &provider_message_id],
        )
        .await?
        .is_some())
}

/// When the recipient last wrote to the account.
#[instrument(skip(tx), err)]
pub async fn last_inbound_at(
    tx: &Transaction<'_>,
    account_id: &Uuid,
    destination_digest: &str,
) -> Result<Option<DateTime<Utc>>> {
    Ok(tx
        .query_one(
            r#"
            SELECT max(created_at) AS at FROM sequent_backend.message
            WHERE account_id = $1 AND destination_digest = $2 AND direction = 'INBOUND'
            "#,
            &[account_id, &destination_digest],
        )
        .await?
        .try_get("at")?)
}

/// When the account last sent an automatic reply to this recipient.
#[instrument(skip(tx), err)]
pub async fn last_auto_reply_at(
    tx: &Transaction<'_>,
    account_id: &Uuid,
    destination_digest: &str,
) -> Result<Option<DateTime<Utc>>> {
    Ok(tx
        .query_one(
            r#"
            SELECT max(created_at) AS at FROM sequent_backend.message
            WHERE account_id = $1 AND destination_digest = $2
              AND direction = 'OUTBOUND' AND template_alias = $3
            "#,
            &[account_id, &destination_digest, &AUTO_REPLY_ALIAS],
        )
        .await?
        .try_get("at")?)
}

/// `template_alias` of automatic replies to incoming messages.
pub const AUTO_REPLY_ALIAS: &str = "auto-reply";

/// Attempts whose outcome is not known yet: unknown ones, and queued ones
/// older than `stale_before`, which a worker may have dispatched before it
/// stopped.
#[instrument(skip(tx), err)]
pub async fn list_unresolved_messages(
    tx: &Transaction<'_>,
    stale_before: DateTime<Utc>,
    limit: i64,
) -> Result<Vec<MessageRecord>> {
    tx.query(
        r#"
        SELECT * FROM sequent_backend.message
        WHERE direction = 'OUTBOUND'
          AND (state = 'UNKNOWN' OR (state = 'QUEUED' AND created_at < $1))
        ORDER BY created_at
        LIMIT $2
        "#,
        &[&stale_before, &limit],
    )
    .await?
    .into_iter()
    .map(TryInto::try_into)
    .collect()
}

#[derive(Debug, Clone, PartialEq)]
pub struct MessengerLink {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub election_event_id: Option<Uuid>,
    pub account_id: Uuid,
    pub reference_digest: String,
    pub link_word_digest: String,
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
            link_word_digest: row.try_get("link_word_digest")?,
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
    pub link_word_digest: String,
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
            (tenant_id, election_event_id, account_id, reference_digest, link_word_digest,
             auth_session_digest, challenge_digest, encrypted_payload, language, state,
             expires_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'PENDING', $10)
        RETURNING *
        "#,
        &[
            &link.tenant_id,
            &link.election_event_id,
            &link.account_id,
            &link.reference_digest,
            &link.link_word_digest,
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

/// The pending link of this account a voter starts from the chat, by its
/// reference or its link word.
#[instrument(skip_all, err)]
pub async fn get_pending_messenger_link(
    tx: &Transaction<'_>,
    account_id: &Uuid,
    reference_digest: Option<&str>,
    link_word_digest: Option<&str>,
) -> Result<Option<MessengerLink>> {
    tx.query_opt(
        r#"
        SELECT * FROM sequent_backend.messenger_link
        WHERE account_id = $1 AND state = 'PENDING'
          AND (reference_digest = $2 OR link_word_digest = $3)
        FOR UPDATE
        "#,
        &[account_id, &reference_digest, &link_word_digest],
    )
    .await?
    .map(TryInto::try_into)
    .transpose()
}

/// Moves a link on. Leaving PENDING or CODE_SENT deletes its code.
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
            encrypted_payload = CASE WHEN $2 IN ('PENDING', 'CODE_SENT')
                                     THEN encrypted_payload ELSE NULL END,
            updated_at = now()
        WHERE id = $1
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
