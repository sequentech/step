// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Keycloak Admin REST client and realm management helpers.

/// Admin client, token caching, and credential helpers.
mod admin_client;
/// Realm permission role CRUD and group assignment.
mod permission;
/// Realm import/export, naming, and ID replacement.
mod realm;
/// Keycloak realm attribute read/write helpers.
mod realm_attributes;
/// Keycloak role listing and assignment helpers.
mod role;
/// Keycloak user search and management helpers.
mod user;

pub use self::admin_client::*;
pub use self::permission::*;
pub use self::realm::*;
pub use self::realm_attributes::*;
pub use self::role::*;
pub use self::user::*;
