// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::domain::keys_ceremony::{
    KeygenAuditEntry, KeysBoardMessage, KeysBoardMessages, KeysBoardStep, NewKeysCeremony,
};
use crate::ports::keys_ceremony::{
    KeysBoard, KeysCeremonies, KeysCeremonyAudit, KeysCeremonyElectionEvents,
    KeysCeremonyElections, KeysCeremonyTasks, KeysCeremonyTrustees,
};
use anyhow::{anyhow, Result};
use sequent_core::types::ceremonies::Log;
use sequent_core::types::hasura::core::{Election, ElectionEvent, KeysCeremony, Trustee};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, MutexGuard};

/// A store call that a test can make fail.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StoreCall {
    TrusteesByName,
    KeysCeremonies,
    InsertKeysCeremony,
    UpdateKeysCeremonyStatus,
    ElectionPermissionLabels,
}

/// A status update the store received.
#[derive(Clone, Debug, PartialEq)]
pub struct StatusUpdate {
    pub keys_ceremony_id: String,
    pub status: Value,
    pub execution_status: String,
    /// Whether the ceremony had been locked before the update.
    pub locked: bool,
}

#[derive(Default)]
pub struct KeysCeremonyTables {
    pub keys_ceremonies: Vec<KeysCeremony>,
    pub trustees: Vec<Trustee>,
    pub elections: Vec<Election>,
    pub election_events: Vec<ElectionEvent>,
    pub status_updates: Vec<StatusUpdate>,
    pub locked: HashSet<String>,
    pub failing: HashSet<StoreCall>,
}

/// The keys ceremony tables. There is no transaction: what a use case wrote
/// stays even if it fails afterwards.
#[derive(Default)]
pub struct MemoryKeysCeremonyStore(Mutex<KeysCeremonyTables>);

impl MemoryKeysCeremonyStore {
    pub fn tables(&self) -> MutexGuard<'_, KeysCeremonyTables> {
        self.0.lock().expect("keys ceremony tables lock")
    }

    pub fn fail(&self, call: StoreCall) {
        self.tables().failing.insert(call);
    }

    fn tables_for(&self, call: StoreCall) -> Result<MutexGuard<'_, KeysCeremonyTables>> {
        let tables = self.tables();
        if tables.failing.contains(&call) {
            return Err(anyhow!("{call:?} failed"));
        }
        Ok(tables)
    }
}

