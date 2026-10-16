// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Audit entries of administrative actions.
//!
//! A route or a task that changes state queues its entry with [`stage`] in
//! the transaction of the change, so the entry exists exactly when the change
//! commits. The outbox worker signs and posts it afterwards: to the election
//! event's log when the change is inside an event, to the tenant's log
//! otherwise. Changes made through Hasura are queued by the database trigger
//! `sequent_backend.audit_change()`.

use crate::postgres::audit_log_outbox::{
    insert_audit_log_outbox, AuditLogTarget, NewAuditLogEntry,
};
use anyhow::{Context, Result};
use deadpool_postgres::Transaction;
use electoral_log::messages::newtypes::AuditStatementKind;
use electoral_log::messages::statement::{StatementEventType, StatementLogType};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use strum_macros::{Display, EnumString};
use tracing::instrument;
use uuid::Uuid;

/// Texts longer than this are recorded as their SHA-256 before and after.
pub const LONG_TEXT_BYTES: usize = 64 * 1024;
/// Paths a description names before it counts the rest.
const DESCRIBED_PATHS: usize = 5;
/// A path segment that ends in one of these words names a secret. The
/// trigger's `sequent_backend.audit_is_secret` applies the same rule.
const SECRET_ENDINGS: [&str; 15] = [
    "password",
    "passwords",
    "passwd",
    "pwd",
    "passphrase",
    "secret",
    "secrets",
    "token",
    "tokens",
    "credential",
    "credentials",
    "authorization",
    "apikey",
    "apikeys",
    "access_key_id",
];
/// A segment that ends in `key` or `keys` after one of these names a secret.
const SECRET_KEY_QUALIFIERS: [&str; 10] = [
    "api",
    "gapi",
    "private",
    "secret",
    "signing",
    "access",
    "auth",
    "client",
    "encryption",
    "webhook",
];
/// What a secret's name may end with and still name the secret.
const SECRET_DERIVATIVES: [&str; 2] = ["_hash", "_salt"];

/// What was done to an object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Display, EnumString)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum AdminAction {
    Create,
    Update,
    Delete,
    Import,
    Export,
    Archive,
    Unarchive,
    Enable,
    Disable,
    ResetPassword,
    Grant,
    Revoke,
}

impl AdminAction {
    /// The verb that opens a description.
    fn past_tense(self) -> &'static str {
        match self {
            Self::Create => "Created",
            Self::Update => "Updated",
            Self::Delete => "Deleted",
            Self::Import => "Imported",
            Self::Export => "Exported",
            Self::Archive => "Archived",
            Self::Unarchive => "Unarchived",
            Self::Enable => "Enabled",
            Self::Disable => "Disabled",
            Self::ResetPassword => "Reset the password of",
            Self::Grant => "Granted",
            Self::Revoke => "Revoked",
        }
    }
}

/// Where a change was made.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Display, EnumString)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum AuditOrigin {
    Hasura,
    Harvest,
    Windmill,
}

/// Who made a change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct AuditActor {
    pub user_id: Option<String>,
    pub username: Option<String>,
    pub role: Option<String>,
}

/// The object a change is about. `kind` is a table name without its schema
/// (`tenant`, `election_event`, `template`...), or `staff_account`, `role`,
/// `tenant_log`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditObject {
    #[serde(rename = "type")]
    pub kind: String,
    pub id: String,
    pub name: Option<String>,
}

impl AuditObject {
    pub fn new(kind: &str, id: impl ToString, name: Option<&str>) -> Self {
        AuditObject {
            kind: kind.to_string(),
            id: id.to_string(),
            name: name.map(str::to_string),
        }
    }
}

/// One changed field. A secret never carries its values; a long text carries
/// only its SHA-256 before and after.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldChange {
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub old: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub new: Option<Value>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub secret: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub old_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub new_sha256: Option<String>,
}

fn is_false(value: &bool) -> bool {
    !*value
}

/// A path segment as snake_case words: camelCase is split, case is ignored
/// and anything that is not a letter or a digit separates words.
fn snake_words(segment: &str) -> String {
    let mut words = String::with_capacity(segment.len() + 4);
    let mut previous_lower = false;
    for character in segment.chars() {
        if !character.is_ascii_alphanumeric() {
            if !words.ends_with('_') {
                words.push('_');
            }
            previous_lower = false;
            continue;
        }
        if character.is_ascii_uppercase() && previous_lower {
            words.push('_');
        }
        previous_lower = character.is_ascii_lowercase() || character.is_ascii_digit();
        words.push(character.to_ascii_lowercase());
    }
    words.trim_matches('_').to_string()
}

