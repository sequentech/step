// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! What the admin portal needs to verify a tally's bulletin boards in the
//! browser with the public `WasmVerifier`: each tally board of a tally
//! session, its DKG board, and the canonical `Configuration` and `Ballots`
//! messages the platform created them with. The verifier derives its trust
//! anchors from those messages, never from the boards it checks.

use anyhow::Result;
use base64::{engine::general_purpose::STANDARD, Engine};
use deadpool_postgres::Transaction;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::postgres::protocol_board::get_tally_boards_with_dkg;

/// One tally board to verify, and the contest area whose ballots it mixes
/// and decrypts: its election's and contest's `presentation`, which hold
/// their names, and its area's name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TallyVerificationBoard {
    pub dkg_board: String,
    pub tally_board: String,
    pub batch: i32,
    pub election_presentation: Option<Value>,
    pub contest_presentation: Option<Value>,
    pub area_name: Option<String>,
    /// The DKG board's `Configuration` message, in padded standard base64.
    pub configuration_message: String,
    /// The tally board's `Ballots` message, in padded standard base64.
    pub ballots_message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TallyVerificationBoards {
    pub boards: Vec<TallyVerificationBoard>,
}

/// The boards of tally session `tally_session_id`, by batch. A tally session
/// of another tenant or election event has none.
pub async fn list_boards(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    tally_session_id: &str,
) -> Result<TallyVerificationBoards> {
    let boards = get_tally_boards_with_dkg(
        hasura_transaction,
        tenant_id,
        election_event_id,
        tally_session_id,
    )
    .await?;
    Ok(TallyVerificationBoards {
        boards: boards
            .into_iter()
            .map(|board| TallyVerificationBoard {
                dkg_board: board.dkg_board,
                tally_board: board.tally_board,
                batch: board.batch,
                election_presentation: board.election_presentation,
                contest_presentation: board.contest_presentation,
                area_name: board.area_name,
                configuration_message: STANDARD.encode(board.configuration_message),
                ballots_message: STANDARD.encode(board.ballots_message),
            })
            .collect(),
    })
}
