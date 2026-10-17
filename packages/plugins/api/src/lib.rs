// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Versioned, tenant-scoped ports for bundled first-party plugins.
//!
//! Plugins receive capabilities for their own records, never a database
//! connection, a tenant selector, or an arbitrary SQL executor. The host owns
//! authentication, operation permissions, transactions, and namespace binding.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// The first published API major. There is no previous major yet.
pub const API_MAJOR: u16 = 1;
/// Bound both HTTP payloads and CSV imports independently of the adapter.
pub const MAX_REQUEST_BYTES: usize = 1_048_576;
/// Maximum size of one JSON record.
pub const MAX_RECORD_BYTES: usize = 65_536;
/// Bound list operations at the host boundary.
pub const MAX_PAGE_SIZE: u32 = 1000;
/// Maximum length of an audit action name.
pub const MAX_ACTION_BYTES: usize = 100;

/// A declared operation and its required realm permission.
#[derive(Clone, Debug, Serialize)]
pub struct Operation {
    pub name: &'static str,
    pub permission: &'static str,
}

/// Code-bundled metadata. Remote manifests cannot install executable code.
#[derive(Clone, Debug, Serialize)]
pub struct Manifest {
    pub id: &'static str,
    pub version: &'static str,
    pub api_major: u16,
    pub data_version: i32,
    pub label_key: &'static str,
    pub route: &'static str,
    pub read_permission: &'static str,
    pub permissions: &'static [&'static str],
    pub record_kinds: &'static [&'static str],
    pub operations: &'static [Operation],
}

impl Manifest {
    /// Refuse an unsupported ABI before registering or enabling a plugin.
    pub fn check_compatible(&self) -> Result<(), PluginError> {
        if self.api_major != API_MAJOR || self.data_version < 1 {
            return Err(PluginError::Incompatible);
        }
        Ok(())
    }

    /// Look up permissions from trusted code, never from request data.
    pub fn operation(&self, name: &str) -> Result<&Operation, PluginError> {
        self.operations
            .iter()
            .find(|operation| operation.name == name)
            .ok_or(PluginError::NotFound)
    }
}

/// Optimistic concurrency applies to the whole stored record.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Record {
    pub id: Uuid,
    pub kind: String,
    pub parent_id: Option<Uuid>,
    pub data: Value,
    pub version: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub updated_by: String,
}

/// Append-only history, stored in the same transaction as a record change.
/// `parent_id` is the changed record's parent when the event was written, so a
/// parent's history keeps its children's changes after they are deleted.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AuditEvent {
    pub id: i64,
    pub record_id: Option<Uuid>,
    pub parent_id: Option<Uuid>,
    pub action: String,
    pub actor: String,
    pub at: DateTime<Utc>,
    pub before: Option<Value>,
    pub after: Option<Value>,
}

/// Which events a history page contains, newest first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HistoryScope {
    /// Events about the record itself.
    Record(Uuid),
    /// Events about a record that was a child of `parent` when written,
    /// including after the child was deleted.
    Child { parent: Uuid, id: Uuid },
    /// Events about the record and every record that was its child.
    RecordAndChildren(Uuid),
}

/// Stable errors; transport adapters choose status codes without string parsing.
#[derive(Clone, Debug, thiserror::Error)]
pub enum PluginError {
    #[error("Permission denied")]
    Forbidden,
    #[error("Plugin is not enabled for this tenant")]
    Disabled,
    #[error("Plugin API or stored data version is incompatible")]
    Incompatible,
    #[error("Record or operation not found")]
    NotFound,
    #[error("This record changed. Load the current version before saving")]
    Conflict(Box<Record>),
    #[error("Plugin settings changed. Reload before trying again")]
    StateConflict,
    #[error("{0}")]
    InvalidInput(String),
    #[error("Plugin storage is unavailable")]
    Storage(String),
}

/// A namespace-bound record store. A host must reject undeclared kinds.
/// Every mutation automatically records its before/after values in the audit
/// port's transaction. A failed operation rolls back all of its mutations.
#[async_trait]
pub trait RecordStore: Send + Sync {
    async fn get(&self, kind: &str, id: Uuid) -> Result<Record, PluginError>;
    async fn list(
        &self,
        kind: &str,
        parent: Option<Uuid>,
        offset: u32,
        limit: u32,
    ) -> Result<Vec<Record>, PluginError>;
    async fn create(
        &self,
        kind: &str,
        id: Uuid,
        parent: Option<Uuid>,
        data: Value,
    ) -> Result<Record, PluginError>;
    async fn replace(
        &self,
        kind: &str,
        id: Uuid,
        version: i32,
        data: Value,
    ) -> Result<Record, PluginError>;
    async fn delete(&self, kind: &str, id: Uuid, version: i32) -> Result<(), PluginError>;
}

/// Extra events (such as export) share the record transaction and trusted actor.
#[async_trait]
pub trait Audit: Send + Sync {
    async fn record(
        &self,
        action: &str,
        record_id: Option<Uuid>,
        before: Option<Value>,
        after: Option<Value>,
    ) -> Result<(), PluginError>;
    async fn history(
        &self,
        scope: HistoryScope,
        before_id: Option<i64>,
        limit: u32,
    ) -> Result<Vec<AuditEvent>, PluginError>;
}

/// Time and IDs come from the host, replaceable in tests.
pub trait Clock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
    fn new_id(&self) -> Uuid;
}

/// Capabilities supplied for one authenticated operation and transaction.
pub trait HostPorts: RecordStore + Audit + Clock {}
impl<T: RecordStore + Audit + Clock> HostPorts for T {}

/// A compiled-in implementation. WASM adapters must implement these same
/// capabilities before they can be registered here; the legacy SQL host is not
/// an implementation of this API.
#[async_trait]
pub trait Plugin: Send + Sync {
    fn manifest(&self) -> &'static Manifest;
    async fn execute(
        &self,
        operation: &str,
        input: Value,
        host: &dyn HostPorts,
    ) -> Result<Value, PluginError>;
}