fn is_secret_segment(segment: &str) -> bool {
    let words = snake_words(segment);
    let name = SECRET_DERIVATIVES
        .iter()
        .find_map(|suffix| words.strip_suffix(suffix))
        .unwrap_or(&words);
    let ends_with_word = |word: &str| {
        name.strip_suffix(word)
            .is_some_and(|before| before.is_empty() || before.ends_with('_'))
    };
    if SECRET_ENDINGS.iter().any(|word| ends_with_word(word)) {
        return true;
    }
    ["_key", "_keys"].iter().any(|key| {
        name.strip_suffix(key).is_some_and(|before| {
            SECRET_KEY_QUALIFIERS.iter().any(|qualifier| {
                before
                    .strip_suffix(qualifier)
                    .is_some_and(|rest| rest.is_empty() || rest.ends_with('_'))
            })
        })
    })
}

/// Whether a dotted path names a secret: one of its segments does. Settings
/// about secrets (`password_policy`, `token_length`) are not secrets.
pub fn is_secret_path(path: &str) -> bool {
    path.split('.').any(is_secret_segment)
}

/// A value logged whole (an array, or a value that replaces one of another
/// type) with the secrets inside it blanked, as the trigger's
/// `sequent_backend.audit_redact` does.
fn redact(value: &Value) -> Value {
    match value {
        Value::Object(members) => members
            .iter()
            .map(|(key, member)| {
                let shown = if is_secret_segment(key) {
                    json!({"secret": true})
                } else {
                    redact(member)
                };
                (key.clone(), shown)
            })
            .collect::<Map<String, Value>>()
            .into(),
        Value::Array(items) => items.iter().map(redact).collect::<Vec<Value>>().into(),
        other => other.clone(),
    }
}

fn sha256_hex(text: &str) -> String {
    hex::encode(Sha256::digest(text.as_bytes()))
}

/// A value's text when it is too long to record.
fn long_text(value: &Value) -> Option<String> {
    let text = match value {
        Value::String(text) => text.clone(),
        Value::Array(_) | Value::Object(_) => value.to_string(),
        _ => return None,
    };
    (text.len() > LONG_TEXT_BYTES).then_some(text)
}

impl FieldChange {
    /// The change of `path` from `old` to `new`, either absent, recorded as
    /// its kind allows.
    pub fn new(path: &str, old: Option<&Value>, new: Option<&Value>) -> Self {
        let mut change = FieldChange {
            path: path.to_string(),
            old: None,
            new: None,
            secret: false,
            old_sha256: None,
            new_sha256: None,
        };
        if is_secret_path(path) {
            change.secret = true;
            return change;
        }
        let (old_long, new_long) = (old.and_then(long_text), new.and_then(long_text));
        if old_long.is_some() || new_long.is_some() {
            let digest = |value: Option<&Value>, long: Option<String>| {
                long.map(|text| sha256_hex(&text)).or_else(|| {
                    value.map(|value| match value {
                        Value::String(text) => sha256_hex(text),
                        other => sha256_hex(&other.to_string()),
                    })
                })
            };
            change.old_sha256 = digest(old, old_long);
            change.new_sha256 = digest(new, new_long);
            return change;
        }
        change.old = old.map(redact);
        change.new = new.map(redact);
        change
    }
}

fn present(value: Option<&Value>) -> Option<&Value> {
    value.filter(|value| !value.is_null())
}

fn collect_changes(
    path: &str,
    old: Option<&Value>,
    new: Option<&Value>,
    changes: &mut Vec<FieldChange>,
) {
    let (old, new) = (present(old), present(new));
    if old == new {
        return;
    }
    let empty = Map::new();
    let object = |value: Option<&Value>| match value {
        Some(Value::Object(map)) => Some(map.clone()),
        None => Some(empty.clone()),
        Some(_) => None,
    };
    let any_object = matches!(old, Some(Value::Object(_))) || matches!(new, Some(Value::Object(_)));
    if let (true, Some(old_map), Some(new_map)) = (any_object, object(old), object(new)) {
        let mut keys: Vec<&String> = old_map.keys().chain(new_map.keys()).collect();
        keys.sort();
        keys.dedup();
        for key in keys {
            let child = if path.is_empty() {
                key.clone()
            } else {
                format!("{path}.{key}")
            };
            collect_changes(&child, old_map.get(key), new_map.get(key), changes);
        }
        return;
    }
    changes.push(FieldChange::new(path, old, new));
}

/// The changed fields between two JSON values, objects compared key by key
/// and named by dotted paths under `prefix`. `None` and `null` are both "no
/// value".
pub fn diff_values(prefix: &str, old: Option<&Value>, new: Option<&Value>) -> Vec<FieldChange> {
    let mut changes = Vec::new();
    collect_changes(prefix, old, new, &mut changes);
    changes
}

