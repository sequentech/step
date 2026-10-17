// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
//! Transactional host adapter. Every statement binds the authenticated tenant
//! and registered plugin. None of these selectors are supplied by plugin code.

use crate::{authorize, Identity, ADMIN_USER, TENANT_WRITE};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sequent_plugin_api::{
    Audit, AuditEvent, Clock, HistoryScope, Manifest, Plugin, PluginError, Record, RecordStore,
    MAX_ACTION_BYTES, MAX_PAGE_SIZE, MAX_RECORD_BYTES, MAX_REQUEST_BYTES,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashSet;
use tokio_postgres::{Row, Transaction};
use uuid::Uuid;

const ACTION_CREATE: &str = "create";
const ACTION_UPDATE: &str = "update";
const ACTION_DELETE: &str = "delete";

fn storage(error: impl ToString) -> PluginError {
    PluginError::Storage(error.to_string())
}

fn invalid(message: &str) -> PluginError {
    PluginError::InvalidInput(message.into())
}

fn page(limit: u32) -> Result<i64, PluginError> {
    if limit == 0 || limit > MAX_PAGE_SIZE {
        return Err(invalid("Page size must be between 1 and 1000"));
    }
    Ok(i64::from(limit))
}

/// Whether a tenant has turned a plugin on. Stored as its snake_case name.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PluginState {
    Enabled,
    Disabled,
}

impl PluginState {
    fn as_str(self) -> &'static str {
        match self {
            Self::Enabled => "enabled",
            Self::Disabled => "disabled",
        }
    }

    fn from_stored(value: &str) -> Self {
        if value == Self::Enabled.as_str() {
            Self::Enabled
        } else {
            Self::Disabled
        }
    }
}

/// Whether the tenant's stored data matches the bundled plugin's data version.
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Compatibility {
    Compatible,
    Incompatible,
}

#[derive(Serialize)]
pub struct TenantPlugin {
    pub manifest: &'static Manifest,
    pub state: PluginState,
    pub version: i32,
    pub compatibility: Compatibility,
}

/// Idempotent provisioning; failure leaves the plugin disabled. A retry may
/// complete roles already created in Keycloak, but never removes existing roles.
#[async_trait]
pub trait RoleProvisioner: Send + Sync {
    async fn ensure_admin_roles(
        &self,
        tenant: Uuid,
        permissions: &[&str],
    ) -> Result<(), PluginError>;
}

fn valid_manifest(manifest: &Manifest) -> bool {
    let mut names = HashSet::new();
    !manifest.id.is_empty()
        && manifest
            .id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        && manifest.permissions.contains(&manifest.read_permission)
        && manifest
            .operations
            .iter()
            .all(|op| manifest.permissions.contains(&op.permission))
        && manifest.operations.iter().all(|op| names.insert(op.name))
}

/// Only code-bundled plugins can enter this registry.
pub struct Registry {
    plugins: Vec<Box<dyn Plugin>>,
}

impl Registry {
    pub fn new(plugins: Vec<Box<dyn Plugin>>) -> Result<Self, PluginError> {
        let mut ids = HashSet::new();
        for plugin in &plugins {
            let manifest = plugin.manifest();
            manifest.check_compatible()?;
            if !ids.insert(manifest.id) || !valid_manifest(manifest) {
                return Err(PluginError::Incompatible);
            }
        }
        Ok(Self { plugins })
    }

    fn plugin(&self, id: &str) -> Result<&dyn Plugin, PluginError> {
        self.plugins
            .iter()
            .find(|plugin| plugin.manifest().id == id)
            .map(|plugin| plugin.as_ref())
            .ok_or(PluginError::NotFound)
    }

    pub async fn list(
        &self,
        tx: &Transaction<'_>,
        identity: &Identity,
    ) -> Result<Vec<TenantPlugin>, PluginError> {
        identity.require(ADMIN_USER)?;
        let rows = tx
            .query(
                "SELECT plugin_id, state, version, data_version \
                 FROM sequent_backend.plugin_tenant WHERE tenant_id=$1",
                &[&identity.tenant_id],
            )
            .await
            .map_err(storage)?;
        Ok(self
            .plugins
            .iter()
            .map(|plugin| {
                let manifest = plugin.manifest();
                let row = rows
                    .iter()
                    .find(|row| row.get::<_, String>("plugin_id") == manifest.id);
                let state = row.map_or(PluginState::Disabled, |row| {
                    PluginState::from_stored(&row.get::<_, String>("state"))
                });
                let compatible = row
                    .is_none_or(|row| row.get::<_, i32>("data_version") == manifest.data_version);
                TenantPlugin {
                    manifest,
                    state,
                    version: row.map_or(0, |row| row.get("version")),
                    compatibility: if compatible {
                        Compatibility::Compatible
                    } else {
                        Compatibility::Incompatible
                    },
                }
            })
            .collect())
    }

