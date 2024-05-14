// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::hasura;
use crate::postgres::scheduled_event::find_all_active_events;
use crate::postgres::scheduled_event::*;
use crate::services::celery_app::get_celery_app;
use crate::services::database::get_hasura_pool;
use crate::services::date::ISO8601;
use crate::tasks::process_board::process_board;
use crate::types::error::Result;
use crate::types::scheduled_event::EventProcessors;
use anyhow::anyhow;
use celery::error::TaskError;
use chrono::Duration;
use deadpool_postgres::Client as DbClient;
use sequent_core::services::keycloak;
use tracing::instrument;
use tracing::{event, Level};

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task]
pub async fn manage_election_date(
    tenant_id: Option<String>,
    election_event_id: Option<String>,
    scheduled_event_id: String,
) -> Result<()> {
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

    /*let election = get_election_by_id(
        &hasura_transaction,
        tenant_id,
        election_event_id,
        election_id,
    ).await?;*/

    let commit = hasura_transaction
        .commit()
        .await
        .map_err(|e| anyhow!("Commit failed: {}", e));

    Ok(())
}
