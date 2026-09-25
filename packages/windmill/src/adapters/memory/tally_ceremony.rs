// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::domain::tally_ceremony::TallyExecuter;
use crate::ports::tally_ceremony::{
    ElectionEventReader, ElectionsById, EnvironmentSlug, KeysCeremonyReader, TallyCeremonyAudit,
    TallySessions, TrusteePrivateKeys,
};
use anyhow::{anyhow, Result};
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::ceremonies::{TallyCeremonyStatus, TallyExecutionStatus, TallyRunReason};
use sequent_core::types::hasura::core::{
    Election, ElectionEvent, KeysCeremony, TallySession, TallySessionExecution,
};
use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};

/// A port operation that a test can make fail.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TallyCall {
    GetSession,
    LockSession,
    LastExecution,
    LastExecutionAndSession,
    AppendExecution,
    SetStatus,
    MarkCompleted,
    GetKeysCeremony,
    GetPrivateKey,
    GetElections,
    GetElectionEvent,
    EnvSlug,
    TallyOpened,
    KeyRestored,
    TallyClosed,
}

/// An electoral log entry posted by the tally ceremony.
#[derive(Debug, Clone, PartialEq)]
pub enum TallyAuditEntry {
    TallyOpened {
        board_name: String,
        tenant_id: String,
        election_event_id: String,
        election_ids: Option<Vec<String>>,
        user_id: String,
        username: String,
    },
    KeyRestored {
        board_name: String,
        tenant_id: String,
        election_event_id: String,
        election_ids: Option<Vec<String>>,
        trustee_name: String,
        user_id: String,
        username: Option<String>,
    },
    TallyClosed {
        board_name: String,
        tenant_id: String,
        election_event_id: String,
        election_ids: Option<Vec<String>>,
        executer: TallyExecuter,
    },
}

type LockHook = Box<dyn FnOnce(&mut TallySession) + Send>;

fn is_session(
    session: &TallySession,
    tenant_id: &str,
    election_event_id: &str,
    tally_session_id: &str,
) -> bool {
    session.tenant_id == tenant_id
        && session.election_event_id == election_event_id
        && session.id == tally_session_id
}

#[derive(Default)]
struct State {
    sessions: Vec<TallySession>,
    executions: Vec<TallySessionExecution>,
    keys_ceremonies: Vec<KeysCeremony>,
    private_keys: HashMap<(String, String), String>,
    elections: Vec<Election>,
    election_events: Vec<ElectionEvent>,
    env_slug: String,
    audit: Vec<TallyAuditEntry>,
    failures: HashMap<TallyCall, String>,
    on_lock: Option<LockHook>,
}

impl State {
    fn check(&self, call: TallyCall) -> Result<()> {
        match self.failures.get(&call) {
            Some(message) => Err(anyhow!("{message}")),
            None => Ok(()),
        }
    }

    fn session(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        tally_session_id: &str,
    ) -> Result<&TallySession> {
        self.sessions
            .iter()
            .find(|session| is_session(session, tenant_id, election_event_id, tally_session_id))
            .ok_or_else(|| anyhow!("Tally Session {tally_session_id} not found"))
    }

    /// Like the `UPDATE` statements it stands for, a missing row is ignored.
    fn update_session(
        &mut self,
        tenant_id: &str,
        election_event_id: &str,
        tally_session_id: &str,
        update: impl FnOnce(&mut TallySession),
    ) {
        if let Some(session) = self
            .sessions
            .iter_mut()
            .find(|session| is_session(session, tenant_id, election_event_id, tally_session_id))
        {
            update(session);
        }
    }

    fn last_execution(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        tally_session_id: &str,
    ) -> Option<TallySessionExecution> {
        self.executions
            .iter()
            .rev()
            .find(|execution| {
                execution.tenant_id == tenant_id
                    && execution.election_event_id == election_event_id
                    && execution.tally_session_id == tally_session_id
            })
            .cloned()
    }
}

/// Tally sessions, their executions, the keys ceremony, trustee keys,
/// elections and the electoral log, kept in memory.
#[derive(Default)]
pub struct InMemoryTallyCeremony {
    state: Mutex<State>,
}

