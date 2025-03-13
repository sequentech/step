// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// use crate::postgres::election_event::{get_election_event_by_id, ElectionEventDatafix};

use crate::services::{
    cast_votes::{get_cast_votes_batch_by_status, CastVoteStatus},
    celery_app::get_celery_app,
    database::{get_hasura_pool, PgConfig},
};
use crate::tasks::process_cast_vote::process_cast_vote;
use crate::types::error::Result;
use anyhow::anyhow;
use celery::error::TaskError;
use deadpool_postgres::Client as DbClient;
use tracing::{error, info, instrument};

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn review_cast_votes() -> Result<()> {
    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|e| anyhow!("Error getting hasura client {e:?}"))?;
    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|e| anyhow!("Error creating a hasura transaction {e:?}"))?;
    let celery_app = get_celery_app().await;

    let mut offset = 0;
    let batch_size = PgConfig::from_env()?.default_sql_batch_size.into();

    info!("review_cast_votes: Checking cast_votes in progress");
    while let Some(ballots_list) = get_cast_votes_batch_by_status(
        &hasura_transaction,
        CastVoteStatus::InProgress,
        batch_size,
        offset,
    )
    .await?
    {
        info!(
            "review_cast_votes: Processing {} cast votes",
            ballots_list.len()
        );
        // For this celery has to be propperly configured with acks_late=true and a ralistic value for prefetch_count which stablishes the number of tasks executed in parallel
        for ballot in ballots_list {
            celery_app
                .send_task(process_cast_vote::new(ballot.clone()))
                .await
                .map_err(|e| anyhow!("Error sending cast_vote_actions task: {e:?}"))?;
        }
        // Move to next batch
        offset += batch_size;
    }
    Ok(())
}
