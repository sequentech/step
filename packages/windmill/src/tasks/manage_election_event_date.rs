// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::election::{
    get_elections, update_election_presentation, update_election_voting_status,
};
use crate::postgres::election_event::get_election_event_by_id;
use crate::postgres::scheduled_event::*;
use crate::services::database::get_hasura_pool;
use crate::services::election_event_status::update_scheduled_event_voting_status;
use crate::services::initialization_schedule::{
    scheduled_post_opening, ScheduledPostOpening, PENDING_ANNOTATION,
};
use crate::services::pg_lock::PgLock;
use crate::services::scheduled_outcome::{fire_time_state, Moment};
use crate::services::signing::actions::voting::{event_wide_decision, scheduled_decisions};
use crate::tasks::scheduled_events::{elections_with_own_row, fires_later};
use crate::types::error::{Error, Result};
use anyhow::{anyhow, Result as AnyhowResult};
use celery::error::TaskError;
use chrono::{Duration, Utc};
use deadpool_postgres::Client as DbClient;
use deadpool_postgres::Transaction;
use sequent_core::ballot::{
    AllowTallyStatus, ElectionPresentation, ElectionStatus, InitReport, VotingPeriodEnd,
    VotingStatus, VotingStatusChannel,
};
use sequent_core::serialization::deserialize_with_path::deserialize_value;
use sequent_core::services::date::ISO8601;
use sequent_core::types::hasura::core::Election;
use sequent_core::types::scheduled_event::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tracing::instrument;
use tracing::{event, info, Level};
use uuid::Uuid;

