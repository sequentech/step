// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/// Domain errors raised by the tally sheet import service that callers (e.g.
/// the harvest HTTP routes) should map to a specific 4xx status rather than a
/// generic 500, instead of relying on error-message string matching.
#[derive(Debug, thiserror::Error)]
pub enum TallySheetImportError {
    #[error("Tally sheet import {0} not found")]
    NotFound(String),
    #[error("Document {0} not found")]
    DocumentNotFound(String),
    #[error("Document {document_id} is too large ({size} bytes, max {max} bytes)")]
    DocumentTooLarge {
        document_id: String,
        size: u64,
        max: u64,
    },
    #[error("Tally sheet import {import_id} cannot be reviewed from status {status}")]
    InvalidReviewState { import_id: String, status: String },
}
