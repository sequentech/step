// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::pipes::do_tally::counting_algorithm::instant_runoff::{TieBreakingState, RunoffStatus};
use serde_json::Value;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug)]
pub enum Error {
    EmptyTallyResults,
    InvalidTallyOperation(String),
    CandidateNotFound(String),
    UnexpectedError(String),
    /// Tie detected that requires external resolution
    /// Contains the paused state and tie information for windmill to save
    TieRequiresExternalInput {
        paused_state: Value,  // Serialized RunoffStatus
        tie_info: TieBreakingState,
    },
}

impl core::fmt::Display for Error {
    fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::result::Result<(), core::fmt::Error> {
        match self {
            Error::TieRequiresExternalInput { paused_state, tie_info } => {
                // Output structured JSON that windmill can parse to extract the data
                let output = serde_json::json!({
                    "error_type": "TieRequiresExternalInput",
                    "paused_state": paused_state,
                    "tie_info": {
                        "round_number": tie_info.round_number,
                        "tied_candidate_ids": tie_info.tied_candidate_ids,
                        "vote_counts": tie_info.vote_counts,
                        "method_used": format!("{:?}", tie_info.method_used),
                        "resolved_by_candidate_id": tie_info.resolved_by_candidate_id,
                    }
                });
                write!(fmt, "TIE_REQUIRES_EXTERNAL_INPUT_JSON:{}", output.to_string())
            }
            _ => write!(fmt, "{self:?}")
        }
    }
}

impl std::error::Error for Error {}
