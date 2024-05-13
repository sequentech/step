// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::hasura;
use crate::postgres::scheduled_event::find_all_active_events;
use crate::services::celery_app::get_celery_app;
use crate::services::database::get_hasura_pool;
use crate::services::date::ISO8601;
use crate::tasks::process_board::process_board;
use crate::types::error::Result;
use crate::types::scheduled_event::EventProcessors;
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
    Ok(())
}
