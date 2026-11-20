// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! One receipt render per cast ballot: the receipt task of a ballot has a
//! fixed id, so repeated requests reuse the render in progress or the finished
//! document instead of queueing another one.

use crate::postgres::tasks_execution::{claim_document_task_execution, DocumentTaskClaim};
use crate::services::serialize_tasks_logs::general_start_log;
use crate::types::tasks::ETasksExecution;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::types::hasura::core::TasksExecution;
use sha2::{Digest, Sha256};
use tracing::instrument;
use uuid::Uuid;

/// After this long an IN_PROGRESS receipt task is treated as lost (for
/// example a worker crash) and a new request may render the receipt again.
pub const BALLOT_RECEIPT_IN_PROGRESS_TIMEOUT_SECS: i32 = 10 * 60;

/// The receipt of one ballot cast by one voter, as rendered for one client.
#[derive(Debug, Clone, Copy)]
pub struct BallotReceiptKey<'a> {
    pub tenant_id: &'a str,
    pub election_event_id: &'a str,
    pub election_id: &'a str,
    pub voter_id: &'a str,
    pub ballot_id: &'a str,
    pub is_kiosk: bool,
}

impl BallotReceiptKey<'_> {
    /// Task id of this receipt: a v4-shaped UUID derived from the key, so it
    /// passes the v4 checks on task ids. Ids are lower-cased first.
    pub fn task_id(&self) -> Uuid {
        let key = serde_json::json!([
            "ballot-receipt-task/v1",
            self.tenant_id.to_lowercase(),
            self.election_event_id.to_lowercase(),
            self.election_id.to_lowercase(),
            self.voter_id,
            self.ballot_id,
            self.is_kiosk,
        ]);
        let digest = Sha256::digest(key.to_string().as_bytes());
        let mut bytes = [0u8; 16];
        bytes.copy_from_slice(&digest[..16]);
        uuid::Builder::from_random_bytes(bytes).into_uuid()
    }
}

/// What the caller does with a receipt request.
#[derive(Debug)]
pub struct BallotReceiptClaim {
    /// Document the receipt is (or will be) stored as.
    pub document_id: String,
    pub task_execution: TasksExecution,
    /// True when the caller must queue the render; false when an earlier
    /// render of the same receipt is reused.
    pub queue_render: bool,
}

/// Claims the receipt task of `key` in `hasura_transaction`; the caller
/// commits before queueing the render.
#[instrument(skip(hasura_transaction), err)]
pub async fn claim_ballot_receipt_task(
    hasura_transaction: &Transaction<'_>,
    key: &BallotReceiptKey<'_>,
    executed_by_user: &str,
) -> Result<BallotReceiptClaim> {
    let task_type = ETasksExecution::CREATE_BALLOT_RECEIPT;
    let new_document_id = Uuid::new_v4().to_string();
    let claim = claim_document_task_execution(
        hasura_transaction,
        key.tenant_id,
        key.election_event_id,
        &key.task_id(),
        &task_type.to_name(),
        &task_type.to_string(),
        &serde_json::json!({ "document_id": new_document_id }),
        &serde_json::to_value(general_start_log())?,
        executed_by_user,
        BALLOT_RECEIPT_IN_PROGRESS_TIMEOUT_SECS,
    )
    .await
    .context("Error claiming the ballot receipt task")?;

    Ok(match claim {
        DocumentTaskClaim::Claimed(task_execution) => BallotReceiptClaim {
            document_id: new_document_id,
            task_execution,
            queue_render: true,
        },
        DocumentTaskClaim::Existing(task_execution) => BallotReceiptClaim {
            document_id: task_execution
                .annotations
                .as_ref()
                .and_then(|annotations| annotations.get("document_id"))
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| anyhow!("Receipt task {} has no document id", task_execution.id))?
                .to_string(),
            task_execution,
            queue_render: false,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(ballot_id: &str, is_kiosk: bool) -> BallotReceiptKey<'_> {
        BallotReceiptKey {
            tenant_id: "4f1c6b38-9f5e-4a59-8d3b-2a7f0c1e5d61",
            election_event_id: "b7e2d915-3c4a-4e8f-9a61-5d0f2c8b7e43",
            election_id: "0d3f6e2a-8b1c-4f7e-9a5d-6c2b1e0f4a87",
            voter_id: "voter",
            ballot_id,
            is_kiosk,
        }
    }

    /// The same receipt always maps to the same task id, which is a valid v4
    /// UUID, and letter case in the ids does not change it.
    #[test]
    fn the_task_id_is_stable_and_v4() {
        let id = key("ballot", false).task_id();
        assert_eq!(id, key("ballot", false).task_id());
        assert_eq!(id.get_version(), Some(uuid::Version::Random));
        let upper = BallotReceiptKey {
            election_event_id: "B7E2D915-3C4A-4E8F-9A61-5D0F2C8B7E43",
            ..key("ballot", false)
        };
        assert_eq!(id, upper.task_id());
    }

    /// Another ballot, voter or client is another receipt.
    #[test]
    fn each_receipt_has_its_own_task_id() {
        let id = key("ballot", false).task_id();
        assert_ne!(id, key("other-ballot", false).task_id());
        assert_ne!(id, key("ballot", true).task_id());
        let other_voter = BallotReceiptKey {
            voter_id: "other-voter",
            ..key("ballot", false)
        };
        assert_ne!(id, other_voter.task_id());
    }
}
