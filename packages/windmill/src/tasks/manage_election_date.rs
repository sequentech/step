use crate::hasura::election_event::{get_election_event, get_election_event_helper};
// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::hasura::election_event::update_election_event_status;
use crate::postgres::election::get_election_by_id;
use crate::postgres::scheduled_event::*;
use crate::services::database::get_hasura_pool;
use crate::services::election_event_board::get_election_event_board;
use crate::services::election_event_status;
use crate::services::election_event_status::get_election_event_status;
use crate::services::electoral_log::*;
use crate::types::error::{Error, Result};
use crate::types::scheduled_event::EventProcessors;
use anyhow::{anyhow, Context};
use celery::error::TaskError;
use deadpool_postgres::Client as DbClient;
use sequent_core::ballot::{ElectionEventStatus, VotingStatus};
use sequent_core::services::keycloak::get_client_credentials;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use tracing::{event, Level};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ManageElectionDatePayload {
    pub election_id: String,
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task]
pub async fn manage_election_date(
    tenant_id: Option<String>,
    election_event_id: Option<String>,
    scheduled_event_id: String,
) -> Result<()> {
    let auth_headers = get_client_credentials().await?;
    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|e| anyhow!("Error getting hasura client {}", e))?;
    let hasura_transaction = hasura_db_client.transaction().await?;
    let scheduled_manage_date_opt = find_scheduled_event_by_id(
        &hasura_transaction,
        tenant_id,
        election_event_id,
        &scheduled_event_id,
    )
    .await?;
    let Some(scheduled_manage_date) = scheduled_manage_date_opt else {
        event!(
            Level::WARN,
            "Can't find scheduled event with id: {scheduled_event_id}"
        );
        return Ok(());
    };

    let Some(tenant_id) = scheduled_manage_date.tenant_id.clone() else {
        event!(Level::WARN, "Missing tenant_id");
        return Ok(());
    };

    let Some(election_event_id) = scheduled_manage_date.election_event_id.clone() else {
        event!(Level::WARN, "Missing election_event_id");
        return Ok(());
    };

    let Some(event_payload) = scheduled_manage_date.event_payload.clone() else {
        event!(Level::WARN, "Missing event_payload");
        return Ok(());
    };
    let payload: ManageElectionDatePayload = serde_json::from_value(event_payload)?;

    let Some(_election) = get_election_by_id(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        &payload.election_id,
    )
    .await?
    else {
        event!(Level::WARN, "Election not found");
        return Ok(());
    };

    let election_event = get_election_event_helper(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
    )
    .await?;
    let mut status: ElectionEventStatus =
        get_election_event_status(election_event.status).unwrap_or(Default::default());

    let Some(event_processor) = scheduled_manage_date.event_processor.clone() else {
        event!(Level::WARN, "Missing event processor");
        return Ok(());
    };

    status.voting_status = if EventProcessors::START_ELECTION == event_processor {
        VotingStatus::OPEN
    } else {
        VotingStatus::CLOSED
    };

    // update the database
    update_election_event_status(
        auth_headers,
        tenant_id.to_string(),
        election_event_id.to_string(),
        serde_json::to_value(status.clone())?,
    )
    .await?;

    // update the board
    let board_name = get_election_event_board(election_event.bulletin_board_reference.clone())
        .with_context(|| "missing bulletin board")?;

    let electoral_log = ElectoralLog::new(board_name.as_str()).await?;

    match status.voting_status {
        VotingStatus::OPEN => {
            electoral_log
                .post_election_open(election_event_id.clone(), None)
                .await
                .with_context(|| "error posting to the electoral log")?;
        }
        VotingStatus::CLOSED => {
            electoral_log
                .post_election_close(election_event_id.clone(), None)
                .await
                .with_context(|| "error posting to the electoral log")?;
        }
        voting_status @ _ => {
            return Err(Error::Anyhow(anyhow!(
                "Invalid scheduled event type: {voting_status:?}"
            )));
        }
    };
    stop_scheduled_event(&hasura_transaction, &tenant_id, &scheduled_manage_date.id).await?;

    let _commit = hasura_transaction
        .commit()
        .await
        .map_err(|e| anyhow!("Commit failed: {}", e));

    Ok(())
}
