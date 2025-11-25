// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct KeycloakTokenResponse {
    pub access_token: String,
    pub refresh_token: String,
}
