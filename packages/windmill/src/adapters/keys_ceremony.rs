// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::domain::keys_ceremony::{
    KeygenAuditEntry, KeysBoardMessage, KeysBoardMessages, KeysBoardStatement, NewKeysCeremony,
};
use crate::ports::keys_ceremony::{
    KeysBoard, KeysCeremonies, KeysCeremonyAudit, KeysCeremonyElectionEvents,
    KeysCeremonyElections, KeysCeremonyTasks, KeysCeremonyTrustees,
};
use crate::postgres::{election, election_event, keys_ceremony, trustee};
use crate::services::ceremonies::serialize_logs::generate_logs;
use crate::services::electoral_log::ElectoralLog;
use crate::services::{private_keys, protocol_manager, public_keys};
use crate::tasks::create_keys::create_keys;
use crate::tasks::set_public_key::set_public_key;
use crate::types::error::Error;
use anyhow::{Context, Result};
use b4::messages::statement::StatementType;
use celery::Celery;
use deadpool_postgres::Transaction;
use sequent_core::types::hasura::core::{Election, ElectionEvent, KeysCeremony, Trustee};
use serde_json::Value;
use strand::signature::StrandSignaturePk;

/// Keys ceremonies, trustees, elections and election events in the Hasura
/// database.
pub struct PgKeysCeremonyStore<'a> {
    pub transaction: &'a Transaction<'a>,
}

impl KeysCeremonies for PgKeysCeremonyStore<'_> {
    async fn keys_ceremony(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        keys_ceremony_id: &str,
    ) -> Result<KeysCeremony> {
        keys_ceremony::get_keys_ceremony_by_id(
            self.transaction,
            tenant_id,
            election_event_id,
            keys_ceremony_id,
        )
        .await
    }

    async fn lock_keys_ceremony(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        keys_ceremony_id: &str,
    ) -> Result<KeysCeremony> {
        keys_ceremony::lock_keys_ceremony_by_id(
            self.transaction,
            tenant_id,
            election_event_id,
            keys_ceremony_id,
        )
        .await
    }

    async fn keys_ceremonies(
        &self,
        tenant_id: &str,
        election_event_id: &str,
    ) -> Result<Vec<KeysCeremony>> {
        keys_ceremony::get_keys_ceremonies(self.transaction, tenant_id, election_event_id).await
    }

    async fn insert_keys_ceremony(&self, ceremony: NewKeysCeremony) -> Result<()> {
        keys_ceremony::insert_keys_ceremony(
            self.transaction,
            ceremony.id,
            ceremony.tenant_id,
            ceremony.election_event_id,
            ceremony.trustee_ids,
            ceremony.threshold,
            Some(ceremony.status),
            Some(ceremony.execution_status),
            ceremony.name,
            Some(ceremony.settings),
            ceremony.is_default,
            ceremony.permission_labels,
        )
        .await
        .map(|_| ())
    }

    async fn update_keys_ceremony_status(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        keys_ceremony_id: &str,
        status: &Value,
        execution_status: &str,
    ) -> Result<()> {
        keys_ceremony::update_keys_ceremony_status(
            self.transaction,
            tenant_id,
            election_event_id,
            keys_ceremony_id,
            status,
            execution_status,
        )
        .await
    }
}

impl KeysCeremonyTrustees for PgKeysCeremonyStore<'_> {
    async fn trustees_by_name(&self, tenant_id: &str, names: &[String]) -> Result<Vec<Trustee>> {
        trustee::get_trustees_by_name(self.transaction, tenant_id, &names.to_vec()).await
    }

    async fn trustees_by_id(&self, tenant_id: &str, ids: &[String]) -> Result<Vec<Trustee>> {
        trustee::get_trustees_by_id(self.transaction, tenant_id, &ids.to_vec()).await
    }

    async fn trustee_by_name(&self, tenant_id: &str, name: &str) -> Result<Trustee> {
        trustee::get_trustee_by_name(self.transaction, tenant_id, name).await
    }
}

impl KeysCeremonyElections for PgKeysCeremonyStore<'_> {
    async fn election(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        election_id: &str,
    ) -> Result<Option<Election>> {
        election::get_election_by_id(self.transaction, tenant_id, election_event_id, election_id)
            .await
    }

    async fn assign_keys_ceremony(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        election_id: Option<String>,
        keys_ceremony_id: &str,
    ) -> Result<Vec<Election>> {
        election::set_election_keys_ceremony(
            self.transaction,
            tenant_id,
            election_event_id,
            election_id,
            keys_ceremony_id,
        )
        .await
    }

    async fn first_election_of_keys_ceremony(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        keys_ceremony_id: &str,
    ) -> Result<Option<Election>> {
        Ok(election::get_elections_by_keys_ceremony_id(
            self.transaction,
            tenant_id,
            election_event_id,
            keys_ceremony_id,
        )
        .await?
        .into_iter()
        .next())
    }

    async fn election_permission_labels(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        election_id: Option<String>,
    ) -> Result<Vec<String>> {
        election::get_election_permission_label(
            self.transaction,
            tenant_id,
            election_event_id,
            election_id,
        )
        .await
    }
}

