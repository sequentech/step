// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Runs a protected action once its signing request has every signature;
//! see [`crate::services::signing::actions`].

use crate::services::database::get_hasura_pool;
use crate::services::signing::actions::{
    run_dispatched, NoSeal, ProductionEffects, RunOutcome, SignedActionTask,
};
use crate::types::error::Result;
use anyhow::anyhow;
use celery::error::TaskError;
use tracing::{info, instrument};

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn run_signed_action(
    tenant_id: String,
    election_event_id: String,
    request_id: String,
    task_execution_id: String,
) -> Result<()> {
    let task = SignedActionTask::parse(
        &tenant_id,
        &election_event_id,
        &request_id,
        &task_execution_id,
    )?;
    let mut client = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|error| anyhow!("Error getting the hasura client: {error:?}"))?;
    match run_dispatched(&mut client, &ProductionEffects::default(), &NoSeal, &task).await? {
        RunOutcome::NotClaimed => info!(%request_id, "another task runs this signed action"),
        RunOutcome::Executed(_) => info!(%request_id, "ran the signed action"),
        RunOutcome::Failed(code) => info!(%request_id, "the signed action failed: {code}"),
        RunOutcome::Retry(error) => {
            return Err(anyhow!("signed action {request_id} failed before acting: {error}").into())
        }
    }
    Ok(())
}
