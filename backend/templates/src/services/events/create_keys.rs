// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use immu_board::{Board, BoardClient, BoardMessage};
use rocket::serde::{Deserialize, Serialize};
use std::env;

use crate::connection;

#[derive(Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
pub struct CreateKeysBody {
    pub board_name: String,
    pub trustee_pks: Vec<String>,
    pub threshold: usize,
}

pub async fn create_keys(
    auth_headers: connection::AuthHeaders,
    body: CreateKeysBody,
) -> Result<()> {
    Ok(())
}