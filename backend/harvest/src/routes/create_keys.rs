// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use rocket::response::Debug;
use rocket::serde::json::Json;
use rocket::serde::Deserialize;

use crate::connection;

#[derive(Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
pub struct CreateKeysBody {
    tenant_id: String,
    election_event_id: String,
}

#[post("/create-keys", format = "json", data = "<body>")]
pub async fn create_keys(
    body: Json<CreateKeysBody>,
    _auth_headers: connection::AuthHeaders,
) -> Result<(), Debug<anyhow::Error>> {
    Ok(())
}
