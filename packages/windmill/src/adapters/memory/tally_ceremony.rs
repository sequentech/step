// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::domain::tally_ceremony::TallyExecuter;
use crate::ports::tally_ceremony::{
    DecryptionSet, ElectionEventReader, ElectionsById, EnvironmentSlug, KeysCeremonyReader,
    NewTallySession, TallyCeremonyAudit, TallyCreationReader, TallyEventSnapshot, TallySessions,
    TrusteePrivateKeys,
};
use anyhow::{anyhow, Result};
use b4::messages::newtypes::BatchNumber;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::ceremonies::{TallyCeremonyStatus, TallyExecutionStatus, TallyRunReason};
use sequent_core::types::hasura::core::{
    Area, AreaContest, BallotStyle, Contest, Election, ElectionEvent, KeysCeremony, TallySession,
    TallySessionContest, TallySessionExecution, TallySheet,
};
use sequent_core::types::keycloak::VOTE_WEIGHT_BATCHES;
use sequent_core::types::tally_sheets::TallySheetStatus;
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
    InsertSession,
    NextBatch,
    InsertContest,
    EventSnapshot,
    PublishedBallotStyles,
    ApprovedTallySheets,
    GetKeysCeremony,
    GetPrivateKey,
    GetElections,
    GetElectionEvent,
    EnvSlug,
    TallyOpened,
    KeyRestored,
    KeyInsertionStarted,
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
    KeyInsertionStarted {
        board_name: String,
        tenant_id: String,
        election_event_id: String,
        election_ids: Vec<String>,
        user_id: String,
        username: String,
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
    session_contests: Vec<TallySessionContest>,
    contests: Vec<Contest>,
    areas: Vec<Area>,
    area_contests: Vec<AreaContest>,
    ballot_styles: Vec<BallotStyle>,
    tally_sheets: Vec<TallySheet>,
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

    pub fn add_session_contest(&self, session_contest: TallySessionContest) {
        self.state().session_contests.push(session_contest);
    }

    pub fn add_contest(&self, contest: Contest) {
        self.state().contests.push(contest);
    }

    pub fn add_area(&self, area: Area) {
        self.state().areas.push(area);
    }

    pub fn add_area_contest(&self, area_contest: AreaContest) {
        self.state().area_contests.push(area_contest);
    }

    pub fn add_ballot_style(&self, ballot_style: BallotStyle) {
        self.state().ballot_styles.push(ballot_style);
    }

    pub fn add_tally_sheet(&self, tally_sheet: TallySheet) {
        self.state().tally_sheets.push(tally_sheet);
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

    pub fn sessions(&self) -> Vec<TallySession> {
        self.state().sessions.clone()
    }

    pub fn session_contests(&self, tally_session_id: &str) -> Vec<TallySessionContest> {
        self.state()
            .session_contests
            .iter()
            .filter(|session_contest| session_contest.tally_session_id == tally_session_id)
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

    async fn insert(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        tally_session: NewTallySession,
    ) -> Result<()> {
        let mut state = self.state();
        state.check(TallyCall::InsertSession)?;
        state.sessions.push(TallySession {
            id: tally_session.id,
            tenant_id: tenant_id.to_string(),
            election_event_id: election_event_id.to_string(),
            created_at: None,
            last_updated_at: None,
            labels: None,
            annotations: Some(tally_session.annotations),
            election_ids: Some(tally_session.election_ids),
            area_ids: Some(tally_session.area_ids),
            is_execution_completed: false,
            keys_ceremony_id: tally_session.keys_ceremony_id,
            execution_status: Some(tally_session.execution_status.to_string()),
            threshold: i64::from(tally_session.threshold),
            configuration: tally_session.configuration,
            tally_type: Some(tally_session.tally_type),
            permission_label: Some(tally_session.permission_labels),
        });
        Ok(())
    }

    /// Like `get_tally_session_highest_batch`, skips a whole run of
    /// `VOTE_WEIGHT_BATCHES` after the highest stored batch.
    async fn next_batch(&self, tenant_id: &str, election_event_id: &str) -> Result<BatchNumber> {
        let state = self.state();
        state.check(TallyCall::NextBatch)?;
        Ok(state
            .session_contests
            .iter()
            .filter(|session_contest| {
                session_contest.tenant_id == tenant_id
                    && session_contest.election_event_id == election_event_id
            })
            .map(|session_contest| session_contest.session_id as BatchNumber)
            .max()
            .map_or(0, |highest| {
                highest + BatchNumber::from(VOTE_WEIGHT_BATCHES)
            }))
    }

    async fn insert_contest(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        tally_session_id: &str,
        (election_id, area_id, contest_id): &DecryptionSet,
        batch: BatchNumber,
    ) -> Result<()> {
        let mut state = self.state();
        state.check(TallyCall::InsertContest)?;
        let session_contest = TallySessionContest {
            id: format!("session-contest-{}", state.session_contests.len() + 1),
            tenant_id: tenant_id.to_string(),
            election_event_id: election_event_id.to_string(),
            area_id: area_id.clone(),
            contest_id: contest_id.clone(),
            session_id: batch as i32,
            created_at: None,
            last_updated_at: None,
            labels: None,
            annotations: None,
            tally_session_id: tally_session_id.to_string(),
            election_id: election_id.clone(),
        };
        state.session_contests.push(session_contest);
        Ok(())
    }
}

impl TallyCreationReader for InMemoryTallyCeremony {
    async fn event_snapshot(
        &self,
        tenant_id: &str,
        election_event_id: &str,
    ) -> Result<TallyEventSnapshot> {
        let election_event = ElectionEventReader::get(self, tenant_id, election_event_id).await;
        let state = self.state();
        state.check(TallyCall::EventSnapshot)?;
        Ok(TallyEventSnapshot {
            election_event: election_event?,
            elections: state
                .elections
                .iter()
                .filter(|election| {
                    election.tenant_id == tenant_id
                        && election.election_event_id == election_event_id
                })
                .cloned()
                .collect(),
            contests: state
                .contests
                .iter()
                .filter(|contest| {
                    contest.tenant_id == tenant_id && contest.election_event_id == election_event_id
                })
                .cloned()
                .collect(),
            areas: state
                .areas
                .iter()
                .filter(|area| {
                    area.tenant_id == tenant_id && area.election_event_id == election_event_id
                })
                .cloned()
                .collect(),
            area_contests: state.area_contests.clone(),
        })
    }

    async fn published_ballot_styles(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        election_ids: &[String],
    ) -> Result<Vec<BallotStyle>> {
        let state = self.state();
        state.check(TallyCall::PublishedBallotStyles)?;
        Ok(state
            .ballot_styles
            .iter()
            .filter(|ballot_style| {
                ballot_style.tenant_id == tenant_id
                    && ballot_style.election_event_id == election_event_id
                    && election_ids.contains(&ballot_style.election_id)
                    && ballot_style.deleted_at.is_none()
            })
            .cloned()
            .collect())
    }

    async fn approved_tally_sheets(
        &self,
        tenant_id: &str,
        election_event_id: &str,
    ) -> Result<Vec<TallySheet>> {
        let state = self.state();
        state.check(TallyCall::ApprovedTallySheets)?;
        Ok(state
            .tally_sheets
            .iter()
            .filter(|tally_sheet| {
                tally_sheet.tenant_id == tenant_id
                    && tally_sheet.election_event_id == election_event_id
                    && tally_sheet.reviewed_at.is_some()
                    && tally_sheet.reviewed_by_user_id.is_some()
                    && tally_sheet.status == TallySheetStatus::APPROVED
                    && tally_sheet.deleted_at.is_none()
            })
            .cloned()
            .collect())
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

    async fn key_insertion_started(
        &self,
        board_name: &str,
        tenant_id: &str,
        election_event_id: &str,
        election_ids: Vec<String>,
        user_id: &str,
        username: &str,
    ) -> Result<()> {
        self.record(
            TallyCall::KeyInsertionStarted,
            TallyAuditEntry::KeyInsertionStarted {
                board_name: board_name.to_string(),
                tenant_id: tenant_id.to_string(),
                election_event_id: election_event_id.to_string(),
                election_ids,
                user_id: user_id.to_string(),
                username: username.to_string(),
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
