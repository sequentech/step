// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::domain::tally_execution::ExecutionConclusion;
use crate::ports::tally_execution::*;
use crate::types::error::{Error, Result};
use sequent_core::types::ceremonies::{Log, TallyCeremonyStatus};
use sequent_core::types::hasura::core::KeysCeremony;
use std::{collections::HashMap, sync::Mutex};

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum Write {
    Insert,
    AwaitInput,
    Complete,
    RefreshEvent,
    Initialization,
}
#[derive(Default)]
pub struct ExecutionState {
    pub ceremonies: Vec<KeysCeremony>,
    pub read_failure: Option<&'static str>,
    pub reads: Vec<(String, String)>,
    pub failure: Option<(Write, usize)>,
    attempts: HashMap<Write, usize>,
    pub records: Vec<(ExecutionScope, ExecutionRecord)>,
    pub awaiting_input: Vec<ExecutionScope>,
    pub completed: Vec<ExecutionScope>,
    pub refreshed_events: Vec<ExecutionScope>,
    pub initialized: Vec<(ExecutionScope, String)>,
}
impl ExecutionState {
    fn check(&mut self, write: Write) -> Result<()> {
        let attempt = self.attempts.entry(write).or_default();
        *attempt += 1;
        if self.failure == Some((write, *attempt)) {
            return Err(Error::String(format!("injected {write:?} failure")));
        }
        Ok(())
    }
}
#[derive(Default)]
pub struct MemoryTallyExecution(pub Mutex<ExecutionState>);
impl ExecutionCeremonies for MemoryTallyExecution {
    async fn list(&self, tenant: &str, event: &str) -> anyhow::Result<Vec<KeysCeremony>> {
        let mut state = self.0.lock().unwrap();
        state.reads.push((tenant.into(), event.into()));
        if let Some(message) = state.read_failure {
            anyhow::bail!(message);
        }
        Ok(state.ceremonies.clone())
    }
}
impl ExecutionLedger for MemoryTallyExecution {
    async fn insert(&self, scope: &ExecutionScope, record: ExecutionRecord) -> Result<()> {
        let mut state = self.0.lock().unwrap();
        state.check(Write::Insert)?;
        state.records.push((scope.clone(), record));
        Ok(())
    }
    async fn await_input(&self, scope: &ExecutionScope) -> Result<()> {
        let mut state = self.0.lock().unwrap();
        state.check(Write::AwaitInput)?;
        state.awaiting_input.push(scope.clone());
        Ok(())
    }
    async fn complete(&self, scope: &ExecutionScope) -> Result<()> {
        let mut state = self.0.lock().unwrap();
        state.check(Write::Complete)?;
        state.completed.push(scope.clone());
        Ok(())
    }
    async fn refresh_event_status(&self, scope: &ExecutionScope) -> Result<()> {
        let mut state = self.0.lock().unwrap();
        state.check(Write::RefreshEvent)?;
        state.refreshed_events.push(scope.clone());
        Ok(())
    }
    async fn mark_initialization(&self, scope: &ExecutionScope, election: &str) -> Result<()> {
        let mut state = self.0.lock().unwrap();
        state.check(Write::Initialization)?;
        state.initialized.push((scope.clone(), election.into()));
        Ok(())
    }
}
#[derive(Default)]
pub struct ReverseTrusteeOrder(pub Mutex<Vec<Vec<String>>>);
impl TrusteeOrder for ReverseTrusteeOrder {
    fn shuffle(&self, trustees: &mut [String]) {
        self.0.lock().unwrap().push(trustees.to_vec());
        trustees.reverse();
    }
}
pub struct FixedExecutionLogs;
impl ExecutionLogs for FixedExecutionLogs {
    fn append(
        &self,
        status: &mut TallyCeremonyStatus,
        conclusion: ExecutionConclusion,
        elections: &[String],
    ) {
        status.logs.push(Log {
            created_date: "2026-01-01T00:00:00.000Z".into(),
            log_text: format!("{conclusion:?}: {elections:?}"),
        });
    }
}
