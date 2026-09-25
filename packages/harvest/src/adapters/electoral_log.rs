// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::ports::electoral_log::{
    ElectoralLogs, PhoneBlacklistChange, PhoneBlacklistEntryLog,
};
use anyhow::anyhow;
use deadpool_postgres::Transaction;
use windmill::postgres::election_event::get_election_event_by_id;
use windmill::services::election_event_board::get_election_event_board;
use windmill::services::electoral_log::{
    post_voter_secret_attribute_audit, ElectoralLog, ElectoralLogAdminContext,
    VoterSecretAttributeAction, VoterSecretAttributeAudit,
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

    async fn phone_blacklist_entry(
        &self,
        transaction: &Transaction<'_>,
        entry: PhoneBlacklistEntryLog<'_>,
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
        let event_id = entry.election_event_id.to_string();
        let user_id = Some(entry.user_id.to_string());
        match entry.change {
            PhoneBlacklistChange::Created => {
                electoral_log
                    .post_phone_blacklist_entry_created(
                        event_id,
                        entry.phone_e164,
                        user_id,
                        entry.username,
                    )
                    .await?
            }
            PhoneBlacklistChange::Deleted => {
                electoral_log
                    .post_phone_blacklist_entry_deleted(
                        event_id,
                        entry.phone_e164,
                        user_id,
                        entry.username,
                    )
                    .await?
            }
        }
        Ok(())
    }
}
