// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use deadpool_postgres::{Client, Transaction};
use std::sync::Mutex;
use windmill::services::monitoring::config_store::{
    Author, EventRef, MonitoringConfigAudit, RecordedChange,
};

/// The electoral log's record of configuration changes, kept in memory.
#[derive(Default)]
pub struct MemoryConfigAudit {
    changes: Mutex<Vec<RecordedChange>>,
}

impl MemoryConfigAudit {
    pub fn changes(&self) -> Vec<RecordedChange> {
        self.changes.lock().unwrap().clone()
    }
}

#[rocket::async_trait]
impl MonitoringConfigAudit for MemoryConfigAudit {
    async fn prepare(
        &self,
        _client: &mut Client,
        _event: EventRef,
        _author: &Author,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    async fn record(
        &self,
        _transaction: &Transaction<'_>,
        change: &RecordedChange,
    ) -> anyhow::Result<()> {
        self.changes.lock().unwrap().push(change.clone());
        Ok(())
    }
}
