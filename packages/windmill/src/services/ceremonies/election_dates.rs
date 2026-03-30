// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::election::get_elections;
use crate::services::celery_app::get_worker_threads;
use anyhow::{anyhow, Result};
use base64::{
    alphabet,
    engine::{self, general_purpose},
    Engine as _,
};
use chrono::{DateTime, Utc};
use csv::WriterBuilder;
use deadpool_postgres::Transaction;
use sequent_core::ballot::{ContestEncryptionPolicy, DelegatedVotingPolicy, ElectionPresentation};
use sequent_core::serialization::base64::{Base64Deserialize, Base64Serialize};
use sequent_core::serialization::deserialize_with_path::deserialize_value;
use sequent_core::services::date::ISO8601;
use sequent_core::types::hasura::core::{TallySessionContest, TallySessionContestAnnotations};
use serde_json::json;
use std::collections::HashMap;
use tracing::instrument;

#[instrument(skip_all, err)]
pub async fn get_elections_end_dates(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<HashMap<String, Option<DateTime<Utc>>>> {
    // Use ballot publications instead?
    let elections = get_elections(hasura_transaction, tenant_id, election_event_id)
        .await
        .map_err(|err| anyhow!("Error getting elections {:?}", err))?;

    let elections_dates: HashMap<String, Option<DateTime<_>>> = elections
        .into_iter()
        .map(|election| {
            let election_presentation: ElectionPresentation = election
                .presentation
                .clone()
                .map(|presentation| deserialize_value(presentation))
                .transpose()
                .map_err(|err| anyhow!("Error parsing election presentation {:?}", err))?
                .unwrap_or(Default::default());
            let current_dates = election_presentation
                .dates
                .clone()
                .unwrap_or(Default::default());
            let end_date = current_dates
                .end_date
                .clone()
                .map(|val| ISO8601::to_date_utc(&val).ok())
                .flatten();
            Ok((election.id, end_date))
        })
        .collect::<Result<HashMap<_, _>>>()
        .map_err(|err| anyhow!("Error parsing election dates {:?}", err))?;
    Ok(elections_dates)
}
