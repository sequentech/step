// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use serde::{Deserialize, Serialize};

pub use sequent_core::util::external_config::{
    DuplicateVotes, ExternalConfigData, GenerateApplications, GenerateVoters,
};

#[derive(Serialize, Deserialize)]
/// Config data
pub struct ConfigData {
    /// Endpoint URL (of graphql engine)
    pub endpoint_url: String,
    /// Tenant ID
    pub tenant_id: String,
    /// Keycloak URL (of keycloak server)
    pub keycloak_url: String,
    /// Auth token
    pub auth_token: String,
    /// Refresh token
    pub refresh_token: String,
    /// Client ID (of keycloak client)
    pub client_id: String,
    /// Client secret (of keycloak client)
    pub client_secret: String,
    /// Username (of keycloak user)
    pub username: String,
}
