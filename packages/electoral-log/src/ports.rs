// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::domain::{ElectoralLogMessage, LogEntry, LogQuery};
use anyhow::Result;
use async_trait::async_trait;

/// Storage owns transaction boundaries. An iterator permits bounded-memory
/// imports; either every entry commits or the whole append rolls back.
#[async_trait]
pub trait ElectoralLogStore: Send + Sync {
    async fn create_board(&self, board: &str) -> Result<()>;
    async fn delete_board(&self, board: &str) -> Result<()>;
    async fn has_board(&self, board: &str) -> Result<bool>;
    async fn append(
        &self,
        board: &str,
        entries: &mut (dyn Iterator<Item = Result<LogEntry>> + Send),
    ) -> Result<()>;
    async fn query(&self, board: &str, query: &LogQuery) -> Result<Vec<ElectoralLogMessage>>;
    async fn count(&self, board: &str, query: &LogQuery) -> Result<i64>;
}
