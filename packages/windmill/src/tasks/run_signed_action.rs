// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Runs a protected action once its signing request has every signature;
//! see [`crate::services::signing::actions`].

use crate::services::ballot_box_seal::sink::PendingSealSink;
use crate::services::database::get_hasura_pool;
use crate::services::signing::actions::reports::{mail_held_report, HeldMail, StoredReports};
use crate::services::signing::actions::{
    run_dispatched, ProductionEffects, RunOutcome, SignedActionTask,
};
use crate::services::signing::pdf::S3RevisionStore;
use crate::tasks::seal_ballot_boxes::kick_ballot_box_sealer;
use crate::types::error::Result;
use anyhow::anyhow;
use celery::error::TaskError;
use tracing::{info, instrument, warn};

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
    // A signed close records its request and signers on the seals the close
    // made (VOTE-FREEZE).
    let seal = PendingSealSink {
        tenant_id: task.tenant_id,
        election_event_id: task.election_event_id,
        request_id: task.request_id,
    };
    let outcome = run_dispatched(&mut client, &ProductionEffects::default(), &seal, &task).await?;
    if matches!(outcome, RunOutcome::Executed(_)) {
        kick_ballot_box_sealer();
    }
    // A released report's held e-mail goes after its release committed,
    // once, also when another copy of the task released it.
    if matches!(outcome, RunOutcome::Executed(_) | RunOutcome::NotClaimed) {
        match mail_held_report(
            &mut client,
            &S3RevisionStore,
            &StoredReports,
            task.tenant_id,
            task.election_event_id,
            task.request_id,
        )
        .await
        {
            Ok(HeldMail::Failed(error)) => warn!(%request_id, "held e-mail failed: {error}"),
            Ok(_) => {}
            Err(error) => warn!(%request_id, "held e-mail not sent: {error:?}"),
        }
    }
    match outcome {
        RunOutcome::NotClaimed => info!(%request_id, "another task runs this signed action"),
        RunOutcome::Executed(_) => info!(%request_id, "ran the signed action"),
        RunOutcome::Failed(code) => info!(%request_id, "the signed action failed: {code}"),
        RunOutcome::Retry(error) => {
            return Err(anyhow!("signed action {request_id} failed before acting: {error}").into())
        }
    }
    Ok(())
}
