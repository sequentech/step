// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::scheduled_event::*;
use crate::{postgres::election::get_election_by_id, types::scheduled_event::CronConfig};
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::ballot::ElectionPresentation;
use tracing::{event, instrument, Level};

use super::date::ISO8601;

#[instrument]
pub fn generate_manage_date_election_task_name(
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
    is_start: bool,
) -> String {
    format!(
        "tenant_{}_event_{}_election_{}_{}",
        tenant_id,
        election_event_id,
        election_id,
        if is_start { "start" } else { "end" },
    )
}

#[instrument(skip(hasura_transaction), err)]
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
    let current_dates = election_presentation
        .dates
        .clone()
        .unwrap_or(Default::default());
    let mut new_dates = current_dates.clone();

    let task_id = generate_manage_date_election_task_name(
        tenant_id,
        election_event_id,
        election_id,
        is_start,
    );
    let scheduled_manage_date_opt =
        find_scheduled_event_by_task_id(hasura_transaction, tenant_id, election_event_id, &task_id)
            .await?;

    if is_unset {
        if is_start {
            new_dates.scheduled_opening = Some(false);
        } else {
            new_dates.scheduled_closing = Some(false);
        }
        if let Some(scheduled_manage_date) = scheduled_manage_date_opt {
            stop_scheduled_event(hasura_transaction, tenant_id, &scheduled_manage_date.id).await?;
        }
    } else {
        let Some(manage_date_str) = (if is_start {
            current_dates.start_date
        } else {
            current_dates.end_date
        }) else {
            return Err(anyhow!("Empty date"));
        };
        let manage_date_date = ISO8601::to_date(&manage_date_str)?;
        let now = ISO8601::now();
        if manage_date_date > now {
            return Err(anyhow!("date can't be before now"));
        }

        let cron_config = CronConfig {
            cron: None,
            scheduled_date: Some(manage_date_str),
        };
        if let Some(scheduled_manage_date) = scheduled_manage_date_opt {
            update_scheduled_event(
                hasura_transaction,
                tenant_id,
                &scheduled_manage_date.id,
                cron_config,
            )
            .await?;
        } else {
            insert_scheduled_event(
                hasura_transaction,
                tenant_id,
                &task_id,
                cron_config,
            )
            .await?;
        }
    }

    let mut new_election_presentation: ElectionPresentation = election_presentation.clone();
    new_election_presentation.dates = Some(new_dates);
    /*update_election_presentation(
        hasura_transaction,
        tenant_id,
        election_event_id,
        election_id,
        new_election_presentation
    ).await?;*/
    Ok(())
}
