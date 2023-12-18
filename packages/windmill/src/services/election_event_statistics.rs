// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::hasura;
use anyhow::{anyhow, Result};
use sequent_core::ballot::*;
use sequent_core::services::keycloak::get_client_credentials;
use serde_json::value::Value;
use std::default::Default;
use tracing::instrument;

pub fn get_election_event_statistics(
    statistics_json_opt: Option<Value>,
) -> Option<ElectionEventStatistics> {
    statistics_json_opt.and_then(|statistics_json| serde_json::from_value(statistics_json).ok())
}

#[instrument(err)]
pub async fn update_election_event_statistics(
    tenant_id: String,
    election_event_id: String,
    statistics: ElectionEventStatistics,
) -> Result<()> {
    let auth_headers = get_client_credentials().await?;
    let statistics_json = serde_json::to_value(&statistics)?;

    hasura::election_event::update_election_event_statistics(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
        statistics_json,
    )
    .await?;

    Ok(())
}
