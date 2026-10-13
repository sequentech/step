// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The tenant's log: signed entries of the activity that belongs to no election
//! event, such as an administrator signing in or creating an election event. It
//! is a board of the electoral-log server's base database with a signing key of
//! its own, created on first use.

use crate::postgres::tenant::get_tenant_by_id_if_exist;
use crate::services::database::get_hasura_pool;
use crate::services::electoral_log::ElectoralLog;
use crate::services::electoral_log_board::{
    get_event_databases, get_protocol_manager_secret_path, parse_signing_identity,
};
use crate::services::vault;
use anyhow::{Context, Result};
use deadpool_postgres::{Client as DbClient, Transaction};
use electoral_log::messages::message::{Message, GENERIC_EVENT};
use electoral_log::messages::newtypes::{
    ErrorMessageString, EventIdString, KeycloakEventTypeString, TenantActivityEntry,
    TenantActivityKind, TenantIdString, Timestamp,
};
use electoral_log::messages::statement::{StatementEventType, StatementLogType};
use electoral_log::{BoardClient, ElectoralLogMessage, LogEntry, LogQuery};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use strum_macros::{Display, EnumString};
use tokio::sync::OnceCell;
use tracing::instrument;
use voting_crypto::signing::SigningKey;

/// Delivery id of a tenant log's first entry: one per log, however often the log
/// is opened.
pub const TENANT_LOG_OPENED_DELIVERY_ID: &str = "tenant-log:opened";
/// Longest description of an entry, in characters; the details keep the rest.
pub const MAX_DESCRIPTION_CHARS: usize = 200;

static TENANT_LOG_CLIENT: OnceCell<BoardClient> = OnceCell::const_new();

/// The tenants' logs, in the electoral-log server's base database.
pub async fn get_tenant_log_client() -> Result<BoardClient> {
    let client = TENANT_LOG_CLIENT
        .get_or_try_init(|| async {
            let store = get_event_databases().await?.tenant_logs().await?;
            Ok::<_, anyhow::Error>(BoardClient::new(Arc::new(store)))
        })
        .await?;
    Ok(client.clone())
}

/// Name of a tenant's log. Unlike an election event's board it names the whole
/// tenant id and no event, so it is never routed to an event's database.
pub fn get_tenant_board(tenant_id: &str, slug: &str) -> String {
    format!("{slug}tenant{tenant_id}")
        .chars()
        .filter(|&c| c != '-')
        .collect()
}

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

/// A tenant's log and its signing key.
pub struct TenantLog {
    tenant_id: String,
    signer: ElectoralLog,
}

impl TenantLog {
    /// The log of `tenant_id` on `board`, signing with an already loaded key.
    pub fn with_signing_key(tenant_id: &str, board: &str, signing_key: &SigningKey) -> Self {
        Self {
            tenant_id: tenant_id.to_string(),
            signer: ElectoralLog::for_system_with_signing_key(board, signing_key),
        }
    }

    pub fn board(&self) -> &str {
        &self.signer.elog_database
    }

    fn entry_message(
        &self,
        entry: TenantActivityEntry,
        timestamp: Timestamp,
        user_id: Option<String>,
        username: Option<String>,
    ) -> Result<ElectoralLogMessage> {
        let message =
            Message::tenant_activity_message(entry, timestamp, &self.signer.sd, user_id, username)?;
        (&message).try_into()
    }

    /// The log's first entry.
    pub fn opened_entry(&self) -> Result<LogEntry> {
        let entry = TenantActivityEntry {
            tenant_id: TenantIdString(self.tenant_id.clone()),
            kind: TenantActivityKind::TenantLogOpened,
            event_type: StatementEventType::SYSTEM,
            log_type: StatementLogType::INFO,
            description: "Opened the tenant log.".to_string(),
            details_json: json!({
                "board": self.board(),
                "public_key": self.signer.sd.system_public_key()?.to_b64_string()?,
            })
            .to_string(),
        };
        Ok(LogEntry {
            delivery_id: TENANT_LOG_OPENED_DELIVERY_ID.to_string(),
            message: self.entry_message(entry, electoral_log::timestamp(), None, None)?,
        })
    }

