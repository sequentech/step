// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use serde::{Deserialize, Serialize};

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
