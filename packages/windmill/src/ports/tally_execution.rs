// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::domain::tally_execution::ExecutionConclusion;
use crate::types::error::Result;
use sequent_core::types::ceremonies::{TallyCeremonyStatus, TallyRunReason, TallySessionDocuments};
use sequent_core::types::hasura::core::KeysCeremony;
use std::future::Future;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionScope {
    pub tenant_id: String,
    pub election_event_id: String,
    pub tally_session_id: String,
}

#[derive(Clone, Debug)]
pub struct ExecutionRecord {
    pub current_message_id: i32,
    pub status: TallyCeremonyStatus,
    pub results_event_id: Option<String>,
    pub session_ids: Option<Vec<i32>>,
    pub documents: Option<TallySessionDocuments>,
    pub run_reason: TallyRunReason,
}

pub trait ExecutionCeremonies: Sync {
    fn list(
        &self,
        tenant_id: &str,
        event_id: &str,
    ) -> impl Future<Output = anyhow::Result<Vec<KeysCeremony>>> + Send;
}

pub trait TrusteeOrder: Sync {
    fn shuffle(&self, trustees: &mut [String]);
}

pub trait ExecutionLogs: Sync {
    fn append(
        &self,
        status: &mut TallyCeremonyStatus,
        conclusion: ExecutionConclusion,
        elections: &[String],
    );
}

pub trait ExecutionLedger: Sync {
    fn insert(
        &self,
        scope: &ExecutionScope,
        record: ExecutionRecord,
    ) -> impl Future<Output = Result<()>> + Send;
    fn await_input(&self, scope: &ExecutionScope) -> impl Future<Output = Result<()>> + Send;
    fn complete(&self, scope: &ExecutionScope) -> impl Future<Output = Result<()>> + Send;
    fn refresh_event_status(
        &self,
        scope: &ExecutionScope,
    ) -> impl Future<Output = Result<()>> + Send;
    fn mark_initialization(
        &self,
        scope: &ExecutionScope,
        election_id: &str,
    ) -> impl Future<Output = Result<()>> + Send;
}