    /// The entry of a request an administrator made. The actor is part of the
    /// signed details; a request that did not succeed is an ERROR entry.
    pub fn activity_message(
        &self,
        activity: &TenantActivity,
        occurred_at: Timestamp,
        user_id: Option<String>,
        username: Option<String>,
    ) -> Result<ElectoralLogMessage> {
        let log_type = match activity.outcome {
            TenantActivityOutcome::Succeeded => StatementLogType::INFO,
            TenantActivityOutcome::Denied | TenantActivityOutcome::Failed => {
                StatementLogType::ERROR
            }
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
            tenant_id: TenantIdString(self.tenant_id.clone()),
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
        self.entry_message(entry, occurred_at, user_id, username)
    }

    /// The entry of a Keycloak event of the tenant's realm, such as an
    /// administrator signing in.
    pub fn keycloak_event_message(
        &self,
        event_type: &str,
        error_message: String,
        user_id: Option<String>,
        username: Option<String>,
    ) -> Result<ElectoralLogMessage> {
        let message = Message::keycloak_user_event(
            EventIdString(GENERIC_EVENT.to_string()),
            KeycloakEventTypeString(event_type.to_string()),
            ErrorMessageString(error_message),
            user_id,
            username,
            &self.signer.sd,
            None,
        )?;
        (&message).try_into()
    }
}

/// The signing key of a tenant's log, created in `transaction` the first time it
/// is asked for. `None` if the tenant does not exist.
pub async fn tenant_log_signing_key(
    transaction: &Transaction<'_>,
    tenant_id: &str,
    board: &str,
) -> Result<Option<SigningKey>> {
    #[derive(Serialize)]
    struct SigningIdentity {
        signing_key: String,
    }
    if get_tenant_by_id_if_exist(transaction, tenant_id)
        .await?
        .is_none()
    {
        return Ok(None);
    }
    let path = get_protocol_manager_secret_path(board);
    // One process creates the key; the others wait and read it.
    transaction
        .execute(
            "SELECT pg_advisory_xact_lock(hashtextextended($1, 0))",
            &[&path],
        )
        .await?;
    let signing_key = match vault::read_secret(transaction, tenant_id, None, &path).await? {
        Some(contents) => parse_signing_identity(&contents)?,
        None => {
            let signing_key = SigningKey::generate()?;
            let identity = toml::to_string(&SigningIdentity {
                signing_key: signing_key.to_b64_string()?,
            })?;
            vault::save_secret(transaction, tenant_id, None, &path, &identity).await?;
            signing_key
        }
    };
    Ok(Some(signing_key))
}

/// Create a tenant's board if it is missing, and its first entry if it has none.
pub async fn ensure_tenant_board(client: &BoardClient, log: &TenantLog) -> Result<()> {
    if !client.has_board(log.board()).await? {
        client.create_board(log.board()).await?;
    }
    if client.count(log.board(), &LogQuery::default()).await? == 0 {
        client.append(log.board(), &[log.opened_entry()?]).await?;
    }
    Ok(())
}

/// A tenant's log, created with its signing key and its first entry if it has
/// none yet. `None` if the tenant does not exist.
///
/// The key is committed before the board is created, so an entry is never signed
/// with a key that was not kept.
#[instrument(err)]
pub async fn open_tenant_log(tenant_id: &str) -> Result<Option<TenantLog>> {
    let slug = std::env::var("ENV_SLUG").context("missing env var ENV_SLUG")?;
    let board = get_tenant_board(tenant_id, &slug);
    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .context("Error getting a connection for the tenant log")?;
    let transaction = hasura_db_client.transaction().await?;
    let Some(signing_key) = tenant_log_signing_key(&transaction, tenant_id, &board).await? else {
        return Ok(None);
    };
    transaction.commit().await?;
    let log = TenantLog::with_signing_key(tenant_id, &board, &signing_key);
    ensure_tenant_board(&get_tenant_log_client().await?, &log).await?;
    Ok(Some(log))
}

#[cfg(test)]
mod tests {
    use super::*;
    use electoral_log::messages::statement::StatementBody;
    use voting_crypto::signing::VerificationKey;
    use voting_crypto::RecordDeserialize;

    const TENANT: &str = "90505c8a-23a9-4cdf-a26b-4e19f6a097d5";

    fn log() -> (TenantLog, VerificationKey) {
        let key = SigningKey::generate().unwrap();
        let public = VerificationKey::from_sk(&key).unwrap();
        let board = get_tenant_board(TENANT, "dev");
        (TenantLog::with_signing_key(TENANT, &board, &key), public)
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
    fn a_tenant_board_names_the_whole_tenant_and_no_event() {
        let board = get_tenant_board(TENANT, "dev");
        assert_eq!(board, "devtenant90505c8a23a94cdfa26b4e19f6a097d5");
        assert!(electoral_log::adapters::events::event_of_board(&board).is_err());
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
        let stored = log
            .activity_message(
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
            let stored = log.activity_message(&request, 1, None, None).unwrap();
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
        let stored = log.activity_message(&request, 1, None, None).unwrap();
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

    #[test]
    fn the_first_entry_is_a_system_entry_with_the_logs_key() {
        let (log, public) = log();
        let opened = log.opened_entry().unwrap();
        assert_eq!(opened.delivery_id, TENANT_LOG_OPENED_DELIVERY_ID);
        assert_eq!(opened.message.statement_kind, "TenantLogOpened");
        assert_eq!(opened.message.user_id, None);
        let entry = signed(&opened.message, &public);
        assert_eq!(entry.event_type, StatementEventType::SYSTEM);
        assert_eq!(entry.log_type, StatementLogType::INFO);
        let details: Value = serde_json::from_str(&entry.details_json).unwrap();
        assert_eq!(details["board"], log.board());
        assert_eq!(details["public_key"], public.to_b64_string().unwrap());
    }

    #[test]
    fn a_keycloak_event_of_the_tenant_realm_names_no_election_event() {
        let (log, public) = log();
        let stored = log
            .keycloak_event_message(
                "LOGIN_ERROR",
                "invalid_user_credentials".into(),
                Some("admin-id".into()),
                Some("admin".into()),
            )
            .unwrap();
        assert_eq!(stored.statement_kind, "KeycloakUserEvent");
        assert_eq!(stored.area_id, None);
        let message = Message::record_deserialize(&stored.message).unwrap();
        message.verify(&public).unwrap();
        assert_eq!(message.statement.head.event.0, GENERIC_EVENT);
        assert_eq!(message.statement.head.log_type, StatementLogType::ERROR);
        assert_eq!(
            message.statement.head.description,
            "LOGIN_ERROR invalid_user_credentials"
        );
    }
}
