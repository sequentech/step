// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The election event's messaging configuration, stored as the
//! `messaging:config` annotation.

use crate::postgres::election_event::update_election_event_annotations;
use crate::postgres::messaging::{list_messaging_accounts, MessagingAccount};
use anyhow::{Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::types::messaging::{
    AccountSummary, EventMessagingConfig, MessagingConfigError, PublicMessagingChannels,
    MESSAGING_CONFIG_ANNOTATION,
};
use serde_json::{Map, Value};
use uuid::Uuid;

pub fn account_summary(account: &MessagingAccount) -> AccountSummary {
    AccountSummary {
        id: account.id.to_string(),
        tenant_id: account.tenant_id.to_string(),
        channel: account.channel,
        provider: account.provider,
        provider_approval: account.provider_approval,
        check: account.status.clone(),
        public_label: account.sender.public_label(),
        messenger_page: account.sender.messenger_page(),
    }
}

async fn event_annotations(
    tx: &Transaction<'_>,
    tenant_id: &Uuid,
    election_event_id: &Uuid,
) -> Result<Option<Map<String, Value>>> {
    let row = tx
        .query_opt(
            r#"
            SELECT annotations FROM sequent_backend.election_event
            WHERE tenant_id = $1 AND id = $2
            "#,
            &[tenant_id, election_event_id],
        )
        .await?;
    Ok(row.map(|row| {
        row.get::<_, Option<Value>>("annotations")
            .and_then(|value| value.as_object().cloned())
            .unwrap_or_default()
    }))
}

/// The event's configuration, or `None` when it has none yet. Events
/// without one keep sending email and SMS as before.
pub async fn get_event_messaging_config(
    tx: &Transaction<'_>,
    tenant_id: &Uuid,
    election_event_id: &Uuid,
) -> Result<Option<EventMessagingConfig>> {
    let Some(annotations) = event_annotations(tx, tenant_id, election_event_id).await? else {
        return Ok(None);
    };
    annotations
        .get(MESSAGING_CONFIG_ANNOTATION)
        .and_then(Value::as_str)
        .map(|raw| serde_json::from_str(raw).context("invalid messaging:config annotation"))
        .transpose()
}

async fn election_ids(
    tx: &Transaction<'_>,
    tenant_id: &Uuid,
    election_event_id: &Uuid,
) -> Result<Vec<String>> {
    Ok(tx
        .query(
            r#"
            SELECT id FROM sequent_backend.election
            WHERE tenant_id = $1 AND election_event_id = $2
            "#,
            &[tenant_id, election_event_id],
        )
        .await?
        .into_iter()
        .map(|row| row.get::<_, Uuid>("id").to_string())
        .collect())
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
    let Some(mut annotations) = event_annotations(tx, tenant_id, election_event_id).await? else {
        return Ok(SaveOutcome::UnknownEvent);
    };
    let accounts: Vec<AccountSummary> = list_messaging_accounts(tx, tenant_id)
        .await?
        .iter()
        .map(account_summary)
        .collect();
    let elections = election_ids(tx, tenant_id, election_event_id).await?;
    if let Err(errors) = config.validate(&tenant_id.to_string(), &accounts, &elections) {
        return Ok(SaveOutcome::Invalid(errors));
    }
    annotations.insert(
        MESSAGING_CONFIG_ANNOTATION.to_string(),
        Value::String(serde_json::to_string(config)?),
    );
    update_election_event_annotations(
        tx,
        &tenant_id.to_string(),
        &election_event_id.to_string(),
        Value::Object(annotations),
    )
    .await?;
    Ok(SaveOutcome::Saved(config.public_projection(&accounts)))
}
