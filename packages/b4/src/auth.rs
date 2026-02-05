// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>//
// SPDX-License-Identifier: AGPL-3.0-only

//! B4-specific permission sets and constraint validation for authentication.

use crate::board_utils;
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

/// Constraint validator for board access in B4.
///
/// This validator checks that browser trustees can only access boards
/// matching their tenant_id and authorized_election_ids from JWT claims.
///
/// Native server trustees (with `x-hasura-default-role: "server"`) bypass
/// this validation and can access any board.
///
/// Uses [`board_utils::verify_board_access`] for the actual verification logic.
pub struct BoardAccessValidator;

impl ValidateConstraints for BoardAccessValidator {
    fn validate(claims: &JwtClaims, path_params: &[(&str, &str)]) -> bool {
        // Bypass for server role (native trustees)
        if claims.hasura_claims.default_role == SERVER_DEFAULT_ROLE {
            tracing::debug!("Server role detected, bypassing board access validation");
            return true;
        }

        // Find the "board" path parameter
        let board_name = path_params
            .iter()
            .find(|(k, _)| *k == "board")
            .map(|(_, v)| *v);

        match board_name {
            Some(board) => {
                // Get authorized election IDs from claims
                let Some(authorized_ids) = &claims.hasura_claims.authorized_election_ids else {
                    tracing::warn!(
                        "No authorized_election_ids in claims for user {}",
                        claims.sub
                    );
                    return false;
                };

                // Use board_utils::verify_board_access for the actual verification
                match board_utils::verify_board_access(
                    board,
                    &claims.hasura_claims.tenant_id,
                    authorized_ids,
                ) {
                    Ok(()) => true,
                    Err(e) => {
                        tracing::warn!("Board access denied for user {}: {e}", claims.sub);
                        false
                    }
                }
            }
            None => {
                // No board param (e.g., POST /boards, GET /boards)
                // These endpoints don't need board-level validation
                true
            }
        }
    }
}
