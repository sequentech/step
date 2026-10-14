// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The requests an administrator makes outside any election event, as entries
//! of the tenant's log (see `electoral_log_board::ensure_tenant_log`). Harvest
//! reports each request with its outcome, including the refused ones; Windmill
//! signs it with the log's system key.

use crate::services::electoral_log::ElectoralLog;
use anyhow::Result;
use electoral_log::messages::message::Message;
use electoral_log::messages::newtypes::{
    TenantActivityEntry, TenantActivityKind, TenantIdString, Timestamp,
};
use electoral_log::messages::statement::{StatementEventType, StatementLogType};
use electoral_log::ElectoralLogMessage;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use strum_macros::{Display, EnumString};

/// Longest description of an entry, in characters; the details keep the rest.
pub const MAX_DESCRIPTION_CHARS: usize = 200;

/// Whether an event names no election event: an empty id, or the `"null"` the
/// Keycloak listener sends for a missing value.
pub fn names_no_election_event(election_event_id: &str) -> bool {
    matches!(election_event_id.trim(), "" | "null")
}

/// How a request an administrator made ended.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Display, EnumString)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum TenantActivityOutcome {
    /// The platform did what was asked.
    Succeeded,
    /// The caller was not allowed to ask for it.
    Denied,
    /// The request was invalid or the platform could not complete it.
    Failed,
}

/// A request an administrator made to the platform outside any election event,
/// as the service that answered it reports it.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct TenantActivity {
    /// What was asked for, such as `delete-election-event`.
    pub action: String,
    /// A short English sentence for the Logs tab, such as "Deleted an election
    /// event".
    pub description: String,
    pub outcome: TenantActivityOutcome,
    /// The HTTP status the request was answered with.
    pub status: u16,
    /// What the request was made on, when the service says.
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub details: Value,
}

