// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::election::get_election_by_id;
use crate::postgres::election_event::get_election_event_by_id;
use crate::postgres::scheduled_event::*;
use crate::services::database::get_hasura_pool;
use crate::services::election_event_status::scheduled_change_applies;
use crate::services::initialization_schedule::{scheduled_post_opening, ScheduledPostOpening};
use crate::services::pg_lock::PgLock;
use crate::services::providers::transactions_provider::provide_hasura_transaction;
use crate::services::signing::actions::voting::scheduled_change_needs_signatures;
use crate::services::voting_status::{self};
use crate::tasks::scheduled_events::fires_later;
use crate::types::error::{Error, Result};
use anyhow::{anyhow, Context, Result as AnyhowResult};
use async_trait::async_trait;
use celery::error::TaskError;
use chrono::Duration;
use deadpool_postgres::Client as DbClient;
use deadpool_postgres::Transaction;
use sequent_core::ballot::{ElectionStatus, VotingStatus, VotingStatusChannel};
use sequent_core::services::date::ISO8601;
use sequent_core::types::scheduled_event::*;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use tracing::{error, event, info, Level};
use uuid::Uuid;

#[instrument(err)]
async fn manage_election_date_wrapper(
    hasura_transaction: &Transaction<'_>,
    tenant_id: String,
    election_event_id: String,
    scheduled_event_id: String,
    election_id: String,
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
    .await
    .with_context(|| "Error obtaining scheduled event by id")?;

    let Some(scheduled_manage_date) = scheduled_manage_date_opt else {
        return Err(anyhow!(
            "Can't find scheduled event with id: {}",
            scheduled_event_id
        ));
    };

    let payload: ManageElectionDatePayload = serde_json::from_value(
        scheduled_manage_date
            .event_payload
            .clone()
            .unwrap_or_else(|| serde_json::json!({})),
    )?;
    if payload.election_id.as_deref() != Some(election_id.as_str()) {
        return Err(anyhow!(
            "The queued scheduled transition no longer targets this Post"
        ));
    }
    if fires_later(&scheduled_manage_date, chrono::Utc::now()) {
        info!("Scheduled event {scheduled_event_id} was moved to a later time; it runs then");
        return Ok(());
    }
    // The election's status is read and written below.
    lock_elections(
        hasura_transaction,
        &tenant_id,
        &election_event_id,
        Some(&election_id),
    )
    .await?;

    let Some(election) = get_election_by_id(
        hasura_transaction,
        &tenant_id,
        &election_event_id,
        &election_id,
    )
    .await
    .with_context(|| "Error obtaining election by id")?
    else {
        return Err(anyhow!("Election not found"));
    };

    let Some(event_processor) = scheduled_manage_date.event_processor.clone() else {
        return Err(anyhow!("Missing event processor"));
    };

    let status = match event_processor {
        EventProcessors::START_VOTING_PERIOD => VotingStatus::OPEN,
        EventProcessors::END_VOTING_PERIOD => VotingStatus::CLOSED,
        _ => {
            info!("Invalid scheduled event type: {:?}", event_processor);
            stop_scheduled_event(&hasura_transaction, &tenant_id, &scheduled_manage_date.id)
                .await?;
            return Ok(());
        }
    };

    let configured = election
        .voting_channels
        .clone()
        .map(serde_json::from_value::<sequent_core::types::hasura::core::VotingChannels>)
        .transpose()?
        .unwrap_or_default();
    let election_status: ElectionStatus = election
        .status
        .clone()
        .map(serde_json::from_value)
        .transpose()?
        .unwrap_or_default();
    let seal_policy = crate::services::ballot_box_seal::seal_policy(
        &crate::postgres::election_event::get_election_event_by_id(
            hasura_transaction,
            &tenant_id,
            &election_event_id,
        )
        .await?,
    );
    if status == VotingStatus::CLOSED
        && seal_policy == sequent_core::ballot::BallotBoxSealPolicy::SEAL_AT_CLOSE
        && crate::services::election_event_status::never_opened(&election_status, &configured)
    {
        info!(
            %election_id,
            reason = crate::services::election_event_status::NEVER_OPENED_REASON,
            "Nothing to close on schedule: the Post never opened, so it stays as it is"
        );
    }
    let voting_channels = payload
        .enabled_channels(&configured)
        .into_iter()
        .filter(|channel| {
            scheduled_change_applies(
                &election_status,
                &configured,
                *channel,
                &status,
                seal_policy,
            )
        })
        .collect::<Vec<_>>();

    // An opening waits for the Post's initialization at the event's scope,
    // and never runs once its voting period has closed (VOTE-LIFECYCLE §9).
    let may_run = match crate::services::scheduled_outcome::fire_time_state(
        hasura_transaction,
        Uuid::parse_str(&tenant_id)?,
        Uuid::parse_str(&election_event_id)?,
        &scheduled_manage_date.id,
    )
    .await?
    {
        Some((state, row)) => {
            state
                .explain(
                    &row,
                    Some(Uuid::parse_str(&election_id)?),
                    crate::services::scheduled_outcome::Moment::FireTime,
                )
                .outcome
                != sequent_core::types::scheduled_outcome::ScheduledOutcomeKind::Refused
        }
        None => false,
    };
    if status == VotingStatus::OPEN && !voting_channels.is_empty() && may_run {
        match scheduled_post_opening(
            hasura_transaction,
            &tenant_id,
            &election_event_id,
            &election,
            &scheduled_manage_date,
        )
        .await?
        {
            ScheduledPostOpening::Open => {}
            ScheduledPostOpening::Wait => return Ok(()),
            ScheduledPostOpening::AfterClose => {
                stop_scheduled_event(hasura_transaction, &tenant_id, &scheduled_manage_date.id)
                    .await?;
                return Ok(());
            }
        }
    }

    // The locked schedule is evaluated against both configuration copies.
    if scheduled_change_needs_signatures(
        hasura_transaction,
        &tenant_id,
        &election_event_id,
        Some(&election_id),
        &status,
        &scheduled_manage_date.id,
    )
    .await?
    {
        stop_scheduled_event(&hasura_transaction, &tenant_id, &scheduled_manage_date.id).await?;
        return Ok(());
    }

    let result = voting_status::update_election_status(
        tenant_id.clone(),
        None,
        None,
        hasura_transaction,
        &election_event_id,
        &election_id,
        &status,
        &Some(voting_channels),
    )
    .await;
    info!("result: {result:?}");

    stop_scheduled_event(&hasura_transaction, &tenant_id, &scheduled_manage_date.id)
        .await
        .map_err(|err| anyhow!("Error stopping scheduled event: {err:?}"))?;

    result?;

    Ok(())
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(time_limit = 10, max_retries = 0, expires = 30)]
pub async fn manage_election_date(
    tenant_id: String,
    election_event_id: String,
    scheduled_event_id: String,
    election_id: String,
) -> Result<()> {
    let lock: PgLock = PgLock::acquire(
        format!(
            "execute_manage_election_date-{}-{}-{}-{}",
            tenant_id, election_event_id, scheduled_event_id, election_id
        ),
        Uuid::new_v4().to_string(),
        ISO8601::now() + Duration::seconds(120),
    )
    .await
    .with_context(|| "Error acquiring pglock")?;

    let res = provide_hasura_transaction(|hasura_transaction| {
        let tenant_id = tenant_id.clone();
        let election_event_id = election_event_id.clone();
        let scheduled_event_id = scheduled_event_id.clone();
        let election_id = election_id.clone();
        Box::pin(async move {
            // Your async code here
            manage_election_date_wrapper(
                hasura_transaction,
                tenant_id,
                election_event_id,
                scheduled_event_id,
                election_id,
            )
            .await
        })
    })
    .await;

    info!("result: {:?}", res);

    lock.release()
        .await
        .with_context(|| "Error releasing pglock")?;

    res?;
    // A close may have created ballot box seals to make (VOTE-FREEZE).
    crate::tasks::seal_ballot_boxes::kick_ballot_box_sealer();
    Ok(())
}