    /// Caller commits only on success. The row lock also serializes execution,
    /// so disabling cannot race a mutation that has already passed its check.
    pub async fn set_state(
        &self,
        tx: &Transaction<'_>,
        identity: &Identity,
        id: &str,
        state: PluginState,
        expected_version: i32,
        roles: &dyn RoleProvisioner,
    ) -> Result<(), PluginError> {
        identity.require(TENANT_WRITE)?;
        let manifest = self.plugin(id)?.manifest();
        manifest.check_compatible()?;
        tx.execute(
            "INSERT INTO sequent_backend.plugin_tenant \
             (tenant_id, plugin_id, data_version, updated_by) \
             VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING",
            &[
                &identity.tenant_id,
                &id,
                &manifest.data_version,
                &identity.user_id,
            ],
        )
        .await
        .map_err(storage)?;
        let row = tx
            .query_one(
                "SELECT state, version, data_version \
                 FROM sequent_backend.plugin_tenant \
                 WHERE tenant_id=$1 AND plugin_id=$2 FOR UPDATE",
                &[&identity.tenant_id, &id],
            )
            .await
            .map_err(storage)?;
        if row.get::<_, i32>("version") != expected_version {
            return Err(PluginError::StateConflict);
        }
        if state == PluginState::Enabled {
            if row.get::<_, i32>("data_version") != manifest.data_version {
                return Err(PluginError::Incompatible);
            }
            roles
                .ensure_admin_roles(identity.tenant_id, manifest.permissions)
                .await?;
        }
        tx.execute(
            "UPDATE sequent_backend.plugin_tenant \
             SET state=$3, version=version+1, updated_at=now(), updated_by=$4 \
             WHERE tenant_id=$1 AND plugin_id=$2",
            &[&identity.tenant_id, &id, &state.as_str(), &identity.user_id],
        )
        .await
        .map_err(storage)?;
        let previous = row.get::<_, String>("state");
        ScopedStore {
            tx,
            identity,
            manifest,
        }
        .event(
            state.as_str(),
            None,
            None,
            Some(json!({ "state": previous })),
            Some(json!({ "state": state })),
        )
        .await
    }

    /// Checks that `identity` may run `operation` of plugin `id`. Transports
    /// call it before contacting any backend; `execute` checks it again.
    pub fn authorize(
        &self,
        identity: &Identity,
        id: &str,
        operation: &str,
    ) -> Result<(), PluginError> {
        authorize(self.plugin(id)?.manifest(), operation, identity)
    }

    /// Executes one operation in the caller's transaction. Any error must cause
    /// the caller to drop/roll back the transaction, including validation errors.
    pub async fn execute(
        &self,
        tx: &Transaction<'_>,
        identity: &Identity,
        id: &str,
        operation: &str,
        input: Value,
    ) -> Result<Value, PluginError> {
        let plugin = self.plugin(id)?;
        let manifest = plugin.manifest();
        authorize(manifest, operation, identity)?;
        if serde_json::to_vec(&input).map_err(storage)?.len() > MAX_REQUEST_BYTES {
            return Err(invalid("Plugin request exceeds 1 MiB"));
        }
        let row = tx
            .query_opt(
                "SELECT state, data_version FROM sequent_backend.plugin_tenant \
                 WHERE tenant_id=$1 AND plugin_id=$2 FOR SHARE",
                &[&identity.tenant_id, &id],
            )
            .await
            .map_err(storage)?
            .ok_or(PluginError::Disabled)?;
        if PluginState::from_stored(&row.get::<_, String>("state")) != PluginState::Enabled {
            return Err(PluginError::Disabled);
        }
        if row.get::<_, i32>("data_version") != manifest.data_version {
            return Err(PluginError::Incompatible);
        }
        plugin
            .execute(
                operation,
                input,
                &ScopedStore {
                    tx,
                    identity,
                    manifest,
                },
            )
            .await
    }
}

/// Private fields prevent a plugin from choosing a different tenant/plugin.
struct ScopedStore<'a, 'tx> {
    tx: &'a Transaction<'tx>,
    identity: &'a Identity,
    manifest: &'static Manifest,
}

