// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::{
    cast_votes::get_in_progress_cast_votes_batch,
    celery_app::get_celery_app,
    database::{get_hasura_pool, PgConfig},
};
use crate::tasks::process_cast_vote::process_cast_vote;
use crate::types::error::Result;
use anyhow::anyhow;
use celery::error::TaskError;
use deadpool_postgres::Client as DbClient;
use tracing::{info, instrument};
use uuid::Uuid;

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn review_cast_votes() -> Result<()> {
    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|e| anyhow!("Error getting hasura client {e:?}"))?;
    // Read-only transaction: PostgreSQL rejects any write inside it, and it is
    // dropped without commit (rollback) at the end of the task. It only serves
    // as the read context for the keyset-paginated scan below.
    let hasura_transaction = hasura_db_client
        .build_transaction()
        .read_only(true)
        .start()
        .await
        .map_err(|e| anyhow!("Error creating a hasura transaction {e:?}"))?;
    let celery_app = get_celery_app().await;

    let mut after: Option<(Uuid, Uuid, Uuid, String)> = None;
    let batch_size = PgConfig::from_env()?.default_sql_batch_size.into();

    info!("review_cast_votes: Checking cast_votes in progress");
    while let Some(ballots_list) =
        get_in_progress_cast_votes_batch(&hasura_transaction, batch_size, after.clone()).await?
    {
        info!(
            "review_cast_votes: Processing {} cast votes",
            ballots_list.len()
        );
        // For this Celery has to be properly configured with acks_late=true and a realistic value for prefetch_count, which establishes the number of tasks executed in parallel.
        for ballot in &ballots_list {
            celery_app
                .send_task(process_cast_vote::new(
                    ballot.tenant_id.to_string(),
                    ballot.election_event_id.to_string(),
                    ballot.id.clone(),
                ))
                .await
                .map_err(|e| anyhow!("Error sending cast_vote_actions task: {e:?}"))?;
        }
        // Move to next batch
        after = ballots_list.last().map(|ballot| {
            (
                ballot.tenant_id,
                ballot.election_event_id,
                ballot.election_id,
                ballot.voter_id.clone(),
            )
        });
    }
    Ok(())
}
