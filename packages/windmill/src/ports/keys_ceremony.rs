// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! What the keys ceremony needs from the database, the bulletin board, the
//! electoral log and the task queue. The database traits use distinct method
//! names so that one store can implement all of them.

use crate::domain::keys_ceremony::{KeygenAuditEntry, NewKeysCeremony};
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

    /// Store the native signed configuration and manager before publishing.
    fn persist_dkg(
        &self,
        tenant_id: &str,
        event_id: &str,
        ceremony_id: &str,
        dkg: &protocol_board::DkgBoard,
        manager: &protocol_board::BoardManager,
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
