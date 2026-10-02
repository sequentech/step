// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
//! Versioned electoral-record commitments and portable verification bundles.
use crate::domain::LogEntry;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
pub use trellis::journal::{Checkpoint, Consistency, Inclusion, Journal};

/// Hash the exact stored record, including its database ID and delivery identity.
pub fn leaf_hash(board: &str, entry: &LogEntry) -> Result<Vec<u8>> {
    Ok(Sha256::digest(serde_json::to_vec(&(
        "sequent-electoral-log-v1",
        board,
        entry,
    ))?)
    .to_vec())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RecordProof {
    pub entry: LogEntry,
    pub inclusion: Inclusion,
}
impl RecordProof {
    pub fn verify(&self, checkpoint: &Checkpoint) -> Result<()> {
        self.inclusion
            .verify(&leaf_hash(&checkpoint.log_name, &self.entry)?, checkpoint)
    }
}
