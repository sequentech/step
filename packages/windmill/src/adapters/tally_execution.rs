// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::domain::tally_execution::ExecutionConclusion;
use crate::ports::tally_execution::*;
use crate::postgres::{
    election, election_event, keys_ceremony, tally_session, tally_session_execution,
};
use crate::services::ceremonies::{serialize_logs, tally_ceremony};
use crate::services::election_event_status::get_election_event_status;
use crate::types::error::Result;
use anyhow::anyhow;
use deadpool_postgres::Transaction;
use rand::{rngs::StdRng, seq::SliceRandom, SeedableRng};
use sequent_core::types::ceremonies::{TallyCeremonyStatus, TallyExecutionStatus};
use sequent_core::types::hasura::core::KeysCeremony;

pub struct PgTallyExecution<'a> {
    pub transaction: &'a Transaction<'a>,
}

impl ExecutionCeremonies for PgTallyExecution<'_> {
    async fn list(&self, tenant_id: &str, event_id: &str) -> anyhow::Result<Vec<KeysCeremony>> {
        keys_ceremony::get_keys_ceremonies(self.transaction, tenant_id, event_id).await
    }
}

pub struct RandomTrusteeOrder;
impl TrusteeOrder for RandomTrusteeOrder {
    fn shuffle(&self, trustees: &mut [String]) {
        trustees.shuffle(&mut StdRng::from_os_rng());
    }
}

pub struct TallyExecutionLogs;
impl ExecutionLogs for TallyExecutionLogs {
    fn append(
        &self,
        status: &mut TallyCeremonyStatus,
        conclusion: ExecutionConclusion,
        elections: &[String],
    ) {
        status.logs = if conclusion == ExecutionConclusion::Completed {
            serialize_logs::append_tally_finished(&status.logs, &elections.to_vec())
        } else {
            serialize_logs::append_tally_updated(&status.logs, &elections.to_vec())
        };
    }
}

impl ExecutionLedger for PgTallyExecution<'_> {
    async fn insert(&self, scope: &ExecutionScope, record: ExecutionRecord) -> Result<()> {
        tally_session_execution::insert_tally_session_execution(
            self.transaction,
            &scope.tenant_id,
            &scope.election_event_id,
            record.current_message_id,
            &scope.tally_session_id,
            Some(record.status),
            record.results_event_id,
            record.session_ids,
            record.documents,
            record.run_reason,
        )
        .await?;
        Ok(())
    }
    async fn await_input(&self, scope: &ExecutionScope) -> Result<()> {
        tally_session::update_tally_session_status(
            self.transaction,
            &scope.tenant_id,
            &scope.election_event_id,
            &scope.tally_session_id,
            TallyExecutionStatus::AWAITING_INPUT,
            false,
        )
        .await?;
        Ok(())
    }
    async fn complete(&self, scope: &ExecutionScope) -> Result<()> {
        tally_ceremony::set_tally_session_completed(
            self.transaction,
            scope.tenant_id.clone(),
            scope.election_event_id.clone(),
            scope.tally_session_id.clone(),
        )
        .await?;
        Ok(())
    }
    async fn refresh_event_status(&self, scope: &ExecutionScope) -> Result<()> {
        let event = election_event::get_election_event_by_id(
            self.transaction,
            &scope.tenant_id,
            &scope.election_event_id,
        )
        .await?;
        let status =
            get_election_event_status(event.status).ok_or(anyhow!("Empty election status"))?;
        let status_json = serde_json::to_value(status)?;
        election_event::update_election_event_status(
            self.transaction,
            &scope.tenant_id,
            &scope.election_event_id,
            status_json,
        )
        .await?;
        Ok(())
    }
    async fn mark_initialization(&self, scope: &ExecutionScope, election_id: &str) -> Result<()> {
        election::set_election_initialization_report_generated(
            self.transaction,
            &scope.tenant_id,
            &scope.election_event_id,
            election_id,
            &true,
        )
        .await?;
        Ok(())
    }
}