fn find_keys_ceremony(
    tables: &KeysCeremonyTables,
    tenant_id: &str,
    election_event_id: &str,
    keys_ceremony_id: &str,
) -> Result<KeysCeremony> {
    tables
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

fn in_scope(
    election: &Election,
    tenant_id: &str,
    election_event_id: &str,
    election_id: Option<&str>,
) -> bool {
    election.tenant_id == tenant_id
        && election.election_event_id == election_event_id
        && election_id.is_none_or(|id| id == election.id)
}

impl KeysCeremonies for MemoryKeysCeremonyStore {
    async fn keys_ceremony(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        keys_ceremony_id: &str,
    ) -> Result<KeysCeremony> {
        find_keys_ceremony(
            &self.tables(),
            tenant_id,
            election_event_id,
            keys_ceremony_id,
        )
    }

    async fn lock_keys_ceremony(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        keys_ceremony_id: &str,
    ) -> Result<KeysCeremony> {
        let mut tables = self.tables();
        let keys_ceremony =
            find_keys_ceremony(&tables, tenant_id, election_event_id, keys_ceremony_id)?;
        tables.locked.insert(keys_ceremony.id.clone());
        Ok(keys_ceremony)
    }

    async fn keys_ceremonies(
        &self,
        tenant_id: &str,
        election_event_id: &str,
    ) -> Result<Vec<KeysCeremony>> {
        let tables = self.tables_for(StoreCall::KeysCeremonies)?;
        Ok(tables
            .keys_ceremonies
            .iter()
            .filter(|keys_ceremony| {
                keys_ceremony.tenant_id == tenant_id
                    && keys_ceremony.election_event_id == election_event_id
            })
            .cloned()
            .collect())
    }

    async fn insert_keys_ceremony(&self, ceremony: NewKeysCeremony) -> Result<()> {
        let mut tables = self.tables_for(StoreCall::InsertKeysCeremony)?;
        tables.keys_ceremonies.push(KeysCeremony {
            id: ceremony.id,
            created_at: None,
            last_updated_at: None,
            tenant_id: ceremony.tenant_id,
            election_event_id: ceremony.election_event_id,
            trustee_ids: ceremony.trustee_ids,
            status: Some(ceremony.status),
            execution_status: Some(ceremony.execution_status),
            labels: None,
            annotations: None,
            threshold: ceremony.threshold.into(),
            name: ceremony.name,
            settings: Some(ceremony.settings),
            is_default: Some(ceremony.is_default),
            permission_label: Some(ceremony.permission_labels),
        });
        Ok(())
    }

    async fn update_keys_ceremony_status(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        keys_ceremony_id: &str,
        status: &Value,
        execution_status: &str,
    ) -> Result<()> {
        let mut guard = self.tables_for(StoreCall::UpdateKeysCeremonyStatus)?;
        let tables = &mut *guard;
        let keys_ceremony = tables
            .keys_ceremonies
            .iter_mut()
            .find(|keys_ceremony| {
                keys_ceremony.tenant_id == tenant_id
                    && keys_ceremony.election_event_id == election_event_id
                    && keys_ceremony.id == keys_ceremony_id
            })
            .ok_or_else(|| anyhow!("No keys ceremony found"))?;
        keys_ceremony.status = Some(status.clone());
        keys_ceremony.execution_status = Some(execution_status.to_string());
        tables.status_updates.push(StatusUpdate {
            keys_ceremony_id: keys_ceremony_id.to_string(),
            status: status.clone(),
            execution_status: execution_status.to_string(),
            locked: tables.locked.contains(keys_ceremony_id),
        });
        Ok(())
    }
}

impl KeysCeremonyTrustees for MemoryKeysCeremonyStore {
    async fn trustees_by_name(&self, tenant_id: &str, names: &[String]) -> Result<Vec<Trustee>> {
        let tables = self.tables_for(StoreCall::TrusteesByName)?;
        Ok(tables
            .trustees
            .iter()
            .filter(|trustee| {
                trustee.tenant_id == tenant_id
                    && trustee
                        .name
                        .as_ref()
                        .is_some_and(|name| names.contains(name))
            })
            .cloned()
            .collect())
    }

    async fn trustees_by_id(&self, tenant_id: &str, ids: &[String]) -> Result<Vec<Trustee>> {
        Ok(self
            .tables()
            .trustees
            .iter()
            .filter(|trustee| trustee.tenant_id == tenant_id && ids.contains(&trustee.id))
            .cloned()
            .collect())
    }

    async fn trustee_by_name(&self, tenant_id: &str, name: &str) -> Result<Trustee> {
        self.tables()
            .trustees
            .iter()
            .find(|trustee| trustee.tenant_id == tenant_id && trustee.name.as_deref() == Some(name))
            .cloned()
            .ok_or_else(|| anyhow!("Trustee {name} not found"))
    }
}

impl KeysCeremonyElections for MemoryKeysCeremonyStore {
    async fn election(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        election_id: &str,
    ) -> Result<Option<Election>> {
        Ok(self
            .tables()
            .elections
            .iter()
            .find(|election| in_scope(election, tenant_id, election_event_id, Some(election_id)))
            .cloned())
    }

    async fn assign_keys_ceremony(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        election_id: Option<String>,
        keys_ceremony_id: &str,
    ) -> Result<Vec<Election>> {
        let mut tables = self.tables();
        let assigned: Vec<Election> = tables
            .elections
            .iter_mut()
            .filter(|election| {
                in_scope(
                    election,
                    tenant_id,
                    election_event_id,
                    election_id.as_deref(),
                )
            })
            .map(|election| {
                election.keys_ceremony_id = Some(keys_ceremony_id.to_string());
                election.clone()
            })
            .collect();
        if assigned.is_empty() {
            return Err(anyhow!("No election found"));
        }
        Ok(assigned)
    }

    async fn first_election_of_keys_ceremony(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        keys_ceremony_id: &str,
    ) -> Result<Option<Election>> {
        Ok(self
            .tables()
            .elections
            .iter()
            .find(|election| {
                in_scope(election, tenant_id, election_event_id, None)
                    && election.keys_ceremony_id.as_deref() == Some(keys_ceremony_id)
            })
            .cloned())
    }

    async fn election_permission_labels(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        election_id: Option<String>,
    ) -> Result<Vec<String>> {
        let tables = self.tables_for(StoreCall::ElectionPermissionLabels)?;
        let elections: Vec<&Election> = tables
            .elections
            .iter()
            .filter(|election| {
                in_scope(
                    election,
                    tenant_id,
                    election_event_id,
                    election_id.as_deref(),
                )
            })
            .collect();
        if elections.is_empty() {
            return Err(anyhow!("No election found"));
        }
        Ok(elections
            .into_iter()
            .filter_map(|election| election.permission_label.clone())
            .collect())
    }
}

impl KeysCeremonyElectionEvents for MemoryKeysCeremonyStore {
    async fn election_event(
        &self,
        tenant_id: &str,
        election_event_id: &str,
    ) -> Result<ElectionEvent> {
        self.tables()
            .election_events
            .iter()
            .find(|election_event| {
                election_event.tenant_id == tenant_id && election_event.id == election_event_id
            })
            .cloned()
            .ok_or_else(|| anyhow!("Election event {election_event_id} not found"))
    }
}

/// A board call that a test can make fail.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BoardCall {
    CreateKeys,
}

