// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>//
// SPDX-License-Identifier: AGPL-3.0-only

//! B4-specific permission sets and constraint validation for authentication.

use sequent_core::services::axum_auth::{PermissionSet, ValidateConstraints};
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::Permissions;

// Re-export commonly used types from sequent-core for convenience
pub use sequent_core::services::axum_auth::{JwtAuth, RequireConstraints, RequirePermissions};

/// Default role for native server trustees.
///
/// When a JWT has `x-hasura-default-role` set to this value, board access
/// validation is bypassed, allowing the server to access any board.
pub const SERVER_DEFAULT_ROLE: &str = "server";

/// Permission set requiring TRUSTEE_CEREMONY.
///
/// Use this for B4 handlers that need trustee ceremony access.
pub struct TrusteeCeremony;

impl PermissionSet for TrusteeCeremony {
    fn required_permissions() -> &'static [Permissions] {
        &[Permissions::TRUSTEE_CEREMONY]
    }
}

/// Permission set requiring ADMIN_CEREMONY.
///
/// Use this for B4 handlers called by the admin portal.
pub struct AdminCeremony;

impl PermissionSet for AdminCeremony {
    fn required_permissions() -> &'static [Permissions] {
        &[Permissions::ADMIN_CEREMONY]
    }
}

/// Constraint validator for board access in B4.
///
/// Checks that browser trustees can only access boards listed in their
/// `authorized-boards` JWT claim. Native server trustees (default role
/// `"server"`) bypass this check and can access any board.
pub struct BoardAccessValidator;

impl ValidateConstraints for BoardAccessValidator {
    fn validate(claims: &JwtClaims, path_params: &[(&str, &str)]) -> bool {
        if claims.hasura_claims.default_role == SERVER_DEFAULT_ROLE {
            tracing::debug!("Server role detected, bypassing board access validation");
            return true;
        }

        let board_name = path_params
            .iter()
            .find(|(k, _)| *k == "board")
            .map(|(_, v)| *v);

        match board_name {
            Some(board) => {
                let Some(authorized_boards) = &claims.hasura_claims.authorized_boards else {
                    tracing::warn!(
                        "No authorized_boards in claims for user {}",
                        claims.sub
                    );
                    return false;
                };

                let authorized = authorized_boards.iter().any(|b| b == board);
                if !authorized {
                    tracing::warn!(
                        "Board access denied for user {}: board '{}' not in authorized_boards",
                        claims.sub,
                        board
                    );
                }
                authorized
            }
            None => true,
        }
    }
}
