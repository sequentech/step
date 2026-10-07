// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::ballot_box::BallotBoxPolicy;
use b4::client::pgsql::B3IndexRow;
use sequent_core::serialization::deserialize_with_path::deserialize_value;
use serde::{Deserialize, Serialize};
use serde_json::value::Value;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct BoardSerializable {
    pub id: i64,
    pub database_name: String,
    pub is_archived: bool,
    /// Where the event's cast votes are stored; absent for events created before
    /// the ballot box.
    #[serde(default)]
    pub ballot_box: BallotBoxPolicy,
}

impl Into<BoardSerializable> for B3IndexRow {
    fn into(self) -> BoardSerializable {
        BoardSerializable {
            id: self.id.into(),
            database_name: self.board_name,
            is_archived: self.is_archived,
            ballot_box: BallotBoxPolicy::default(),
        }
    }
}

pub fn get_election_event_board(bulletin_board_reference: Option<Value>) -> Option<String> {
    bulletin_board_reference.and_then(|board_json| {
        let opt_board: Option<BoardSerializable> = deserialize_value(board_json).ok();

        opt_board.map(|board| board.database_name)
    })
}

/// Where an election event's cast votes are stored.
pub fn get_ballot_box_policy(bulletin_board_reference: Option<Value>) -> BallotBoxPolicy {
    bulletin_board_reference
        .and_then(|board_json| deserialize_value::<BoardSerializable>(board_json).ok())
        .map(|board| board.ballot_box)
        .unwrap_or_default()
}
