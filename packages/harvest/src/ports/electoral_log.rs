// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use deadpool_postgres::Transaction;
use windmill::services::electoral_log::{
    ElectoralLogAdminContext, VoterSecretAttributeAction,
    VoterSecretAttributeAudit,
};

/// A phone blacklist entry that was added or removed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhoneBlacklistChange {
    Created,
    Deleted,
}

/// The admin and number of a phone blacklist change.
pub struct PhoneBlacklistEntryLog<'a> {
    pub change: PhoneBlacklistChange,
    pub tenant_id: &'a str,
    pub election_event_id: &'a str,
    pub user_id: &'a str,
    pub username: Option<String>,
    pub phone_e164: String,
}

/// The electoral-log entries route handlers post before the action they
/// record takes effect.
#[rocket::async_trait]
pub trait ElectoralLogs: Send + Sync {
    async fn voter_secret_attributes(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        admin: &ElectoralLogAdminContext,
        action: VoterSecretAttributeAction,
        audit: VoterSecretAttributeAudit<'_>,
    ) -> anyhow::Result<()>;

    /// Posted on the transaction that writes the entry.
    async fn phone_blacklist_entry(
        &self,
        transaction: &Transaction<'_>,
        entry: PhoneBlacklistEntryLog<'_>,
    ) -> anyhow::Result<()>;
}
