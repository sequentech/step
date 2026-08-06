// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/// Domain errors raised by tally ceremony services that callers (e.g. the
/// harvest HTTP routes) should map to a specific 4xx status rather than a
/// generic 500, instead of relying on error-message string matching.
#[derive(Debug, thiserror::Error)]
pub enum TallyRecountError {
    #[error("Tally session {tally_session_id} has no execution history and cannot be recounted")]
    NoExecutionHistory { tally_session_id: String },
}