impl InMemoryTallyCeremony {
    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().expect("tally ceremony state lock")
    }

    pub fn add_session(&self, session: TallySession) {
        self.state().sessions.push(session);
    }

    pub fn add_execution(&self, execution: TallySessionExecution) {
        self.state().executions.push(execution);
    }

    pub fn add_keys_ceremony(&self, keys_ceremony: KeysCeremony) {
        self.state().keys_ceremonies.push(keys_ceremony);
    }

    pub fn add_private_key(&self, keys_ceremony_id: &str, trustee_name: &str, key: &str) {
        self.state().private_keys.insert(
            (keys_ceremony_id.to_string(), trustee_name.to_string()),
            key.to_string(),
        );
    }

    pub fn add_election(&self, election: Election) {
        self.state().elections.push(election);
    }

    pub fn add_election_event(&self, election_event: ElectionEvent) {
        self.state().election_events.push(election_event);
    }

    pub fn set_env_slug(&self, slug: &str) {
        self.state().env_slug = slug.to_string();
    }

    pub fn fail(&self, call: TallyCall, message: &str) {
        self.state().failures.insert(call, message.to_string());
    }

    /// Runs `hook` on the session while `lock_for_update` holds it, as a
    /// transaction that committed while this one waited for the lock would.
    pub fn on_lock(&self, hook: impl FnOnce(&mut TallySession) + Send + 'static) {
        self.state().on_lock = Some(Box::new(hook));
    }

    pub fn session(&self, tally_session_id: &str) -> TallySession {
        self.state()
            .sessions
            .iter()
            .find(|session| session.id == tally_session_id)
            .cloned()
            .expect("tally session")
    }

    pub fn executions(&self, tally_session_id: &str) -> Vec<TallySessionExecution> {
        self.state()
            .executions
            .iter()
            .filter(|execution| execution.tally_session_id == tally_session_id)
            .cloned()
            .collect()
    }

    pub fn audit_entries(&self) -> Vec<TallyAuditEntry> {
        self.state().audit.clone()
    }

    fn record(&self, call: TallyCall, entry: TallyAuditEntry) -> Result<()> {
        let mut state = self.state();
        state.check(call)?;
        state.audit.push(entry);
        Ok(())
    }
}

impl TallySessions for InMemoryTallyCeremony {
    async fn get(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        tally_session_id: &str,
    ) -> Result<TallySession> {
        let state = self.state();
        state.check(TallyCall::GetSession)?;
        state
            .session(tenant_id, election_event_id, tally_session_id)
            .cloned()
    }

    async fn lock_for_update(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        tally_session_id: &str,
    ) -> Result<()> {
        let mut state = self.state();
        state.check(TallyCall::LockSession)?;
        state.session(tenant_id, election_event_id, tally_session_id)?;
        if let Some(hook) = state.on_lock.take() {
            state.update_session(tenant_id, election_event_id, tally_session_id, hook);
        }
        Ok(())
    }

    async fn last_execution(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        tally_session_id: &str,
    ) -> Result<Option<TallySessionExecution>> {
        let state = self.state();
        state.check(TallyCall::LastExecution)?;
        Ok(state.last_execution(tenant_id, election_event_id, tally_session_id))
    }

    async fn last_execution_and_session(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        tally_session_id: &str,
        _election_ids: Vec<String>,
    ) -> Result<Option<(TallySessionExecution, TallySession)>> {
        let state = self.state();
        state.check(TallyCall::LastExecutionAndSession)?;
        let Some(execution) = state.last_execution(tenant_id, election_event_id, tally_session_id)
        else {
            return Ok(None);
        };
        let session = state
            .session(tenant_id, election_event_id, tally_session_id)?
            .clone();
        Ok(Some((execution, session)))
    }

    async fn append_execution(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        tally_session_id: &str,
        current_message_id: i32,
        status: TallyCeremonyStatus,
        run_reason: TallyRunReason,
    ) -> Result<()> {
        let mut state = self.state();
        state.check(TallyCall::AppendExecution)?;
        let execution = TallySessionExecution {
            id: format!("execution-{}", state.executions.len() + 1),
            tenant_id: tenant_id.to_string(),
            election_event_id: election_event_id.to_string(),
            created_at: None,
            last_updated_at: None,
            labels: None,
            annotations: None,
            current_message_id,
            tally_session_id: tally_session_id.to_string(),
            session_ids: None,
            status: Some(serde_json::to_value(status)?),
            results_event_id: None,
            documents: None,
            run_reason: Some(run_reason.to_string()),
        };
        state.executions.push(execution);
        Ok(())
    }

    async fn set_status(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        tally_session_id: &str,
        execution_status: TallyExecutionStatus,
        is_execution_completed: bool,
    ) -> Result<()> {
        let mut state = self.state();
        state.check(TallyCall::SetStatus)?;
        state.update_session(tenant_id, election_event_id, tally_session_id, |session| {
            session.execution_status = Some(execution_status.to_string());
            session.is_execution_completed = is_execution_completed;
        });
        Ok(())
    }