/// A change to record as an `AdminChange` entry.
#[derive(Debug, Clone, PartialEq)]
pub struct AdminChange {
    pub target: AuditLogTarget,
    pub object: AuditObject,
    pub action: AdminAction,
    pub actor: AuditActor,
    pub origin: AuditOrigin,
    pub changes: Vec<FieldChange>,
    /// Facts that are not field changes: an import's count and file hash, a
    /// deleted event's last entry and state hash.
    pub extra: Value,
    pub election_id: Option<Uuid>,
    pub area_id: Option<Uuid>,
    /// Replaces the description built from the object, action and changes.
    pub description: Option<String>,
}

impl AdminChange {
    pub fn new(
        target: AuditLogTarget,
        object: AuditObject,
        action: AdminAction,
        actor: AuditActor,
        origin: AuditOrigin,
    ) -> Self {
        AdminChange {
            target,
            object,
            action,
            actor,
            origin,
            changes: vec![],
            extra: Value::Null,
            election_id: None,
            area_id: None,
            description: None,
        }
    }

    pub fn with_changes(mut self, changes: Vec<FieldChange>) -> Self {
        self.changes = changes;
        self
    }

    pub fn with_extra(mut self, extra: Value) -> Self {
        self.extra = extra;
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// `Updated tenant "acme": settings.logs_timezone`.
    pub fn describe(&self) -> String {
        if let Some(description) = &self.description {
            return description.clone();
        }
        let kind = self.object.kind.replace('_', " ");
        let named = match &self.object.name {
            Some(name) if !name.is_empty() => format!("{kind} \"{name}\""),
            _ => format!("{kind} {}", self.object.id),
        };
        let mut description = format!("{} {named}", self.action.past_tense());
        if self.action == AdminAction::Update && !self.changes.is_empty() {
            let paths: Vec<&str> = self
                .changes
                .iter()
                .take(DESCRIBED_PATHS)
                .map(|change| change.path.as_str())
                .collect();
            description.push_str(": ");
            description.push_str(&paths.join(", "));
            if self.changes.len() > DESCRIBED_PATHS {
                description.push_str(&format!(
                    " and {} more",
                    self.changes.len() - DESCRIBED_PATHS
                ));
            }
        }
        description
    }

    pub fn details(&self) -> Result<Value> {
        let mut details = json!({
            "object": self.object,
            "action": self.action,
            "actor": self.actor,
            "origin": self.origin,
            "changes": self.changes,
        });
        if !self.extra.is_null() {
            details["extra"] = self.extra.clone();
        }
        Ok(details)
    }

    fn entry(&self) -> Result<NewAuditLogEntry> {
        Ok(NewAuditLogEntry {
            target: self.target,
            statement_kind: AuditStatementKind::AdminChange,
            event_type: match self.actor.user_id {
                Some(_) => StatementEventType::USER,
                None => StatementEventType::SYSTEM,
            },
            log_type: StatementLogType::INFO,
            user_id: self.actor.user_id.clone(),
            username: self.actor.username.clone(),
            election_id: self.election_id,
            area_id: self.area_id,
            description: self.describe(),
            details: self.details()?,
        })
    }
}

/// Queues a change's entry in the transaction of the change.
#[instrument(skip_all, fields(object = %change.object.kind, action = %change.action), err)]
pub async fn stage(hasura_transaction: &Transaction<'_>, change: &AdminChange) -> Result<Uuid> {
    insert_audit_log_outbox(hasura_transaction, &change.entry()?)
        .await
        .context("Error queueing an administrative change")
}

/// The transaction setting that names, to `sequent_backend.audit_change()`,
/// the admin Step's own writes are made for.
pub const ACTOR_SETTING: &str = "sequent.audit_actor";

/// From now on until the transaction ends, the rows it changes in a table
/// with the `audit_change` trigger are logged as `actor`'s, made through
/// `origin`, with their old and new values. `tenant_id` is the log of a row
/// that names no tenant. A change the trigger cannot see (Keycloak, a file,
/// a table without the trigger) still needs its own entry, staged with
/// [`stage`].
#[instrument(skip_all, fields(origin = %origin), err)]
pub async fn act_for(
    hasura_transaction: &Transaction<'_>,
    actor: &AuditActor,
    origin: AuditOrigin,
    tenant_id: Uuid,
) -> Result<()> {
    let setting = json!({
        "user_id": actor.user_id,
        "username": actor.username,
        "role": actor.role,
        "origin": origin,
        "tenant_id": tenant_id,
    });
    hasura_transaction
        .execute(
            "SELECT set_config($1, $2, true)",
            &[&ACTOR_SETTING, &setting.to_string()],
        )
        .await
        .context("Error naming the admin of the transaction's changes")?;
    Ok(())
}

/// Queues an entry of any audit kind in the caller's transaction.
#[instrument(skip_all, err)]
pub async fn stage_entry(
    hasura_transaction: &Transaction<'_>,
    entry: &NewAuditLogEntry,
) -> Result<Uuid> {
    insert_audit_log_outbox(hasura_transaction, entry).await
}

#[cfg(test)]
#[path = "audit_log_tests.rs"]
mod tests;
