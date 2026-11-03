// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::ports::electoral_log::{ApprovalMatrixLog, ElectoralLogs};
use anyhow::anyhow;
use deadpool_postgres::Transaction;
use electoral_log::messages::newtypes::BallotPublicationStage;
use sequent_core::types::hasura::core::TasksExecution;
use windmill::postgres::election_event::get_election_event_by_id;
use windmill::services::election_event_board::get_election_event_board;
use windmill::services::electoral_log::{
    log_ballot_publication_failure, post_voter_secret_attribute_audit,
    ElectoralLog, ElectoralLogAdminContext, VoterSecretAttributeAction,
    VoterSecretAttributeAudit,
};

/// The election event's electoral log on its bulletin board.
pub struct BoardElectoralLogs;

#[rocket::async_trait]
impl ElectoralLogs for BoardElectoralLogs {
    async fn voter_secret_attributes(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        admin: &ElectoralLogAdminContext,
        action: VoterSecretAttributeAction,
        audit: VoterSecretAttributeAudit<'_>,
    ) -> anyhow::Result<()> {
        post_voter_secret_attribute_audit(
            tenant_id,
            election_event_id,
            admin,
            action,
            audit,
        )
        .await
    }

    async fn approval_matrix_updated(
        &self,
        transaction: &Transaction<'_>,
        entry: ApprovalMatrixLog<'_>,
    ) -> anyhow::Result<()> {
        let event = get_election_event_by_id(
            transaction,
            entry.tenant_id,
            entry.election_event_id,
        )
        .await?;
        let electoral_log = ElectoralLog::for_admin_user(
            transaction,
            &get_election_event_board(event.bulletin_board_reference)
                .ok_or(anyhow!("missing board"))?,
            entry.tenant_id,
            entry.election_event_id,
            entry.user_id,
            entry.username.clone(),
            None,
            None,
        )
        .await?;
        electoral_log
            .post_approval_matrix_updated(
                entry.election_event_id.to_string(),
                entry.version,
                entry.sha256,
                Some(entry.user_id.to_string()),
                entry.username,
            )
            .await
    }

    async fn ballot_publication_failure(
        &self,
        task: &TasksExecution,
        publication_id: &str,
        stage: BallotPublicationStage,
        message: &str,
    ) -> anyhow::Result<()> {
        log_ballot_publication_failure(task, publication_id, stage, message)
            .await
    }
}
