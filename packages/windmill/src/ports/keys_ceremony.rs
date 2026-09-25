// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! What the keys ceremony needs from the database, the bulletin board, the
//! electoral log and the task queue. The database traits use distinct method
//! names so that one store can implement all of them.

use crate::domain::keys_ceremony::{KeygenAuditEntry, KeysBoardMessages, NewKeysCeremony};
use anyhow::Result;
use sequent_core::types::hasura::core::{Election, ElectionEvent, KeysCeremony, Trustee};
use serde_json::Value;
use std::future::Future;

/// Stored keys ceremonies.
pub trait KeysCeremonies: Sync {
    /// Fails when the ceremony doesn't exist.
    fn keys_ceremony(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        keys_ceremony_id: &str,
    ) -> impl Future<Output = Result<KeysCeremony>> + Send;

    /// Reads the ceremony and holds it until the transaction ends, so that a
    /// status derived from it can't overwrite a concurrent update.
    fn lock_keys_ceremony(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        keys_ceremony_id: &str,
    ) -> impl Future<Output = Result<KeysCeremony>> + Send;

    fn keys_ceremonies(
        &self,
        tenant_id: &str,
        election_event_id: &str,
    ) -> impl Future<Output = Result<Vec<KeysCeremony>>> + Send;

    fn insert_keys_ceremony(
        &self,
        keys_ceremony: NewKeysCeremony,
    ) -> impl Future<Output = Result<()>> + Send;

    /// Fails when the ceremony doesn't exist.
    fn update_keys_ceremony_status(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        keys_ceremony_id: &str,
        status: &Value,
        execution_status: &str,
    ) -> impl Future<Output = Result<()>> + Send;
}

/// Stored trustees of a tenant.
pub trait KeysCeremonyTrustees: Sync {
    fn trustees_by_name(
        &self,
        tenant_id: &str,
        names: &[String],
    ) -> impl Future<Output = Result<Vec<Trustee>>> + Send;

    fn trustees_by_id(
        &self,
        tenant_id: &str,
        ids: &[String],
    ) -> impl Future<Output = Result<Vec<Trustee>>> + Send;

    /// Fails when no trustee has the name.
    fn trustee_by_name(
        &self,
        tenant_id: &str,
        name: &str,
    ) -> impl Future<Output = Result<Trustee>> + Send;
}

/// Stored elections, as far as their keys ceremony is concerned.
pub trait KeysCeremonyElections: Sync {
    fn election(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        election_id: &str,
    ) -> impl Future<Output = Result<Option<Election>>> + Send;

    /// Assigns the ceremony to the election, or to every election of the
    /// event when `election_id` is `None`, and returns the updated elections.
    /// Fails when no election matches.
    fn assign_keys_ceremony(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        election_id: Option<String>,
        keys_ceremony_id: &str,
    ) -> impl Future<Output = Result<Vec<Election>>> + Send;

    fn first_election_of_keys_ceremony(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        keys_ceremony_id: &str,
    ) -> impl Future<Output = Result<Option<Election>>> + Send;

    /// The permission labels of the election, or of every election of the
    /// event when `election_id` is `None`. Fails when no election matches.
    fn election_permission_labels(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        election_id: Option<String>,
    ) -> impl Future<Output = Result<Vec<String>>> + Send;
}

/// Stored election events.
pub trait KeysCeremonyElectionEvents: Sync {
    /// Fails when the election event doesn't exist.
    fn election_event(
        &self,
        tenant_id: &str,
        election_event_id: &str,
    ) -> impl Future<Output = Result<ElectionEvent>> + Send;
}

/// The bulletin boards where the trustees generate the election keys.
pub trait KeysBoard: Sync {
    /// How the board identifies the sender of a message.
    type Sender: PartialEq + Send + Sync;

    /// The board of the keys ceremony of a single election.
    fn election_board(&self, tenant_id: &str, election_id: &str) -> Result<String>;

    fn configuration_exists(&self, board_name: &str) -> impl Future<Output = Result<bool>> + Send;

    /// Posts the ceremony configuration, which starts the key generation.
    fn create_keys(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        board_name: &str,
        trustee_public_keys: Vec<String>,
        threshold: usize,
    ) -> impl Future<Output = Result<()>> + Send;

    /// Fails until every trustee has posted its share of the public key.
    fn public_key(&self, board_name: &str) -> impl Future<Output = Result<String>> + Send;

    /// The key generation messages, with log entries for those posted since
    /// `logs_since`, in seconds since the Unix epoch.
    fn public_key_messages(
        &self,
        board_name: &str,
        logs_since: u64,
    ) -> impl Future<Output = Result<KeysBoardMessages<Self::Sender>>> + Send;

    /// The sender identity of a trustee, from its stored public key.
    fn trustee_sender(&self, trustee_public_key: &str) -> Result<Self::Sender>;

    fn trustee_encrypted_private_key(
        &self,
        board_name: &str,
        trustee_public_key: &str,
    ) -> impl Future<Output = Result<String>> + Send;
}

/// The electoral log.
pub trait KeysCeremonyAudit: Sync {
    fn keygen(&self, entry: KeygenAuditEntry) -> impl Future<Output = Result<()>> + Send;
}

/// The tasks that advance ceremonies on the board. Both return the task id.
pub trait KeysCeremonyTasks: Sync {
    fn send_create_keys(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        keys_ceremony_id: &str,
    ) -> impl Future<Output = Result<String>> + Send;

    fn send_set_public_key(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        keys_ceremony_id: &str,
    ) -> impl Future<Output = Result<String>> + Send;
}
