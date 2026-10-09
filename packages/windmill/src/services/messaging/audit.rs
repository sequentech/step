// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The electoral log's `SendCommunications` entry of a message attempt.
//!
//! Written once per attempt, when the attempt's outcome is first recorded
//! (accepted, failed or unknown): a replayed job records nothing new, so it
//! logs nothing new. The entry never holds content, codes, credentials or
//! the address: the receiver is always masked.
//!
//! The statement stays `SendCommunications(Option<String>)`, with these
//! fields in its JSON string, so earlier entries keep decoding.

use crate::postgres::election_event::get_election_event_by_id;
use crate::services::celery_app::get_celery_app;
use crate::services::election_event_board::get_election_event_board;
use crate::services::electoral_log::ElectoralLog;
use crate::tasks::electoral_log::{
    enqueue_electoral_log_event, LogEventBody, LogEventInput, LogMessageType,
};
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::types::messaging::{MessageAttemptState, MessageChannel, MessagePurpose};
use serde_json::{json, Map, Value};
use strum_macros::{Display, EnumString};
use tracing::{instrument, warn};
use uuid::Uuid;

/// What a message is, from its logical key.
#[allow(non_camel_case_types)]
#[derive(Display, EnumString, Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageKind {
    /// A notice of a bulk send (Send Notification).
    BULK_NOTICE,
    /// A code or notice Keycloak asked for (`/messages/send`).
    KEYCLOAK,
    /// A code sent to the Messenger conversation that opened a link.
    MESSENGER_CODE,
    /// An administrator's test message through one account.
    ACCOUNT_TEST,
    /// The result of a reviewed enrollment application.
    APPLICATION_RESULT,
    OTHER,
}

impl MessageKind {
    pub fn of(logical_key: &str) -> MessageKind {
        match logical_key.split(':').next().unwrap_or_default() {
            "send-template" => MessageKind::BULK_NOTICE,
            "keycloak" => MessageKind::KEYCLOAK,
            "messenger-link" => MessageKind::MESSENGER_CODE,
            "account-test" => MessageKind::ACCOUNT_TEST,
            "application" => MessageKind::APPLICATION_RESULT,
            _ => MessageKind::OTHER,
        }
    }
}

/// Provider name logged for the environment's email and SMS transport.
pub const ENVIRONMENT_PROVIDER: &str = "ENVIRONMENT";

/// One attempt whose outcome was just recorded.
#[derive(Debug, Clone, PartialEq)]
pub struct AttemptOutcome {
    pub tenant_id: Uuid,
    pub election_event_id: Option<Uuid>,
    pub voter_id: Option<String>,
    pub message_id: Uuid,
    pub attempt: i32,
    pub channel: MessageChannel,
    pub account_id: Option<Uuid>,
    /// The account's provider, or [`ENVIRONMENT_PROVIDER`].
    pub provider: String,
    pub purpose: MessagePurpose,
    pub kind: MessageKind,
    pub outcome: MessageAttemptState,
    /// The reason of a failed or unknown outcome.
    pub reason: Option<String>,
    /// The masked destination.
    pub receiver: String,
}

impl AttemptOutcome {
    /// The JSON string of the `SendCommunications` statement:
    /// `{channel, account_id, provider, purpose, kind, message_id, attempt,
    /// outcome, reason?, receiver}`. The channel is lowercase, as the
    /// first release wrote it.
    pub fn log_body(&self) -> String {
        let mut record = Map::new();
        record.insert(
            "channel".to_string(),
            json!(self.channel.to_string().to_lowercase()),
        );
        record.insert(
            "account_id".to_string(),
            json!(self.account_id.map(|id| id.to_string())),
        );
        record.insert("provider".to_string(), json!(self.provider));
        record.insert("purpose".to_string(), json!(self.purpose.to_string()));
        record.insert("kind".to_string(), json!(self.kind.to_string()));
        record.insert("message_id".to_string(), json!(self.message_id.to_string()));
        record.insert("attempt".to_string(), json!(self.attempt));
        record.insert("outcome".to_string(), json!(self.outcome.to_string()));
        if let Some(reason) = &self.reason {
            record.insert("reason".to_string(), json!(reason));
        }
        record.insert("receiver".to_string(), json!(self.receiver));
        Value::Object(record).to_string()
    }
}

