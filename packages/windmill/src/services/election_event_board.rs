// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use b3::client::pgsql::B3IndexRow;
use sequent_core::serialization::deserialize_with_path::deserialize_value;
use serde::{Deserialize, Serialize};
use serde_json::value::Value;

#[derive(Deserialize, Serialize, Debug, Clone)]
/// Serializable view of a B3 board reference stored in event configuration.
pub struct BoardSerializable {
    /// Internal numeric board id.
    pub id: i64,
    /// Database/board name used by B3/immudb backends.
    pub database_name: String,
    /// Whether the board has been archived.
    pub is_archived: bool,
}

impl From<B3IndexRow> for BoardSerializable {
    fn from(val: B3IndexRow) -> Self {
        BoardSerializable {
            id: val.id.into(),
            database_name: val.board_name,
            is_archived: val.is_archived,
        }
    }
}

/// Get the database name for a B3 board reference.
pub fn get_election_event_board(bulletin_board_reference: Option<Value>) -> Option<String> {
    bulletin_board_reference.and_then(|board_json| {
        let opt_board: Option<BoardSerializable> = deserialize_value(board_json).ok();

        opt_board.map(|board| board.database_name)
    })
}
