// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use serde::{Deserialize, Serialize};

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: String,
    pub first_name: String,
    pub last_name: String,
    pub middle_name: String,
    pub country: String,
    pub id_card_number: String,
    pub id_card_type: String,
    pub date_of_birth: String,
    pub embassy: String,
}