#[instrument(err)]
pub async fn manage_election_event_date_wrapped(
    hasura_transaction: &Transaction<'_>,
    tenant_id: String,
    election_event_id: String,
    scheduled_event_id: String,
) -> AnyhowResult<()> {
    lock_scheduled_event(
        hasura_transaction,
        &tenant_id,
        &election_event_id,
        &scheduled_event_id,
    )
    .await?;
    if get_election_event_by_id(hasura_transaction, &tenant_id, &election_event_id)
        .await?
        .is_archived
    {
        info!("Skipping scheduled transition {scheduled_event_id}: the event is archived");
        return Ok(());
    }
    let scheduled_manage_date_opt = find_scheduled_event_by_id(
        hasura_transaction,
        Some(tenant_id.clone()),
        Some(election_event_id.clone()),
        &scheduled_event_id,
    )
    .await?;
    let Some(scheduled_manage_date) = scheduled_manage_date_opt else {
        return Err(anyhow!(
            "Can't find scheduled event with id: {scheduled_event_id}"
        ));
    };

    let Some(event_processor) = scheduled_manage_date.event_processor.clone() else {
        return Err(anyhow!("Missing event processor"));
    };
    if matches!(
        event_processor,
        EventProcessors::START_VOTING_PERIOD | EventProcessors::END_VOTING_PERIOD
    ) && scheduled_manage_date
        .event_payload
        .as_ref()
        .and_then(|payload| payload.get("election_id"))
        .is_some_and(|target| !target.is_null())
    {
        return Err(anyhow!(
            "The queued scheduled transition is no longer event-wide"
        ));
    }
    if fires_later(&scheduled_manage_date, Utc::now()) {
        info!("Scheduled event {scheduled_event_id} was moved to a later time; it runs then");
        return Ok(());
    }
    // Every election's status is read and written below.
    lock_elections(hasura_transaction, &tenant_id, &election_event_id, None).await?;
    // A Post's own row for the same change decides for it.
    let own_rows = elections_with_own_row(
        &find_scheduled_event_by_election_event_id(
            hasura_transaction,
            &tenant_id,
            &election_event_id,
        )
        .await?,
        &tenant_id,
        &election_event_id,
        &event_processor,
    );

    let voting_status = match event_processor {
        EventProcessors::START_VOTING_PERIOD => VotingStatus::OPEN,
        EventProcessors::END_VOTING_PERIOD => VotingStatus::CLOSED,
        EventProcessors::ALLOW_INIT_REPORT
        | EventProcessors::ALLOW_VOTING_PERIOD_END
        | EventProcessors::ALLOW_TALLY => {
            apply_to_every_election(
                hasura_transaction,
                &tenant_id,
                &election_event_id,
                &scheduled_manage_date,
                &event_processor,
                &own_rows,
            )
            .await?;
            stop_scheduled_event(&hasura_transaction, &tenant_id, &scheduled_manage_date.id)
                .await?;
            return Ok(());
        }
        _ => {
            info!("Invalid scheduled event type: {:?}", event_processor);
            stop_scheduled_event(&hasura_transaction, &tenant_id, &scheduled_manage_date.id)
                .await?;
            return Ok(());
        }
    };
    let payload: ManageElectionDatePayload = serde_json::from_value(
        scheduled_manage_date
            .event_payload
            .clone()
            .unwrap_or_else(|| serde_json::json!({})),
    )?;
    // Each Post this event-wide row changes is decided (its signed
    // configuration and the lifecycle policies, VOTE-LIFECYCLE §5a) and
    // logged; only the Posts whose decision runs change. The others are
    // skipped like Posts with a row of their own.
    let mut waiting = Vec::new();
    let decisions = if voting_status == VotingStatus::OPEN {
        let pending: Option<HashSet<String>> = scheduled_manage_date
            .annotations
            .as_ref()
            .and_then(|annotations| annotations.get(PENDING_ANNOTATION))
            .map(|value| serde_json::from_value(value.clone()))
            .transpose()?;
        let (state, row) = fire_time_state(
            hasura_transaction,
            Uuid::parse_str(&tenant_id)?,
            Uuid::parse_str(&election_event_id)?,
            &scheduled_manage_date.id,
        )
        .await?
        .ok_or_else(|| anyhow!("The scheduled opening cannot be evaluated"))?;
        let targets: HashSet<String> = state
            .posts_of(&row)
            .into_iter()
            .map(|id| id.to_string())
            .collect();
        let mut ready_or_refused = Vec::new();
        for election in get_elections(hasura_transaction, &tenant_id, &election_event_id).await? {
            if !targets.contains(&election.id)
                || pending
                    .as_ref()
                    .is_some_and(|pending| !pending.contains(&election.id))
            {
                continue;
            }
            let allowed = state
                .explain(&row, Some(Uuid::parse_str(&election.id)?), Moment::FireTime)
                .outcome
                != sequent_core::types::scheduled_outcome::ScheduledOutcomeKind::Refused;
            if allowed {
                match scheduled_post_opening(
                    hasura_transaction,
                    &tenant_id,
                    &election_event_id,
                    &election,
                    &scheduled_manage_date,
                )
                .await?
                {
                    ScheduledPostOpening::Wait => {
                        waiting.push(election.id);
                        continue;
                    }
                    ScheduledPostOpening::AfterClose => continue,
                    ScheduledPostOpening::Open => {}
                }
            }
            ready_or_refused.push(election.id);
        }
        scheduled_decisions(
            hasura_transaction,
            &tenant_id,
            &election_event_id,
            &ready_or_refused,
            &voting_status,
            &scheduled_manage_date.id,
        )
        .await?
    } else {
        event_wide_decision(
            hasura_transaction,
            &tenant_id,
            &election_event_id,
            &scheduled_manage_date,
        )
        .await?
    };
    let runs: HashSet<String> = decisions
        .into_iter()
        .filter(|decision| decision.runs)
        .map(|decision| decision.election_id)
        .collect();
    let skipped: HashSet<String> =
        get_elections(hasura_transaction, &tenant_id, &election_event_id)
            .await?
            .into_iter()
            .map(|election| election.id)
            .filter(|election_id| !runs.contains(election_id))
            .collect();
    if !runs.is_empty() {
        update_scheduled_event_voting_status(
            &hasura_transaction,
            &tenant_id,
            None,
            None,
            &election_event_id,
            &voting_status,
            &Some(payload.channels()),
            &skipped,
        )
        .await?;
    }

    if voting_status == VotingStatus::OPEN {
        crate::postgres::election_initialization::set_scheduled_event_annotation(
            hasura_transaction,
            Uuid::parse_str(&scheduled_manage_date.id)?,
            PENDING_ANNOTATION,
            &serde_json::json!(waiting),
        )
        .await?;
        if !waiting.is_empty() {
            return Ok(());
        }
    }
    stop_scheduled_event(&hasura_transaction, &tenant_id, &scheduled_manage_date.id).await?;

    Ok(())
}

