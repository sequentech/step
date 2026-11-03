// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Operator-scheduled retention: callers supply the approved event and scope.
//! No period is assumed and no public API exposes this destructive task.
use crate::services::electoral_log_board::get_event_databases;
use crate::types::error::Result;
use celery::error::TaskError;
use electoral_log::adapters::privacy::PrivacyPurge;

/// One bounded transaction per active/retired copy in each invocation. Repeat
/// until the total count is zero; evidence and tombstones survive retries.
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn purge_vote_privacy(
    tenant_id: String,
    election_event_id: String,
    scope: PrivacyPurge,
) -> Result<u64> {
    let databases = get_event_databases().await?;
    Ok(databases
        .purge_event_privacy(&tenant_id, &election_event_id, scope, 5000)
        .await?)
}
