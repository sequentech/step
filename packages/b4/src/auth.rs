// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! B4-specific permission sets for authentication.

use sequent_core::services::axum_auth::PermissionSet;
use sequent_core::types::permissions::Permissions;

// Re-export commonly used types from sequent-core for convenience
pub use sequent_core::services::axum_auth::{JwtAuth, RequirePermissions};

/// Permission set requiring TRUSTEE_CEREMONY
///
/// Use this for B4 handlers that need trustee ceremony access.
pub struct TrusteeCeremony;

impl PermissionSet for TrusteeCeremony {
    fn required_permissions() -> &'static [Permissions] {
        &[Permissions::TRUSTEE_CEREMONY]
    }
}