    async fn mark_completed(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        tally_session_id: &str,
        execution_status: TallyExecutionStatus,
    ) -> Result<()> {
        let mut state = self.state();
        state.check(TallyCall::MarkCompleted)?;
        state.update_session(tenant_id, election_event_id, tally_session_id, |session| {
            session.execution_status = Some(execution_status.to_string());
            session.is_execution_completed = true;
        });
        Ok(())
    }
}

impl KeysCeremonyReader for InMemoryTallyCeremony {
    async fn get(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        keys_ceremony_id: &str,
    ) -> Result<KeysCeremony> {
        let state = self.state();
        state.check(TallyCall::GetKeysCeremony)?;
        state
            .keys_ceremonies
            .iter()
            .find(|keys_ceremony| {
                keys_ceremony.tenant_id == tenant_id
                    && keys_ceremony.election_event_id == election_event_id
                    && keys_ceremony.id == keys_ceremony_id
            })
            .cloned()
            .ok_or_else(|| anyhow!("Keys ceremony {keys_ceremony_id} not found"))
    }
}

impl TrusteePrivateKeys for InMemoryTallyCeremony {
    async fn encrypted_private_key(
        &self,
        _tenant_id: &str,
        _election_event_id: &str,
        trustee_name: &str,
        keys_ceremony: &KeysCeremony,
    ) -> Result<String> {
        let state = self.state();
        state.check(TallyCall::GetPrivateKey)?;
        state
            .private_keys
            .get(&(keys_ceremony.id.clone(), trustee_name.to_string()))
            .cloned()
            .ok_or_else(|| anyhow!("can't get trustee public key"))
    }
}

impl ElectionsById for InMemoryTallyCeremony {
    async fn get(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        election_ids: &[String],
    ) -> Result<Vec<Election>> {
        let state = self.state();
        state.check(TallyCall::GetElections)?;
        Ok(state
            .elections
            .iter()
            .filter(|election| {
                election.tenant_id == tenant_id
                    && election.election_event_id == election_event_id
                    && election_ids.contains(&election.id)
            })
            .cloned()
            .collect())
    }
}

impl ElectionEventReader for InMemoryTallyCeremony {
    async fn get(&self, tenant_id: &str, election_event_id: &str) -> Result<ElectionEvent> {
        let state = self.state();
        state.check(TallyCall::GetElectionEvent)?;
        state
            .election_events
            .iter()
            .find(|election_event| {
                election_event.tenant_id == tenant_id && election_event.id == election_event_id
            })
            .cloned()
            .ok_or_else(|| anyhow!("Election event {election_event_id} not found"))
    }
}

impl EnvironmentSlug for InMemoryTallyCeremony {
    fn env_slug(&self) -> Result<String> {
        let state = self.state();
        state.check(TallyCall::EnvSlug)?;
        Ok(state.env_slug.clone())
    }
}

impl TallyCeremonyAudit for InMemoryTallyCeremony {
    async fn tally_opened(
        &self,
        board_name: &str,
        tenant_id: &str,
        election_event_id: &str,
        election_ids: Option<Vec<String>>,
        user_id: &str,
        username: &str,
    ) -> Result<()> {
        self.record(
            TallyCall::TallyOpened,
            TallyAuditEntry::TallyOpened {
                board_name: board_name.to_string(),
                tenant_id: tenant_id.to_string(),
                election_event_id: election_event_id.to_string(),
                election_ids,
                user_id: user_id.to_string(),
                username: username.to_string(),
            },
        )
    }

    async fn key_restored(
        &self,
        board_name: &str,
        tenant_id: &str,
        election_event_id: &str,
        election_ids: Option<Vec<String>>,
        trustee_name: &str,
        claims: &JwtClaims,
    ) -> Result<()> {
        self.record(
            TallyCall::KeyRestored,
            TallyAuditEntry::KeyRestored {
                board_name: board_name.to_string(),
                tenant_id: tenant_id.to_string(),
                election_event_id: election_event_id.to_string(),
                election_ids,
                trustee_name: trustee_name.to_string(),
                user_id: claims.hasura_claims.user_id.clone(),
                username: claims.preferred_username.clone(),
            },
        )
    }

    async fn tally_closed(
        &self,
        board_name: &str,
        tenant_id: &str,
        election_event_id: &str,
        election_ids: Option<Vec<String>>,
        executer: TallyExecuter,
    ) -> Result<()> {
        self.record(
            TallyCall::TallyClosed,
            TallyAuditEntry::TallyClosed {
                board_name: board_name.to_string(),
                tenant_id: tenant_id.to_string(),
                election_event_id: election_event_id.to_string(),
                election_ids,
                executer,
            },
        )
    }
}
