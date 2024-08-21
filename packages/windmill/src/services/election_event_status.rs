// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::election::{get_election_by_id, update_election_voting_status};
use crate::postgres::election_event::{
    get_election_event_by_id, update_election_event_status,
    update_elections_status_by_election_event,
};
use anyhow::{anyhow, Result};
use deadpool_postgres::Transaction;
use sequent_core::ballot::*;
use sequent_core::serialization::deserialize_with_path::deserialize_value;
use sequent_core::types::hasura::core::ElectionEvent;
use serde_json::value::Value;
use tracing::{event, info, instrument, Level};

use super::database::get_hasura_pool;
use super::voting_status::update_board_on_status_change;

pub fn get_election_event_status(status_json_opt: Option<Value>) -> Option<ElectionEventStatus> {
    status_json_opt.and_then(|status_json| deserialize_value(status_json).ok())
}

pub fn get_election_status(status_json_opt: Option<Value>) -> Option<ElectionStatus> {
    status_json_opt.and_then(|status_json| deserialize_value(status_json).ok())
}

pub fn has_config_created(status_json_opt: Option<Value>) -> bool {
    get_election_event_status(status_json_opt)
        .map(|status| status.config_created)
        .unwrap_or(Some(false))
        .unwrap_or(false)
}

#[instrument(err)]
pub async fn update_event_voting_status(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    new_status: &VotingStatus,
) -> Result<ElectionEvent> {
    let election_event =
        get_election_event_by_id(hasura_transaction, tenant_id, election_event_id).await?;

    let mut status =
        get_election_event_status(election_event.status.clone()).unwrap_or(Default::default());
    let mut election_status = ElectionStatus::default();

    let current_voting_status = status.voting_status.clone();

    let expected_next_status = match current_voting_status {
        VotingStatus::NOT_STARTED => {
            vec![VotingStatus::OPEN]
        }
        VotingStatus::OPEN => {
            vec![VotingStatus::PAUSED, VotingStatus::CLOSED]
        }
        VotingStatus::PAUSED => {
            vec![VotingStatus::CLOSED, VotingStatus::OPEN]
        }
        VotingStatus::CLOSED => {
            vec![VotingStatus::OPEN]
        }
    };

    if !expected_next_status.contains(&new_status) {
        return Err(anyhow!(
            "Unexpected next status {:?}, expected {:?}",
            new_status,
            expected_next_status
        ));
    }

    status.voting_status = new_status.clone();

    update_election_event_status(
        &hasura_transaction,
        &&tenant_id,
        election_event_id,
        serde_json::to_value(&status)?,
    )
    .await?;
    let mut elections_ids: Vec<String> = Vec::new();
    if *new_status == VotingStatus::OPEN || *new_status == VotingStatus::CLOSED {
        election_status.voting_status = new_status.clone();
        elections_ids = update_elections_status_by_election_event(
            &hasura_transaction,
            &tenant_id,
            &election_event_id,
            serde_json::to_value(&election_status)?,
        )
        .await?;
    }

    update_board_on_status_change(
        election_event.id.to_string(),
        election_event.bulletin_board_reference.clone(),
        new_status.clone(),
        None,
        Some(elections_ids),
    )
    .await?;
    Ok(election_event)
}

#[instrument(err)]
pub async fn update_election_voting_status_impl(
    tenant_id: String,
    election_event_id: String,
    election_id: String,
    new_status: VotingStatus,
    hasura_transaction: &Transaction<'_>,
) -> Result<()> {
    let Some(election) = get_election_by_id(
        hasura_transaction,
        &tenant_id,
        &election_event_id,
        &election_id,
    )
    .await?
    else {
        event!(Level::WARN, "Election not found");
        return Ok(());
    };

    let mut status = get_election_status(election.status.clone()).unwrap_or(ElectionStatus {
        voting_status: VotingStatus::NOT_STARTED,
    });

    let current_voting_status = status.voting_status.clone();

    let expected_next_status = match current_voting_status {
        VotingStatus::NOT_STARTED => {
            vec![VotingStatus::OPEN]
        }
        VotingStatus::OPEN => {
            vec![VotingStatus::PAUSED, VotingStatus::CLOSED]
        }
        VotingStatus::PAUSED => {
            vec![VotingStatus::CLOSED, VotingStatus::OPEN]
        }
        VotingStatus::CLOSED => {
            vec![VotingStatus::OPEN]
        }
    };

    if !expected_next_status.contains(&new_status) {
        return Err(anyhow!(
            "Unexpected next status {:?}, expected {:?}",
            new_status,
            expected_next_status
        ));
    }

    status.voting_status = new_status;

    let status_js = serde_json::to_value(&status)?;

    update_election_voting_status(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        &election_id,
        status_js,
    )
    .await?;

    Ok(())
}
