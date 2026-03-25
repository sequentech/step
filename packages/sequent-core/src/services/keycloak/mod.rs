// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
//! Keycloak integration modules: admin client, permission, realm, role, and user management.
/// Admin client integration for Keycloak.
mod admin_client;
/// Permission management for Keycloak.
mod permission;
/// Realm management for Keycloak.
mod realm;
/// Role management for Keycloak.
mod role;
/// User management for Keycloak.
mod user;

pub use self::admin_client::*;
pub use self::permission::*;
pub use self::realm::*;
pub use self::role::*;
pub use self::user::*;
