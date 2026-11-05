// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The database refuses to change a voting status (`status`) or the
//! lockdown (`presentation.locked_down`) of an election or election event
//! unless the transaction is marked as the server's own (migration
//! `1791000000800_guard_trusted_status_and_lockdown`). Hasura's admin roles
//! can write those columns, so without the mark an admin acting alone could
//! open or close voting, or lift the lockdown, skipping the server's checks,
//! signatures and electoral log entries.

use anyhow::{Context, Result};
use deadpool_postgres::Transaction;

/// Marks the transaction as the server's own until it ends, so it can
/// change the guarded values. Call it only in the paths that implement those
/// changes (the voting status actions and tasks, the lockdown task), right
/// before their writes.
pub async fn trusted_write(transaction: &Transaction<'_>) -> Result<()> {
    transaction
        .execute(
            "SELECT set_config('sequent.trusted_write', 'on', true)",
            &[],
        )
        .await
        .context("Error marking the transaction as a trusted write")?;
    Ok(())
}
