// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::date::ISO8601;
use crate::postgres::scheduled_event::*;
use crate::tasks::manage_election_date::ManageElectionDatePayload;
use crate::types::scheduled_event::EventProcessors;
use crate::{postgres::election::*, types::scheduled_event::CronConfig};
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::ballot::{ElectionPresentation, ElectionDates};
use tracing::{event, info, instrument, Level};

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

    info!("current_dates={current_dates:?}");
    if is_unset {
        info!("is_unset is true");
        if is_start {
            new_dates.scheduled_opening = Some(false);
        } else {
            new_dates.scheduled_closing = Some(false);
        }
        if let Some(scheduled_manage_date) = scheduled_manage_date_opt {
            stop_scheduled_event(hasura_transaction, tenant_id, &scheduled_manage_date.id).await?;
        }
    } else {
        info!("is_unset is false");
        if is_start {
            new_dates.scheduled_opening = Some(true);
        } else {
            new_dates.scheduled_closing = Some(true);
        }
        let Some(manage_date_str) = (if is_start {
            current_dates.start_date
        } else {
            current_dates.end_date
        }) else {
            info!("Empty date");
            return Err(anyhow!("Empty date"));
        };
        info!("manage_date_str = {manage_date_str}");
        let manage_date_date = ISO8601::to_date(&manage_date_str)?;
        let now = ISO8601::now();
        let now_str = now.to_string();
        if manage_date_date < now {
            info!("date {manage_date_str} can't be before now {now_str}");
            return Err(anyhow!("date can't be before now"));
        }

        let cron_config = CronConfig {
            cron: None,
            scheduled_date: Some(manage_date_str),
        };
        if let Some(scheduled_manage_date) = scheduled_manage_date_opt {
            info!("update_scheduled_event");
            update_scheduled_event(
                hasura_transaction,
                tenant_id,
                &scheduled_manage_date.id,
                cron_config,
            )
            .await?;
        } else {
            info!("insert_scheduled_event");
            let event_processor = if is_start {
                EventProcessors::START_ELECTION
            } else {
                EventProcessors::END_ELECTION
            };
            let payload = ManageElectionDatePayload {
                election_id: election_id.to_string(),
            };
            insert_scheduled_event(
                hasura_transaction,
                tenant_id,
                election_event_id,
                event_processor,
                &task_id,
                cron_config,
                serde_json::to_value(payload)?,
            )
            .await?;
        }
    }

    let mut new_election_presentation: ElectionPresentation = election_presentation.clone();
    info!("update_election_presentation with new_dates={new_dates:?}");
    new_election_presentation.dates = Some(new_dates);
    update_election_presentation(
        hasura_transaction,
        tenant_id,
        election_event_id,
        election_id,
        serde_json::to_value(new_election_presentation)?,
    )
    .await?;
    Ok(())
}
