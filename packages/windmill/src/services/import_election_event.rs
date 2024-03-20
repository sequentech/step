// SPDX-FileCopyrightText: 2024 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::services::keycloak;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use tracing::instrument;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct ImportElectionEventSchema {
    pub name: String,
}

pub async fn process(data: &ImportElectionEventSchema) {
    dbg!(&data);
}