/// Posts each outcome to its event's electoral log through the durable
/// electoral-log queue, signed with the event's system key. For sends with
/// no administrator: Keycloak's messages, Messenger codes, retries and
/// application results. Outcomes without an event aren't logged (there is
/// no event log to write to).
#[instrument(skip_all, err)]
pub async fn post_attempt_outcomes(
    hasura_transaction: &Transaction<'_>,
    outcomes: &[AttemptOutcome],
    username: Option<String>,
    area_id: Option<String>,
) -> Result<()> {
    for outcome in outcomes {
        let Some(event_id) = outcome.election_event_id else {
            continue;
        };
        let event_id = event_id.to_string();
        let tenant_id = outcome.tenant_id.to_string();
        let election_event = get_election_event_by_id(hasura_transaction, &tenant_id, &event_id)
            .await
            .with_context(|| "Error obtaining the election event")?;
        let board = get_election_event_board(election_event.bulletin_board_reference.clone())
            .ok_or_else(|| anyhow!("missing bulletin board"))?;
        let electoral_log =
            ElectoralLog::new(hasura_transaction, &tenant_id, Some(&event_id), &board).await?;
        let message = electoral_log.build_send_template_message(
            Some(outcome.log_body()),
            event_id.clone(),
            outcome.voter_id.clone(),
            username.clone(),
            None,
            area_id.clone(),
        )?;
        let input = LogEventInput {
            election_event_id: event_id,
            message_type: LogMessageType::Internal,
            user_id: outcome.voter_id.clone(),
            username: username.clone(),
            tenant_id,
            body: LogEventBody::Plain(
                serde_json::to_string(&message)
                    .with_context(|| "Error serializing the electoral log message")?,
            ),
            event_time_ms: None,
        };
        get_celery_app()
            .await
            .send_task(enqueue_electoral_log_event::new(input))
            .await?;
    }
    Ok(())
}

/// [`post_attempt_outcomes`] for callers that must not fail the send when
/// the log is unavailable: the error is logged instead.
pub async fn post_attempt_outcomes_or_warn(
    hasura_transaction: &Transaction<'_>,
    outcomes: &[AttemptOutcome],
    username: Option<String>,
    area_id: Option<String>,
) {
    if outcomes.is_empty() {
        return;
    }
    if let Err(error) = post_attempt_outcomes(hasura_transaction, outcomes, username, area_id).await
    {
        warn!("could not write message attempts to the electoral log: {error:#}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_come_from_the_logical_key() {
        assert_eq!(
            MessageKind::of("send-template:s1:v1"),
            MessageKind::BULK_NOTICE
        );
        assert_eq!(MessageKind::of("keycloak:abc"), MessageKind::KEYCLOAK);
        assert_eq!(
            MessageKind::of("messenger-link:1"),
            MessageKind::MESSENGER_CODE
        );
        assert_eq!(
            MessageKind::of("account-test:1:2"),
            MessageKind::ACCOUNT_TEST
        );
        assert_eq!(
            MessageKind::of("application:a1:ACCEPTED"),
            MessageKind::APPLICATION_RESULT
        );
        assert_eq!(MessageKind::of("other"), MessageKind::OTHER);
    }

    #[test]
    fn the_log_body_has_the_fields_and_never_the_address() {
        let outcome = AttemptOutcome {
            tenant_id: Uuid::nil(),
            election_event_id: Some(Uuid::nil()),
            voter_id: Some("voter-1".to_string()),
            message_id: Uuid::nil(),
            attempt: 2,
            channel: MessageChannel::WHATSAPP,
            account_id: Some(Uuid::nil()),
            provider: "WHATSAPP_CLOUD_API".to_string(),
            purpose: MessagePurpose::NOTICE,
            kind: MessageKind::BULK_NOTICE,
            outcome: MessageAttemptState::FAILED,
            reason: Some("rate limited by the provider".to_string()),
            receiver: "+63 \u{2022}\u{2022}\u{2022} \u{2022}\u{2022}\u{2022} 567".to_string(),
        };
        let body: Value = serde_json::from_str(&outcome.log_body()).unwrap();
        assert_eq!(
            body,
            json!({
                "channel": "whatsapp",
                "account_id": Uuid::nil().to_string(),
                "provider": "WHATSAPP_CLOUD_API",
                "purpose": "NOTICE",
                "kind": "BULK_NOTICE",
                "message_id": Uuid::nil().to_string(),
                "attempt": 2,
                "outcome": "FAILED",
                "reason": "rate limited by the provider",
                "receiver": "+63 \u{2022}\u{2022}\u{2022} \u{2022}\u{2022}\u{2022} 567",
            })
        );
        let accepted = AttemptOutcome {
            outcome: MessageAttemptState::ACCEPTED,
            reason: None,
            ..outcome
        };
        assert!(!accepted.log_body().contains("reason"));
    }
}
