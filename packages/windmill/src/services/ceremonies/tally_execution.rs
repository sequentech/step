// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::domain::tally_execution::{
    eligible_trustees, select_trustees, ExecutionConclusion, TrusteeSelection,
};
use crate::ports::tally_execution::*;
use crate::types::error::Result;
use anyhow::Context;
use sequent_core::types::ceremonies::{TallyCeremonyStatus, TallyRunReason, TallyType};
use sequent_core::types::hasura::core::KeysCeremony;

pub async fn select_execution_trustees_with(
    ceremonies: &impl ExecutionCeremonies,
    order: &impl TrusteeOrder,
    tenant_id: &str,
    event_id: &str,
    linked_ceremony: &KeysCeremony,
    status: TallyCeremonyStatus,
) -> anyhow::Result<TrusteeSelection> {
    let existing = ceremonies
        .list(tenant_id, event_id)
        .await
        .with_context(|| "error listing existing keys ceremonies")?;
    let Some(first) = existing.first() else {
        return Ok(TrusteeSelection::NoCeremony);
    };
    let threshold = first.threshold as usize;
    let mut available = eligible_trustees(status.trustees, linked_ceremony.policy());
    order.shuffle(&mut available);
    Ok(select_trustees(available, threshold))
}

pub async fn persist_execution_with(
    ledger: &impl ExecutionLedger,
    logs: &impl ExecutionLogs,
    scope: &ExecutionScope,
    mut record: ExecutionRecord,
    conclusion: ExecutionConclusion,
    tally_type: TallyType,
    elections: Vec<String>,
) -> Result<()> {
    logs.append(&mut record.status, conclusion, &elections);
    record.run_reason = TallyRunReason::NORMAL;
    ledger.insert(scope, record).await?;
    match conclusion {
        ExecutionConclusion::AwaitingInput => ledger.await_input(scope).await?,
        ExecutionConclusion::InProgress => {}
        ExecutionConclusion::Completed => {
            ledger.complete(scope).await?;
            ledger.refresh_event_status(scope).await?;
            if tally_type == TallyType::INITIALIZATION_REPORT {
                for election in elections {
                    ledger.mark_initialization(scope, &election).await?;
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "tally_execution_tests.rs"]
mod tests;
