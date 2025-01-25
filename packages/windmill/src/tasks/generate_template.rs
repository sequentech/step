// SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::tasks_execution::update_fail;
use crate::types::error::Error;
use crate::types::error::Result;
use anyhow::{anyhow, Context};
use celery::error::TaskError;
use deadpool_postgres::Client as DbClient;
use rocket::serde::json::Json;
use sequent_core::types::hasura::core::TasksExecution;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tracing::info;
use tracing::instrument;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum EGenerateTemplate {
    BallotImages {
        election_event_id: String,
        election_id: String,
        tally_session_id: String,
    },
    VoteReceipts {
        election_event_id: String,
        election_id: String,
        tally_session_id: String,
    },
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task]
pub async fn generate_template(
    document_id: String,
    task_execution: Option<TasksExecution>,
    executer_username: Option<String>,
) -> Result<()> {
    Ok(())
}