/// A ceremony configuration posted to a board.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoardConfiguration {
    pub board_name: String,
    pub tenant_id: String,
    pub election_event_id: String,
    pub trustee_public_keys: Vec<String>,
    pub threshold: usize,
}

/// A key generation message, its log entry and when it was posted, in
/// seconds since the Unix epoch.
#[derive(Clone, Debug)]
pub struct PostedMessage {
    pub message: KeysBoardMessage<String>,
    pub log: Log,
    pub posted_at: u64,
}

#[derive(Default)]
pub struct KeysBoards {
    pub configurations: Vec<BoardConfiguration>,
    pub public_keys: HashMap<String, String>,
    pub messages: HashMap<String, Vec<PostedMessage>>,
    /// Encrypted private keys by board and trustee public key.
    pub private_keys: HashMap<(String, String), String>,
    /// Trustee public keys the board can't read.
    pub invalid_keys: HashSet<String>,
    pub failing: HashSet<BoardCall>,
}

/// Bulletin boards whose senders are identified by their public key string.
#[derive(Default)]
pub struct MemoryKeysBoard(Mutex<KeysBoards>);

impl MemoryKeysBoard {
    pub fn boards(&self) -> MutexGuard<'_, KeysBoards> {
        self.0.lock().expect("keys boards lock")
    }

    pub fn election_board_name(election_id: &str) -> String {
        format!("election-board-{election_id}")
    }
}

impl KeysBoard for MemoryKeysBoard {
    type Sender = String;

    fn election_board(&self, _tenant_id: &str, election_id: &str) -> Result<String> {
        Ok(Self::election_board_name(election_id))
    }

    async fn configuration_exists(&self, board_name: &str) -> Result<bool> {
        Ok(self
            .boards()
            .configurations
            .iter()
            .any(|configuration| configuration.board_name == board_name))
    }

    async fn create_keys(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        board_name: &str,
        trustee_public_keys: Vec<String>,
        threshold: usize,
    ) -> Result<()> {
        let mut boards = self.boards();
        if boards.failing.contains(&BoardCall::CreateKeys) {
            return Err(anyhow!("CreateKeys failed"));
        }
        boards.configurations.push(BoardConfiguration {
            board_name: board_name.to_string(),
            tenant_id: tenant_id.to_string(),
            election_event_id: election_event_id.to_string(),
            trustee_public_keys,
            threshold,
        });
        Ok(())
    }

    async fn public_key(&self, board_name: &str) -> Result<String> {
        self.boards()
            .public_keys
            .get(board_name)
            .cloned()
            .ok_or_else(|| anyhow!("Public Key not found on board {board_name}"))
    }