/// The entry of a request an administrator made, signed by the tenant log's
/// `signer`. The actor is part of the signed details; a request that did not
/// succeed is an ERROR entry.
pub fn tenant_activity_message(
    signer: &ElectoralLog,
    tenant_id: &str,
    activity: &TenantActivity,
    occurred_at: Timestamp,
    user_id: Option<String>,
    username: Option<String>,
) -> Result<ElectoralLogMessage> {
    let log_type = match activity.outcome {
        TenantActivityOutcome::Succeeded => StatementLogType::INFO,
        TenantActivityOutcome::Denied | TenantActivityOutcome::Failed => StatementLogType::ERROR,
    };
    let mut details = json!({
        "action": activity.action,
        "outcome": activity.outcome,
        "status": activity.status,
        "user_id": user_id,
        "username": username,
    });
    if !activity.details.is_null() {
        details["details"] = activity.details.clone();
    }
    let entry = TenantActivityEntry {
        tenant_id: TenantIdString(tenant_id.to_string()),
        kind: TenantActivityKind::TenantAdminAction,
        event_type: StatementEventType::USER,
        log_type,
        description: activity
            .description
            .chars()
            .take(MAX_DESCRIPTION_CHARS)
            .collect(),
        details_json: details.to_string(),
    };
    let message =
        Message::tenant_activity_message(entry, occurred_at, &signer.sd, user_id, username)?;
    (&message).try_into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::electoral_log_board::get_tenant_board;
    use electoral_log::messages::message::GENERIC_EVENT;
    use electoral_log::messages::statement::StatementBody;
    use voting_crypto::signing::SigningKey;
    use voting_crypto::signing::VerificationKey;
    use voting_crypto::RecordDeserialize;

    const TENANT: &str = "90505c8a-23a9-4cdf-a26b-4e19f6a097d5";

    fn log() -> (ElectoralLog, VerificationKey) {
        let key = SigningKey::generate().unwrap();
        let public = VerificationKey::from_sk(&key).unwrap();
        let board = get_tenant_board(TENANT, "dev");
        (
            ElectoralLog::for_system_with_signing_key(&board, &key),
            public,
        )
    }

    fn activity(outcome: TenantActivityOutcome, status: u16) -> TenantActivity {
        TenantActivity {
            action: "delete-election-event".into(),
            description: "Deleted an election event".into(),
            outcome,
            status,
            details: json!({"election_event_id": "fdd21db2-dd68-4974-90eb-7f2750b2b5df"}),
        }
    }

    /// The signed entry of a stored record, checked against the log's key.
    fn signed(stored: &ElectoralLogMessage, public: &VerificationKey) -> TenantActivityEntry {
        let message = Message::record_deserialize(&stored.message).unwrap();
        message.verify(public).unwrap();
        assert_eq!(message.statement.head.event.0, GENERIC_EVENT);
        match message.statement.body {
            StatementBody::TenantActivity(entry) => entry,
            other => panic!("unexpected body {other:?}"),
        }
    }

    #[test]
    fn empty_and_null_ids_name_no_election_event() {
        for id in ["", " ", "null"] {
            assert!(names_no_election_event(id), "{id:?}");
        }
        for id in ["fdd21db2-dd68-4974-90eb-7f2750b2b5df", "nullish", "event"] {
            assert!(!names_no_election_event(id), "{id:?}");
        }
    }

    #[test]
    fn a_request_is_a_signed_user_entry_with_its_actor_and_outcome() {
        let (log, public) = log();
        let stored = tenant_activity_message(
            &log,
            TENANT,
            &activity(TenantActivityOutcome::Succeeded, 200),
            1_800_000_000,
            Some("admin-id".into()),
            Some("admin".into()),
        )
        .unwrap();
        assert_eq!(stored.statement_kind, "TenantAdminAction");
        assert_eq!(stored.statement_timestamp, 1_800_000_000);
        assert_eq!(stored.user_id.as_deref(), Some("admin-id"));
        assert_eq!(stored.username.as_deref(), Some("admin"));
        assert_eq!(stored.election_id, None);

        let entry = signed(&stored, &public);
        assert_eq!(entry.tenant_id.0, TENANT);
        assert_eq!(entry.kind, TenantActivityKind::TenantAdminAction);
        assert_eq!(entry.event_type, StatementEventType::USER);
        assert_eq!(entry.log_type, StatementLogType::INFO);
        assert_eq!(entry.description, "Deleted an election event");
        let details: Value = serde_json::from_str(&entry.details_json).unwrap();
        assert_eq!(
            details,
            json!({
                "action": "delete-election-event",
                "outcome": "succeeded",
                "status": 200,
                "user_id": "admin-id",
                "username": "admin",
                "details": {"election_event_id": "fdd21db2-dd68-4974-90eb-7f2750b2b5df"},
            })
        );
    }

    #[test]
    fn a_request_that_did_not_succeed_is_an_error_entry() {
        let (log, public) = log();
        for (outcome, status, name) in [
            (TenantActivityOutcome::Denied, 403, "denied"),
            (TenantActivityOutcome::Failed, 500, "failed"),
        ] {
            let mut request = activity(outcome, status);
            request.details = Value::Null;
            let stored = tenant_activity_message(&log, TENANT, &request, 1, None, None).unwrap();
            let entry = signed(&stored, &public);
            assert_eq!(entry.log_type, StatementLogType::ERROR);
            let details: Value = serde_json::from_str(&entry.details_json).unwrap();
            assert_eq!(details["outcome"], name);
            assert_eq!(details["status"], status);
            assert_eq!(details["user_id"], Value::Null);
            assert!(details.get("details").is_none());
            assert_eq!(outcome.to_string(), name);
            assert_eq!(name.parse::<TenantActivityOutcome>().unwrap(), outcome);
        }
    }

    #[test]
    fn a_long_description_is_cut_and_the_details_keep_the_action() {
        let (log, public) = log();
        let mut request = activity(TenantActivityOutcome::Succeeded, 200);
        request.description = "é".repeat(MAX_DESCRIPTION_CHARS + 50);
        let stored = tenant_activity_message(&log, TENANT, &request, 1, None, None).unwrap();
        let entry = signed(&stored, &public);
        assert_eq!(entry.description.chars().count(), MAX_DESCRIPTION_CHARS);
        assert!(entry.details_json.contains("delete-election-event"));
    }

    #[test]
    fn an_activity_parses_without_details() {
        let parsed: TenantActivity = serde_json::from_str(
            r#"{"action":"create-role","description":"Created a role","outcome":"succeeded","status":200}"#,
        )
        .unwrap();
        assert_eq!(parsed.details, Value::Null);
        assert!(!serde_json::to_string(&parsed).unwrap().contains("details"));
        assert!(serde_json::from_str::<TenantActivity>(
            r#"{"action":"x","description":"x","outcome":"unheard-of","status":200}"#
        )
        .is_err());
    }
}
