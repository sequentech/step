// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::ports::electoral_log::{
    ElectoralLogs, PhoneBlacklistChange, PhoneBlacklistEntryLog,
};
use anyhow::anyhow;
use deadpool_postgres::Transaction;
use std::sync::Mutex;
use windmill::services::electoral_log::{
    ElectoralLogAdminContext, VoterSecretAttributeAction,
    VoterSecretAttributeAudit,
};

/// An entry as the electoral log received it.
#[derive(Clone, Debug, PartialEq)]
pub enum LoggedEntry {
    VoterSecretAttributes {
        election_event_id: String,
        admin_id: String,
        action: VoterSecretAttributeAction,
        voter_id: Option<String>,
        attribute_names: Vec<String>,
        document_id: Option<String>,
    },
    PhoneBlacklistEntry {
        change: PhoneBlacklistChange,
        election_event_id: String,
        admin_id: String,
        phone_e164: String,
    },
}

/// Records electoral-log entries. A refusing log rejects every entry, as
/// when the bulletin board is unreachable.
#[derive(Default)]
pub struct MemoryElectoralLogs {
    entries: Mutex<Vec<LoggedEntry>>,
    refuses: bool,
}

impl MemoryElectoralLogs {
    pub fn refusing() -> Self {
        Self {
            refuses: true,
            ..Default::default()
        }
    }

    pub fn entries(&self) -> Vec<LoggedEntry> {
        self.entries.lock().unwrap().clone()
    }

    fn record(&self, entry: LoggedEntry) -> anyhow::Result<()> {
        if self.refuses {
            return Err(anyhow!("the bulletin board is unreachable"));
        }
        self.entries.lock().unwrap().push(entry);
        Ok(())
    }
}

#[rocket::async_trait]
impl ElectoralLogs for MemoryElectoralLogs {
    async fn voter_secret_attributes(
        &self,
        _tenant_id: &str,
        election_event_id: &str,
        admin: &ElectoralLogAdminContext,
        action: VoterSecretAttributeAction,
        audit: VoterSecretAttributeAudit<'_>,
    ) -> anyhow::Result<()> {
        self.record(LoggedEntry::VoterSecretAttributes {
            election_event_id: election_event_id.to_string(),
            admin_id: admin.user_id.clone(),
            action,
            voter_id: audit.voter_id.map(str::to_string),
            attribute_names: audit.attribute_names.to_vec(),
            document_id: audit.document_id.map(str::to_string),
        })
    }

    async fn phone_blacklist_entry(
        &self,
        _transaction: &Transaction<'_>,
        entry: PhoneBlacklistEntryLog<'_>,
    ) -> anyhow::Result<()> {
        self.record(LoggedEntry::PhoneBlacklistEntry {
            change: entry.change,
            election_event_id: entry.election_event_id.to_string(),
            admin_id: entry.user_id.to_string(),
            phone_e164: entry.phone_e164,
        })
    }
}
