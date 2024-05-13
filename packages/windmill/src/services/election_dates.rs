// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::election::get_election_by_id;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::ballot::ElectionPresentation;
use tracing::{event, instrument, Level};

use super::date::ISO8601;

#[instrument(err)]
pub async fn manage_dates(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
    is_start: bool,
    is_unset: bool,
) -> Result<()> {
    let found_election = get_election_by_id(
        hasura_transaction,
        tenant_id,
        election_event_id,
        election_id,
    )
    .await?;
    let Some(election) = found_election else {
        return Err(anyhow!("Election not found"));
    };

    let election_presentation: ElectionPresentation = election
        .presentation
        .clone()
        .map(|presentation| serde_json::from_value(presentation))
        .transpose()
        .map_err(|err| anyhow!("Error parsing election presentation {:?}", err))?
        .unwrap_or(Default::default());
    let current_dates = election_presentation.dates.unwrap_or(Default::default());

    if is_start {
        let Some(start_date_str) = current_dates.start_date else {
            return Err(anyhow!("Empty start date"));
        };
        let start_date = ISO8601::to_date(&start_date_str)?;
        let now = ISO8601::now();
        if start_date > now {
            return Err(anyhow!("start date can't be before now"));
        }
    } else {
        let Some(end_date_str) = current_dates.end_date else {
            return Err(anyhow!("Empty end date"));
        };
    }
    Ok(())
}
