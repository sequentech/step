// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
/// Keycloak token response
pub struct KeycloakTokenResponse {
    /// Access token
    pub access_token: String,
    /// Refresh token
    pub refresh_token: String,
}
