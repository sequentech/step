// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::domain::tally_ceremony::TallyExecuter;
use crate::ports::tally_ceremony::{
    DecryptionSet, ElectionEventReader, ElectionsById, EnvironmentSlug, KeysCeremonyReader,
    NewTallySession, TallyCeremonyAudit, TallyCreationReader, TallyEventSnapshot, TallySessions,
    TrusteePrivateKeys,
};
use crate::postgres::area::get_event_areas;
use crate::postgres::area_contest::export_area_contests;
use crate::postgres::ballot_style::get_ballot_styles_by_elections;
use crate::postgres::contest::export_contests;
use crate::postgres::election::{export_elections, get_elections_by_ids};
use crate::postgres::election_event::get_election_event_by_id;
use crate::postgres::keys_ceremony::get_keys_ceremony_by_id;
use crate::postgres::tally_session::{
    get_tally_session_by_id, insert_tally_session, lock_tally_session_for_update,
    set_tally_session_completed, update_tally_session_status,
};
use crate::postgres::tally_session_contest::{
    get_tally_session_highest_batch, insert_tally_session_contest,
};
use crate::postgres::tally_session_execution::{
    get_last_tally_session_execution, insert_tally_session_execution,
};
use crate::postgres::tally_sheet::get_approved_tally_sheets_by_event;
use crate::services::ceremonies::keys_ceremony::find_trustee_private_key;
use crate::services::ceremonies::tally_ceremony::find_last_tally_session_execution_and_all_related_data;
use crate::services::electoral_log::ElectoralLog;
use anyhow::{Context, Result};
use b4::messages::newtypes::BatchNumber;
use deadpool_postgres::Transaction;
use futures::try_join;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::ceremonies::{TallyCeremonyStatus, TallyExecutionStatus, TallyRunReason};
use sequent_core::types::hasura::core::{
    BallotStyle, Election, ElectionEvent, KeysCeremony, TallySession, TallySessionExecution,
    TallySheet,
};

const ELECTORAL_LOG_POST_FAILED: &str = "error posting to the electoral log";

pub struct PgTallySessions<'a> {
    transaction: &'a Transaction<'a>,
}

impl<'a> PgTallySessions<'a> {
    pub fn new(transaction: &'a Transaction<'a>) -> Self {
        Self { transaction }
    }
}

impl TallySessions for PgTallySessions<'_> {
    async fn get(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        tally_session_id: &str,
    ) -> Result<TallySession> {
        get_tally_session_by_id(
            self.transaction,
            tenant_id,
            election_event_id,
            tally_session_id,
        )
        .await
    }

    async fn lock_for_update(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        tally_session_id: &str,
    ) -> Result<()> {
        lock_tally_session_for_update(
            self.transaction,
            tenant_id,
            election_event_id,
            tally_session_id,
        )
        .await
    }

    async fn last_execution(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        tally_session_id: &str,
    ) -> Result<Option<TallySessionExecution>> {
        get_last_tally_session_execution(
            self.transaction,
            tenant_id,
            election_event_id,
            tally_session_id,
        )
        .await
    }

    async fn last_execution_and_session(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        tally_session_id: &str,
        election_ids: Vec<String>,
    ) -> Result<Option<(TallySessionExecution, TallySession)>> {
        Ok(find_last_tally_session_execution_and_all_related_data(
            self.transaction,
            tenant_id.to_string(),
            election_event_id.to_string(),
            tally_session_id.to_string(),
            election_ids,
        )
        .await?
        .map(|(execution, tally_session, _, _)| (execution, tally_session)))
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
        insert_tally_session_execution(
            self.transaction,
            tenant_id,
            election_event_id,
            current_message_id,
            tally_session_id,
            Some(status),
            None,
            None,
            None,
            run_reason,
        )
        .await?;
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
        update_tally_session_status(
            self.transaction,
            tenant_id,
            election_event_id,
            tally_session_id,
            execution_status,
            is_execution_completed,
        )
        .await
    }

    async fn mark_completed(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        tally_session_id: &str,
        execution_status: TallyExecutionStatus,
    ) -> Result<()> {
        set_tally_session_completed(
            self.transaction,
            tenant_id,
            election_event_id,
            tally_session_id,
            execution_status,
        )
        .await
    }

    async fn insert(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        tally_session: NewTallySession,
    ) -> Result<()> {
        insert_tally_session(
            self.transaction,
            tenant_id,
            election_event_id,
            tally_session.election_ids,
            tally_session.area_ids,
            &tally_session.id,
            &tally_session.keys_ceremony_id,
            tally_session.execution_status,
            tally_session.threshold,
            tally_session.configuration,
            &tally_session.tally_type,
            tally_session.annotations,
            tally_session.permission_labels,
        )
        .await?;
        Ok(())
    }

    async fn next_batch(&self, tenant_id: &str, election_event_id: &str) -> Result<BatchNumber> {
        get_tally_session_highest_batch(self.transaction, tenant_id, election_event_id).await
    }

    async fn insert_contest(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        tally_session_id: &str,
        (election_id, area_id, contest_id): &DecryptionSet,
        batch: BatchNumber,
    ) -> Result<()> {
        insert_tally_session_contest(
            self.transaction,
            tenant_id,
            election_event_id,
            area_id,
            contest_id.clone(),
            batch,
            tally_session_id,
            election_id,
        )
        .await?;
        Ok(())
    }
}

