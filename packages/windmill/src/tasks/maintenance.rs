use crate::postgres::election_event::{
    get_election_event_by_id, update_election_event_presentation,
};
use crate::postgres::maintenance::vacuum_analyze_direct;
use crate::postgres::scheduled_event::*;
use crate::services::database::get_hasura_pool;
use crate::services::pg_lock::PgLock;
use crate::services::providers::transactions_provider::provide_hasura_transaction;
use crate::services::voting_status::{self};
use crate::types::error::{Error, Result};
use anyhow::{anyhow, Context, Result as AnyhowResult};
use async_trait::async_trait;
use celery::error::TaskError;
use chrono::Duration;
use deadpool_postgres::Client as DbClient;
use deadpool_postgres::Transaction;
use sequent_core::ballot::{ElectionEventPresentation, InitReport, LockedDown, VotingStatus};
use sequent_core::serialization::deserialize_with_path::{self, deserialize_value};
use sequent_core::services::date::ISO8601;
use sequent_core::types::scheduled_event::*;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use tracing::{error, event, info, Level};
use uuid::Uuid;

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(time_limit = 10, max_retries = 0, expires = 3600)]
pub async fn database_maintenance() -> Result<()> {
    // Execute database maintenance
    info!("Performing scheduled database mainteinance.");
    vacuum_analyze_direct().await?;
    Ok(())
}