    async fn public_key_messages(
        &self,
        board_name: &str,
        logs_since: u64,
    ) -> Result<KeysBoardMessages<String>> {
        let posted = self
            .boards()
            .messages
            .get(board_name)
            .cloned()
            .unwrap_or_default();
        Ok(KeysBoardMessages {
            logs: posted
                .iter()
                .filter(|posted| posted.posted_at >= logs_since)
                .map(|posted| posted.log.clone())
                .collect(),
            messages: posted.into_iter().map(|posted| posted.message).collect(),
        })
    }

    fn trustee_sender(&self, trustee_public_key: &str) -> Result<String> {
        if self.boards().invalid_keys.contains(trustee_public_key) {
            return Err(anyhow!("invalid public key {trustee_public_key}"));
        }
        Ok(trustee_public_key.to_string())
    }

    async fn trustee_encrypted_private_key(
        &self,
        board_name: &str,
        trustee_public_key: &str,
    ) -> Result<String> {
        self.boards()
            .private_keys
            .get(&(board_name.to_string(), trustee_public_key.to_string()))
            .cloned()
            .ok_or_else(|| anyhow!("Channel not found on board {board_name}"))
    }
}

/// The keygen entries posted to the electoral log.
#[derive(Default)]
pub struct MemoryKeysCeremonyAudit {
    entries: Mutex<Vec<KeygenAuditEntry>>,
    failing: Mutex<bool>,
}

impl MemoryKeysCeremonyAudit {
    pub fn entries(&self) -> Vec<KeygenAuditEntry> {
        self.entries.lock().expect("audit entries lock").clone()
    }

    pub fn fail(&self) {
        *self.failing.lock().expect("audit failure lock") = true;
    }
}

impl KeysCeremonyAudit for MemoryKeysCeremonyAudit {
    async fn keygen(&self, entry: KeygenAuditEntry) -> Result<()> {
        if *self.failing.lock().expect("audit failure lock") {
            return Err(anyhow!("keygen failed"));
        }
        self.entries.lock().expect("audit entries lock").push(entry);
        Ok(())
    }
}

/// A board task that was queued.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueuedTask {
    pub step: KeysBoardStep,
    pub tenant_id: String,
    pub election_event_id: String,
    pub keys_ceremony_id: String,
}

/// A task queue that numbers the tasks it accepts.
#[derive(Default)]
pub struct MemoryKeysCeremonyTasks {
    queued: Mutex<Vec<QueuedTask>>,
    failing: Mutex<Option<KeysBoardStep>>,
}

impl MemoryKeysCeremonyTasks {
    pub fn queued(&self) -> Vec<QueuedTask> {
        self.queued.lock().expect("queued tasks lock").clone()
    }

    pub fn fail(&self, step: KeysBoardStep) {
        *self.failing.lock().expect("task failure lock") = Some(step);
    }

    fn queue(
        &self,
        step: KeysBoardStep,
        tenant_id: &str,
        election_event_id: &str,
        keys_ceremony_id: &str,
    ) -> Result<String> {
        if *self.failing.lock().expect("task failure lock") == Some(step) {
            return Err(anyhow!("{step:?} task failed"));
        }
        let mut queued = self.queued.lock().expect("queued tasks lock");
        queued.push(QueuedTask {
            step,
            tenant_id: tenant_id.to_string(),
            election_event_id: election_event_id.to_string(),
            keys_ceremony_id: keys_ceremony_id.to_string(),
        });
        Ok(format!("task-{}", queued.len()))
    }
}

impl KeysCeremonyTasks for MemoryKeysCeremonyTasks {
    async fn send_create_keys(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        keys_ceremony_id: &str,
    ) -> Result<String> {
        self.queue(
            KeysBoardStep::CreateKeys,
            tenant_id,
            election_event_id,
            keys_ceremony_id,
        )
    }

    async fn send_set_public_key(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        keys_ceremony_id: &str,
    ) -> Result<String> {
        self.queue(
            KeysBoardStep::SetPublicKey,
            tenant_id,
            election_event_id,
            keys_ceremony_id,
        )
    }
}
