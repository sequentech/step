// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use immu_board::Board;
use serde::{Deserialize, Serialize};
use serde_json::value::Value;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct BoardSerializable {
    pub id: i64,
    pub database_name: String,
    pub is_archived: bool,
}

impl Into<BoardSerializable> for Board {
    fn into(self) -> BoardSerializable {
        BoardSerializable {
            id: self.id,
            database_name: self.database_name,
            is_archived: self.is_archived,
        }
    }
}

pub fn get_election_event_board(bulletin_board_reference: Option<Value>) -> Option<String> {
    bulletin_board_reference.and_then(|board_json| {
        let opt_board: Option<BoardSerializable> = deserialize_value(board_json).ok();

        opt_board.map(|board| board.database_name)
    })
}
