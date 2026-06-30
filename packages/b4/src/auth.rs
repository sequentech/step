// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! B4-specific permission sets and constraint validation for authentication.

use axum::http::StatusCode;
use sequent_core::services::axum_auth::{PermissionSet, ValidateConstraints};
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::Permissions;
use tracing::instrument;

// Re-export commonly used types from sequent-core for convenience
pub use sequent_core::services::axum_auth::{JwtAuth, RequireConstraints, RequirePermissions};

/// Default role for native server trustees.
///
/// When a JWT has `x-hasura-default-role` set to this value, board access
/// validation is bypassed, allowing the server to access any board.
pub const SERVER_DEFAULT_ROLE: &str = "server";

/// Returns whether these claims belong to a native server trustee, which
/// bypasses per-board tenant/event scoping.
fn is_server_role(claims: &JwtClaims) -> bool {
    claims.hasura_claims.default_role == SERVER_DEFAULT_ROLE
}

/// Returns whether the given claims may access `board_name`, without logging.
///
/// Server-role trustees (native) may access any board. Browser trustees may only
/// access boards listed in their `authorized-boards` JWT claim, which is
/// populated per trustee in Keycloak during the keys ceremony. Board names
/// already encode the tenant and event, so exact membership is both the tenant
/// and the event check.
///
/// Use this for filtering (e.g. listing boards) where a denial is expected and
/// should not be logged as a warning.
#[instrument(level = "trace", skip(claims), ret)]
pub fn claims_can_access_board(claims: &JwtClaims, board_name: &str) -> bool {
    is_server_role(claims)
        || claims
            .hasura_claims
            .authorized_boards
            .as_ref()
            .is_some_and(|boards| boards.iter().any(|b| b == board_name))
}

/// Authorizes an explicit access attempt to a single board for the given claims.
///
/// Returns `403 Forbidden` and logs a warning on a tenant mismatch or an invalid
/// board name. Use this in handlers whose board names come from the request body
/// (e.g. the multi-board endpoints), where the path-param-based
/// [`BoardAccessValidator`] cannot reach them.
#[instrument(level = "trace", skip(claims), err)]
pub fn authorize_board_for_claims(claims: &JwtClaims, board_name: &str) -> Result<(), StatusCode> {
    if claims_can_access_board(claims, board_name) {
        Ok(())
    } else {
        tracing::warn!(
            "Board access denied for user {}: board '{board_name}'",
            claims.sub
        );
        Err(StatusCode::FORBIDDEN)
    }
}

/// Permission set requiring TRUSTEE_CEREMONY.
///
/// Use this for B4 handlers that need trustee ceremony access.
pub struct TrusteeCeremony;

impl PermissionSet for TrusteeCeremony {
    fn required_permissions() -> &'static [Permissions] {
        &[Permissions::TRUSTEE_CEREMONY]
    }
}

/// Constraint validator for board access in B4, for handlers that take the
/// board name as a path parameter.
///
/// Restricts browser trustees to boards listed in their `authorized-boards`
/// claim; native server trustees (`x-hasura-default-role: "server"`) bypass the
/// check. Delegates to [`authorize_board_for_claims`].
///
/// Handlers whose board names come from the request body (the multi-board
/// endpoints) must call [`authorize_board_for_claims`] directly, since there is
/// no `board` path parameter for this validator to read.
pub struct BoardAccessValidator;

impl ValidateConstraints for BoardAccessValidator {
    #[instrument(skip(claims), ret)]
    fn validate(claims: &JwtClaims, path_params: &[(&str, &str)]) -> bool {
        // Find the "board" path parameter.
        let board_name = path_params
            .iter()
            .find(|(k, _)| *k == "board")
            .map(|(_, v)| *v);

        match board_name {
            Some(board) => authorize_board_for_claims(claims, board).is_ok(),
            // No board param (e.g. POST /boards, GET /boards): the permission
            // check already applied by the extractor is sufficient.
            None => true,
        }
    }
}