impl KeysCeremonyElectionEvents for PgKeysCeremonyStore<'_> {
    async fn election_event(
        &self,
        tenant_id: &str,
        election_event_id: &str,
    ) -> Result<ElectionEvent> {
        election_event::get_election_event_by_id(self.transaction, tenant_id, election_event_id)
            .await
    }
}

/// The B4 bulletin boards. Posting a configuration reads the protocol
/// manager keys through the transaction.
pub struct B4KeysBoard<'a> {
    pub transaction: &'a Transaction<'a>,
}

impl KeysBoard for B4KeysBoard<'_> {
    type Sender = StrandSignaturePk;

    fn election_board(&self, tenant_id: &str, election_id: &str) -> Result<String> {
        let slug = std::env::var("ENV_SLUG").with_context(|| "missing env var ENV_SLUG")?;
        Ok(protocol_manager::get_election_board(
            tenant_id,
            election_id,
            &slug,
        ))
    }

    async fn configuration_exists(&self, board_name: &str) -> Result<bool> {
        protocol_manager::check_configuration_exists(board_name).await
    }

    async fn create_keys(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        board_name: &str,
        trustee_public_keys: Vec<String>,
        threshold: usize,
    ) -> Result<()> {
        public_keys::create_keys(
            self.transaction,
            tenant_id,
            election_event_id,
            board_name,
            trustee_public_keys,
            threshold,
        )
        .await
    }

    async fn public_key(&self, board_name: &str) -> Result<String> {
        public_keys::get_public_key(board_name.to_string()).await
    }

    async fn public_key_messages(
        &self,
        board_name: &str,
        logs_since: u64,
    ) -> Result<KeysBoardMessages<StrandSignaturePk>> {
        let messages = protocol_manager::get_board_public_key_messages(board_name).await?;
        let logs = generate_logs(&messages, logs_since, &vec![0])?;
        Ok(KeysBoardMessages {
            logs,
            messages: messages
                .into_iter()
                .map(|message| KeysBoardMessage {
                    statement: match message.statement.get_kind() {
                        StatementType::PublicKey => KeysBoardStatement::PublicKey,
                        StatementType::PublicKeySigned => KeysBoardStatement::PublicKeySigned,
                        _ => KeysBoardStatement::Other,
                    },
                    sender: message.sender.pk,
                })
                .collect(),
        })
    }

    fn trustee_sender(&self, trustee_public_key: &str) -> Result<StrandSignaturePk> {
        StrandSignaturePk::from_der_b64_string(trustee_public_key)
            .map_err(|err| Error::from(err).into())
    }

    async fn trustee_encrypted_private_key(
        &self,
        board_name: &str,
        trustee_public_key: &str,
    ) -> Result<String> {
        private_keys::get_trustee_encrypted_private_key(board_name, trustee_public_key).await
    }
}

/// The electoral log of the election event, signed by the admin user.
pub struct ElectoralLogKeysCeremonyAudit<'a> {
    pub transaction: &'a Transaction<'a>,
}

impl KeysCeremonyAudit for ElectoralLogKeysCeremonyAudit<'_> {
    async fn keygen(&self, entry: KeygenAuditEntry) -> Result<()> {
        let election_ids = entry.election_id.clone().map(|id| vec![id]);
        let electoral_log = ElectoralLog::for_admin_user(
            self.transaction,
            &entry.board_name,
            &entry.tenant_id,
            &entry.stored_election_event_id,
            &entry.user_id,
            Some(entry.username.clone()),
            election_ids,
            None,
        )
        .await?;
        electoral_log
            .post_keygen(
                entry.election_event_id,
                Some(entry.user_id),
                Some(entry.username),
                entry.election_id,
            )
            .await
            .with_context(|| "error posting to the electoral log")
    }
}

/// The Celery queue.
pub struct CeleryKeysCeremonyTasks<'a> {
    pub celery_app: &'a Celery,
}

impl KeysCeremonyTasks for CeleryKeysCeremonyTasks<'_> {
    async fn send_create_keys(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        keys_ceremony_id: &str,
    ) -> Result<String> {
        let task = self
            .celery_app
            .send_task(create_keys::new(
                tenant_id.to_string(),
                election_event_id.to_string(),
                keys_ceremony_id.to_string(),
            ))
            .await?;
        Ok(task.task_id)
    }

    async fn send_set_public_key(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        keys_ceremony_id: &str,
    ) -> Result<String> {
        let task = self
            .celery_app
            .send_task(set_public_key::new(
                tenant_id.to_string(),
                election_event_id.to_string(),
                keys_ceremony_id.to_string(),
            ))
            .await?;
        Ok(task.task_id)
    }
}