pub struct PgTallyCreationReader<'a> {
    transaction: &'a Transaction<'a>,
}

impl<'a> PgTallyCreationReader<'a> {
    pub fn new(transaction: &'a Transaction<'a>) -> Self {
        Self { transaction }
    }
}

impl TallyCreationReader for PgTallyCreationReader<'_> {
    async fn event_snapshot(
        &self,
        tenant_id: &str,
        election_event_id: &str,
    ) -> Result<TallyEventSnapshot> {
        let (election_event, elections, contests, areas, area_contests) = try_join!(
            get_election_event_by_id(self.transaction, tenant_id, election_event_id),
            export_elections(self.transaction, tenant_id, election_event_id),
            export_contests(self.transaction, tenant_id, election_event_id),
            get_event_areas(self.transaction, tenant_id, election_event_id),
            export_area_contests(self.transaction, tenant_id, election_event_id),
        )?;
        Ok(TallyEventSnapshot {
            election_event,
            elections,
            contests,
            areas,
            area_contests,
        })
    }

    async fn published_ballot_styles(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        election_ids: &[String],
    ) -> Result<Vec<BallotStyle>> {
        get_ballot_styles_by_elections(
            self.transaction,
            tenant_id,
            election_event_id,
            &election_ids.to_vec(),
        )
        .await
    }

    async fn approved_tally_sheets(
        &self,
        tenant_id: &str,
        election_event_id: &str,
    ) -> Result<Vec<TallySheet>> {
        get_approved_tally_sheets_by_event(self.transaction, tenant_id, election_event_id).await
    }
}

pub struct PgKeysCeremonies<'a> {
    transaction: &'a Transaction<'a>,
}

impl<'a> PgKeysCeremonies<'a> {
    pub fn new(transaction: &'a Transaction<'a>) -> Self {
        Self { transaction }
    }
}

impl KeysCeremonyReader for PgKeysCeremonies<'_> {
    async fn get(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        keys_ceremony_id: &str,
    ) -> Result<KeysCeremony> {
        get_keys_ceremony_by_id(
            self.transaction,
            tenant_id,
            election_event_id,
            keys_ceremony_id,
        )
        .await
    }
}

/// Reads a trustee's encrypted private key from the keys ceremony board.
pub struct BoardTrusteePrivateKeys<'a> {
    transaction: &'a Transaction<'a>,
}

impl<'a> BoardTrusteePrivateKeys<'a> {
    pub fn new(transaction: &'a Transaction<'a>) -> Self {
        Self { transaction }
    }
}

impl TrusteePrivateKeys for BoardTrusteePrivateKeys<'_> {
    async fn encrypted_private_key(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        trustee_name: &str,
        keys_ceremony: &KeysCeremony,
    ) -> Result<String> {
        find_trustee_private_key(
            self.transaction,
            tenant_id,
            election_event_id,
            trustee_name,
            keys_ceremony,
        )
        .await
    }
}

