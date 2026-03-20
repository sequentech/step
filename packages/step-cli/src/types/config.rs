// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use serde::{Deserialize, Serialize};

pub use sequent_core::util::external_config::{
    DuplicateVotes, ExternalConfigData, GenerateApplications, GenerateVoters,
};

#[derive(Serialize, Deserialize)]
pub struct ConfigData {
    pub endpoint_url: String,
    pub tenant_id: String,
    pub keycloak_url: String,
    pub auth_token: String,
    pub refresh_token: String,
    pub client_id: String,
    pub client_secret: String,
    pub username: String,
}
