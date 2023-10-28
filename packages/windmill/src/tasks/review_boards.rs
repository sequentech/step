// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::hasura;
use crate::tasks::process_board::process_board;
use crate::types::task_error::into_task_error;
use celery::error::TaskError;
use celery::task::TaskResult;
use chrono::Utc;
use sequent_core::services::openid;
use tracing::instrument;
use tracing::{event, Level};

#[instrument]
#[celery::task]
pub async fn review_boards() -> TaskResult<()> {
    let limit: i64 = 100;
    let mut offset: i64 = 0;
    let mut last_length = limit;
    let auth_headers = openid::get_client_credentials()
        .await
        .map_err(into_task_error)?;

    while last_length == limit {
        let hasura_response = hasura::election_event::get_batch_election_events(
            auth_headers.clone(),
            limit.clone(),
            offset.clone(),
        )
        .await
        .map_err(into_task_error)?;
        let election_events = &hasura_response
            .data
            .expect("expected data".into())
            .sequent_backend_election_event;
        last_length = election_events.len() as i64;
        offset = offset + last_length;

        for election_event in election_events {
            let task2 = celery_app
                .send_task(process_board::new(election_event.id, election_event.tenant_id))
                .await?;
            event!(Level::INFO, "Sent task {}", task2.task_id);
        }
    }

    Ok(())
}