pub struct PgElectionsById<'a> {
    transaction: &'a Transaction<'a>,
}

impl<'a> PgElectionsById<'a> {
    pub fn new(transaction: &'a Transaction<'a>) -> Self {
        Self { transaction }
    }
}

impl ElectionsById for PgElectionsById<'_> {
    async fn get(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        election_ids: &[String],
    ) -> Result<Vec<Election>> {
        get_elections_by_ids(
            self.transaction,
            tenant_id,
            election_event_id,
            &election_ids.to_vec(),
        )
        .await
    }
}

pub struct PgElectionEvents<'a> {
    transaction: &'a Transaction<'a>,
}

impl<'a> PgElectionEvents<'a> {
    pub fn new(transaction: &'a Transaction<'a>) -> Self {
        Self { transaction }
    }
}

impl ElectionEventReader for PgElectionEvents<'_> {
    async fn get(&self, tenant_id: &str, election_event_id: &str) -> Result<ElectionEvent> {
        get_election_event_by_id(self.transaction, tenant_id, election_event_id).await
    }
}

/// Reads `ENV_SLUG` from the process environment on every call.
pub struct EnvSlug;

impl EnvironmentSlug for EnvSlug {
    fn env_slug(&self) -> Result<String> {
        std::env::var("ENV_SLUG").with_context(|| "missing env var ENV_SLUG")
    }
}

pub struct ElectoralLogTallyAudit<'a> {
    transaction: &'a Transaction<'a>,
}

impl<'a> ElectoralLogTallyAudit<'a> {
    pub fn new(transaction: &'a Transaction<'a>) -> Self {
        Self { transaction }
    }
}

impl TallyCeremonyAudit for ElectoralLogTallyAudit<'_> {
    async fn tally_opened(
        &self,
        board_name: &str,
        tenant_id: &str,
        election_event_id: &str,
        election_ids: Option<Vec<String>>,
        user_id: &str,
        username: &str,
    ) -> Result<()> {
        let electoral_log = ElectoralLog::for_admin_user(
            self.transaction,
            board_name,
            tenant_id,
            election_event_id,
            user_id,
            Some(username.to_string()),
            election_ids.clone(),
            None,
        )
        .await?;
        electoral_log
            .post_tally_open(
                election_event_id.to_string(),
                election_ids,
                Some(user_id.to_string()),
                Some(username.to_string()),
            )
            .await
            .with_context(|| ELECTORAL_LOG_POST_FAILED)
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
        let user_id = &claims.hasura_claims.user_id;
        let username = &claims.preferred_username;
        let electoral_log = ElectoralLog::for_admin_user(
            self.transaction,
            board_name,
            tenant_id,
            election_event_id,
            user_id,
            username.clone(),
            election_ids.clone(),
            None,
        )
        .await?;
        electoral_log
            .post_key_insertion(
                election_event_id.to_string(),
                trustee_name.to_string(),
                Some(user_id.to_string()),
                username.clone(),
                election_ids,
            )
            .await
            .with_context(|| ELECTORAL_LOG_POST_FAILED)
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
        let electoral_log = ElectoralLog::for_admin_user(
            self.transaction,
            board_name,
            tenant_id,
            election_event_id,
            user_id,
            Some(username.to_string()),
            Some(election_ids.clone()),
            None,
        )
        .await?;
        electoral_log
            .post_key_insertion_start(
                election_event_id.to_string(),
                Some(user_id.to_string()),
                Some(username.to_string()),
                Some(election_ids),
            )
            .await
            .with_context(|| ELECTORAL_LOG_POST_FAILED)
    }

    async fn tally_closed(
        &self,
        board_name: &str,
        tenant_id: &str,
        election_event_id: &str,
        election_ids: Option<Vec<String>>,
        executer: TallyExecuter,
    ) -> Result<()> {
        let electoral_log = ElectoralLog::new(
            self.transaction,
            tenant_id,
            Some(election_event_id),
            board_name,
        )
        .await?;
        electoral_log
            .post_tally_close(
                election_event_id.to_string(),
                election_ids,
                executer.user_id,
                executer.username,
            )
            .await
            .with_context(|| ELECTORAL_LOG_POST_FAILED)
    }
}