fn record(row: Row) -> Record {
    Record {
        id: row.get("id"),
        kind: row.get("kind"),
        parent_id: row.get("parent_id"),
        data: row.get("data"),
        version: row.get("version"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
        updated_by: row.get("updated_by"),
    }
}

fn audit_event(row: Row) -> AuditEvent {
    AuditEvent {
        id: row.get("id"),
        record_id: row.get("record_id"),
        parent_id: row.get("parent_id"),
        action: row.get("action"),
        actor: row.get("actor"),
        at: row.get("at"),
        before: row.get("before_data"),
        after: row.get("after_data"),
    }
}

impl ScopedStore<'_, '_> {
    fn kind(&self, kind: &str) -> Result<(), PluginError> {
        if self.manifest.record_kinds.contains(&kind) {
            Ok(())
        } else {
            Err(PluginError::Forbidden)
        }
    }

    fn data(&self, value: &Value) -> Result<(), PluginError> {
        if !value.is_object()
            || serde_json::to_vec(value).map_err(storage)?.len() > MAX_RECORD_BYTES
        {
            return Err(invalid("Record must be a JSON object of at most 64 KiB"));
        }
        Ok(())
    }

    async fn event(
        &self,
        action: &str,
        record_id: Option<Uuid>,
        parent_id: Option<Uuid>,
        before: Option<Value>,
        after: Option<Value>,
    ) -> Result<(), PluginError> {
        if action.is_empty() || action.len() > MAX_ACTION_BYTES {
            return Err(invalid("Audit action must be 1 to 100 bytes"));
        }
        self.tx
            .execute(
                "INSERT INTO sequent_backend.plugin_record_event \
                 (tenant_id, plugin_id, record_id, parent_id, action, actor, \
                  before_data, after_data) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
                &[
                    &self.identity.tenant_id,
                    &self.manifest.id,
                    &record_id,
                    &parent_id,
                    &action,
                    &self.identity.user_id,
                    &before,
                    &after,
                ],
            )
            .await
            .map_err(storage)?;
        Ok(())
    }
}

#[async_trait]
impl RecordStore for ScopedStore<'_, '_> {
    async fn get(&self, kind: &str, id: Uuid) -> Result<Record, PluginError> {
        self.kind(kind)?;
        self.tx
            .query_opt(
                "SELECT * FROM sequent_backend.plugin_record \
                 WHERE tenant_id=$1 AND plugin_id=$2 AND kind=$3 AND id=$4",
                &[&self.identity.tenant_id, &self.manifest.id, &kind, &id],
            )
            .await
            .map_err(storage)?
            .map(record)
            .ok_or(PluginError::NotFound)
    }

    async fn list(
        &self,
        kind: &str,
        parent: Option<Uuid>,
        offset: u32,
        limit: u32,
    ) -> Result<Vec<Record>, PluginError> {
        self.kind(kind)?;
        let limit = page(limit)?;
        let rows = self
            .tx
            .query(
                "SELECT * FROM sequent_backend.plugin_record \
                 WHERE tenant_id=$1 AND plugin_id=$2 AND kind=$3 \
                 AND parent_id IS NOT DISTINCT FROM $4 \
                 ORDER BY created_at, id OFFSET $5 LIMIT $6",
                &[
                    &self.identity.tenant_id,
                    &self.manifest.id,
                    &kind,
                    &parent,
                    &i64::from(offset),
                    &limit,
                ],
            )
            .await
            .map_err(storage)?;
        Ok(rows.into_iter().map(record).collect())
    }

    async fn create(
        &self,
        kind: &str,
        id: Uuid,
        parent: Option<Uuid>,
        data: Value,
    ) -> Result<Record, PluginError> {
        self.kind(kind)?;
        self.data(&data)?;
        if let Some(parent_id) = parent {
            let exists = self
                .tx
                .query_opt(
                    "SELECT id FROM sequent_backend.plugin_record \
                     WHERE tenant_id=$1 AND plugin_id=$2 AND id=$3",
                    &[&self.identity.tenant_id, &self.manifest.id, &parent_id],
                )
                .await
                .map_err(storage)?;
            if exists.is_none() {
                return Err(PluginError::NotFound);
            }
        }
        let saved = record(
            self.tx
                .query_one(
                    "INSERT INTO sequent_backend.plugin_record \
                     (tenant_id, plugin_id, id, kind, parent_id, data, updated_by) \
                     VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING *",
                    &[
                        &self.identity.tenant_id,
                        &self.manifest.id,
                        &id,
                        &kind,
                        &parent,
                        &data,
                        &self.identity.user_id,
                    ],
                )
                .await
                .map_err(storage)?,
        );
        self.event(
            ACTION_CREATE,
            Some(id),
            parent,
            None,
            Some(json!({"kind": kind, "data": data, "version": saved.version})),
        )
        .await?;
        Ok(saved)
    }

    async fn replace(
        &self,
        kind: &str,
        id: Uuid,
        version: i32,
        data: Value,
    ) -> Result<Record, PluginError> {
        self.kind(kind)?;
        self.data(&data)?;
        let previous = self.get(kind, id).await?;
        if previous.version != version {
            return Err(PluginError::Conflict(Box::new(previous)));
        }
        let saved = self
            .tx
            .query_opt(
                "UPDATE sequent_backend.plugin_record \
                 SET data=$5, version=version+1, updated_at=now(), updated_by=$6 \
                 WHERE tenant_id=$1 AND plugin_id=$2 AND kind=$3 AND id=$4 \
                 AND version=$7 RETURNING *",
                &[
                    &self.identity.tenant_id,
                    &self.manifest.id,
                    &kind,
                    &id,
                    &data,
                    &self.identity.user_id,
                    &version,
                ],
            )
            .await
            .map_err(storage)?
            .map(record)
            .ok_or_else(|| PluginError::Conflict(Box::new(previous.clone())))?;
        self.event(
            ACTION_UPDATE,
            Some(id),
            previous.parent_id,
            Some(json!({"data": previous.data, "version": previous.version})),
            Some(json!({"data": data, "version": saved.version})),
        )
        .await?;
        Ok(saved)
    }

    async fn delete(&self, kind: &str, id: Uuid, version: i32) -> Result<(), PluginError> {
        self.kind(kind)?;
        let previous = self.get(kind, id).await?;
        if previous.version != version {
            return Err(PluginError::Conflict(Box::new(previous)));
        }
        let deleted = self
            .tx
            .execute(
                "DELETE FROM sequent_backend.plugin_record \
                 WHERE tenant_id=$1 AND plugin_id=$2 AND kind=$3 AND id=$4 \
                 AND version=$5",
                &[
                    &self.identity.tenant_id,
                    &self.manifest.id,
                    &kind,
                    &id,
                    &version,
                ],
            )
            .await
            .map_err(storage)?;
        if deleted != 1 {
            return Err(PluginError::Conflict(Box::new(previous)));
        }
        self.event(
            ACTION_DELETE,
            Some(id),
            previous.parent_id,
            Some(json!({"data": previous.data, "version": previous.version})),
            None,
        )
        .await
    }
}

#[async_trait]
impl Audit for ScopedStore<'_, '_> {
    async fn record(
        &self,
        action: &str,
        record_id: Option<Uuid>,
        before: Option<Value>,
        after: Option<Value>,
    ) -> Result<(), PluginError> {
        let parent_id = match record_id {
            Some(id) => self
                .tx
                .query_opt(
                    "SELECT parent_id FROM sequent_backend.plugin_record \
                     WHERE tenant_id=$1 AND plugin_id=$2 AND id=$3",
                    &[&self.identity.tenant_id, &self.manifest.id, &id],
                )
                .await
                .map_err(storage)?
                .and_then(|row| row.get("parent_id")),
            None => None,
        };
        self.event(action, record_id, parent_id, before, after)
            .await
    }

    async fn history(
        &self,
        scope: HistoryScope,
        before_id: Option<i64>,
        limit: u32,
    ) -> Result<Vec<AuditEvent>, PluginError> {
        let limit = page(limit)?;
        let (record_id, parent_id, condition) = match scope {
            HistoryScope::Record(id) => (Some(id), None, "record_id=$3 AND $4::uuid IS NULL"),
            HistoryScope::Child { parent, id } => {
                (Some(id), Some(parent), "record_id=$3 AND parent_id=$4")
            }
            HistoryScope::RecordAndChildren(id) => {
                (Some(id), Some(id), "(record_id=$3 OR parent_id=$4)")
            }
        };
        let query = format!(
            "SELECT * FROM sequent_backend.plugin_record_event \
             WHERE tenant_id=$1 AND plugin_id=$2 AND {condition} \
             AND ($5::bigint IS NULL OR id<$5) ORDER BY id DESC LIMIT $6"
        );
        let rows = self
            .tx
            .query(
                &query,
                &[
                    &self.identity.tenant_id,
                    &self.manifest.id,
                    &record_id,
                    &parent_id,
                    &before_id,
                    &limit,
                ],
            )
            .await
            .map_err(storage)?;
        Ok(rows.into_iter().map(audit_event).collect())
    }
}

impl Clock for ScopedStore<'_, '_> {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }

    fn new_id(&self) -> Uuid {
        Uuid::new_v4()
    }
}