/// An event-wide ALLOW_INIT_REPORT, ALLOW_VOTING_PERIOD_END or ALLOW_TALLY
/// applies to every election of the event without its own row (`own_rows`),
/// as the election-scoped one does to its election.
#[instrument(skip(hasura_transaction, scheduled_event), err)]
pub async fn apply_to_every_election(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    scheduled_event: &ScheduledEvent,
    event_processor: &EventProcessors,
    own_rows: &HashSet<String>,
) -> AnyhowResult<()> {
    let payload = scheduled_event
        .event_payload
        .clone()
        .unwrap_or_else(|| serde_json::json!({}));
    let elections = get_elections(hasura_transaction, tenant_id, election_event_id).await?;
    for election in elections
        .into_iter()
        .filter(|election| !own_rows.contains(&election.id))
    {
        match event_processor {
            EventProcessors::ALLOW_INIT_REPORT => {
                let payload: ManageAllowInitPayload = deserialize_value(payload.clone())?;
                let mut status = election_status(&election)?;
                status.init_report = if payload.allow_init == Some(true) {
                    InitReport::ALLOWED
                } else {
                    InitReport::DISALLOWED
                };
                update_election_voting_status(
                    hasura_transaction,
                    tenant_id,
                    election_event_id,
                    &election.id,
                    serde_json::to_value(status)?,
                )
                .await?;
            }
            EventProcessors::ALLOW_TALLY => {
                let mut status = election_status(&election)?;
                status.allow_tally = AllowTallyStatus::ALLOWED;
                update_election_voting_status(
                    hasura_transaction,
                    tenant_id,
                    election_event_id,
                    &election.id,
                    serde_json::to_value(status)?,
                )
                .await?;
            }
            EventProcessors::ALLOW_VOTING_PERIOD_END => {
                let payload: ManageAllowVotingPeriodEndPayload =
                    deserialize_value(payload.clone())?;
                let mut presentation: ElectionPresentation = match election.presentation.clone() {
                    Some(value) => deserialize_value(value)?,
                    None => ElectionPresentation::default(),
                };
                presentation.voting_period_end = if payload.allow_voting_period_end != Some(false) {
                    Some(VotingPeriodEnd::ALLOWED)
                } else {
                    Some(VotingPeriodEnd::DISALLOWED)
                };
                update_election_presentation(
                    hasura_transaction,
                    tenant_id,
                    election_event_id,
                    &election.id,
                    serde_json::to_value(presentation)?,
                )
                .await?;
            }
            _ => return Err(anyhow!("{event_processor} isn't applied to every election")),
        }
    }
    Ok(())
}

fn election_status(election: &Election) -> AnyhowResult<ElectionStatus> {
    Ok(match election.status.clone() {
        Some(value) => deserialize_value(value)?,
        None => ElectionStatus::default(),
    })
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(time_limit = 10, max_retries = 0, expires = 30)]
pub async fn manage_election_event_date(
    tenant_id: String,
    election_event_id: String,
    scheduled_event_id: String,
) -> Result<()> {
    let lock: PgLock = PgLock::acquire(
        format!(
            "execute_manage_election_event_date-{}-{}-{}",
            tenant_id, election_event_id, scheduled_event_id
        ),
        Uuid::new_v4().to_string(),
        ISO8601::now() + Duration::seconds(120),
    )
    .await?;
    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|e| anyhow!("Error getting hasura client {}", e))?;
    let hasura_transaction = hasura_db_client.transaction().await?;
    let res = manage_election_event_date_wrapped(
        &hasura_transaction,
        tenant_id.clone(),
        election_event_id.clone(),
        scheduled_event_id.clone(),
    )
    .await;

    match res {
        Ok(data) => {
            let commit = hasura_transaction
                .commit()
                .await
                .map_err(|e| anyhow!("Commit failed manage_event_election_dates: {}", e));
            lock.release().await?;
            commit?;
            // A close may have created ballot box seals to make (VOTE-FREEZE).
            crate::tasks::seal_ballot_boxes::kick_ballot_box_sealer();
        }
        Err(err) => {
            let rollback = hasura_transaction.rollback().await;
            lock.release().await?;
            rollback?;
            return Err(anyhow!("{}", err).into());
        }
    }

    Ok(())
}
