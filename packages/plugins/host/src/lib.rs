// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
//! Authentication-independent host policy and PostgreSQL capabilities.
//!
//! The transport supplies an identity only after verifying the JWT. Plugins
//! never receive this identity or a connection: `ScopedStore` binds both tenant
//! and plugin from the verified identity and the compiled registry.

use sequent_plugin_api::{Manifest, PluginError};
use uuid::Uuid;
pub mod postgres;

/// Every plugin request comes from a tenant administrator.
pub const ADMIN_USER: &str = "admin-user";
/// Turning a plugin on or off changes tenant settings.
pub const TENANT_WRITE: &str = "tenant-write";

/// Constructed by a trusted authentication adapter, not deserialized from input.
#[derive(Clone, Debug)]
pub struct Identity {
    pub tenant_id: Uuid,
    pub user_id: String,
    pub roles: Vec<String>,
}

impl Identity {
    /// Requires an administrator who also holds `permission`.
    pub fn require(&self, permission: &str) -> Result<(), PluginError> {
        let has = |role: &str| self.roles.iter().any(|held| held == role);
        if self.user_id.is_empty() || !has(ADMIN_USER) || !has(permission) {
            return Err(PluginError::Forbidden);
        }
        Ok(())
    }
}

/// Authorization always precedes plugin execution, including read operations.
pub fn authorize(
    manifest: &Manifest,
    operation: &str,
    identity: &Identity,
) -> Result<(), PluginError> {
    manifest.check_compatible()?;
    identity.require(manifest.operation(operation)?.permission)
}
