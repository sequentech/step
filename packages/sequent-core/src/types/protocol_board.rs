// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The row of a board the platform created on the crypto core's board service,
//! and the helpers that tell a DKG board from a tally board.

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};

/// A protocol board the board service of the crypto core.
/// It could be meant for dkg, or tallying.
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolBoard {
    pub id: String,
    pub tenant_id: String,
    pub election_event_id: String,
    /// The DKG board a tally board is the child of; `None` for a DKG board.
    /// Read through [`ProtocolBoard::kind`], [`ProtocolBoard::as_dkg`] and
    /// [`ProtocolBoard::as_tally`].
    pub parent_id: Option<String>,
    pub keys_ceremony_id: String,
    /// The board name on the board service.
    pub name: String,
    /// The canonical bytes of the message sent by the protocol manager when
    /// the ceremony was created. It is what gets published by the platform.
    /// `Configuration` for dkg, `Ballots` for tally.
    pub manager_message: Vec<u8>,
    pub created_at: Option<chrono::DateTime<chrono::Local>>,
    /// The tally session a tally board belongs to; `None` for a DKG board.
    pub tally_session_id: Option<String>,
    /// The weight batch a tally board carries; `None` for a DKG board.
    pub batch: Option<i64>,
}

/// What a protocol board runs.
#[derive(
    Display,
    EnumString,
    Serialize,
    Deserialize,
    Debug,
    PartialEq,
    Eq,
    Clone,
    Copy,
)]
pub enum ProtocolBoardKind {
    /// The distributed key generation of a keys ceremony.
    DKG,
    /// A tally of the key its parent DKG board generated.
    TALLY,
}

impl ProtocolBoard {
    /// What the board runs: a board without a parent is a DKG board.
    pub fn kind(&self) -> ProtocolBoardKind {
        match self.parent_id {
            None => ProtocolBoardKind::DKG,
            Some(_) => ProtocolBoardKind::TALLY,
        }
    }

    /// The row, if it is a DKG board.
    pub fn as_dkg(&self) -> Option<&ProtocolBoard> {
        match self.parent_id {
            None => Some(self),
            Some(_) => None,
        }
    }

    /// The row with its tally columns, if it is a tally board. A row whose
    /// lineage and tally columns disagree is an error.
    pub fn as_tally(&self) -> Result<Option<TallyBoardRow<'_>>> {
        match (
            self.parent_id.as_deref(),
            self.tally_session_id.as_deref(),
            self.batch,
        ) {
            (None, None, None) => Ok(None),
            (Some(parent_id), Some(tally_session_id), Some(batch)) => {
                Ok(Some(TallyBoardRow {
                    board: self,
                    parent_id,
                    tally_session_id,
                    batch,
                }))
            }
            (parent_id, tally_session_id, batch) => bail!(
                "protocol board {} ({}) has parent {parent_id:?}, tally \
                 session {tally_session_id:?} and batch {batch:?}: a tally \
                 board has all three, a DKG board none",
                self.name,
                self.id
            ),
        }
    }
}

/// A tally board's row, with the id of its parent DKG board, its tally
/// session and its weight batch.
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub struct TallyBoardRow<'a> {
    pub board: &'a ProtocolBoard,
    pub parent_id: &'a str,
    pub tally_session_id: &'a str,
    pub batch: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn board(
        parent_id: Option<&str>,
        tally_session_id: Option<&str>,
        batch: Option<i64>,
    ) -> ProtocolBoard {
        ProtocolBoard {
            id: "board".to_string(),
            tenant_id: "tenant".to_string(),
            election_event_id: "event".to_string(),
            parent_id: parent_id.map(str::to_string),
            keys_ceremony_id: "ceremony".to_string(),
            name: "board_name".to_string(),
            manager_message: vec![1, 2, 3],
            created_at: None,
            tally_session_id: tally_session_id.map(str::to_string),
            batch,
        }
    }

    #[test]
    fn a_board_is_exactly_the_kind_its_lineage_says() {
        let dkg = board(None, None, None);
        assert_eq!(dkg.kind(), ProtocolBoardKind::DKG);
        assert_eq!(dkg.as_dkg(), Some(&dkg));
        assert_eq!(dkg.as_tally().unwrap(), None);

        let tally = board(Some("parent"), Some("session"), Some(7));
        assert_eq!(tally.kind(), ProtocolBoardKind::TALLY);
        assert_eq!(tally.as_dkg(), None);
        assert_eq!(
            tally.as_tally().unwrap(),
            Some(TallyBoardRow {
                board: &tally,
                parent_id: "parent",
                tally_session_id: "session",
                batch: 7,
            })
        );
    }

    #[test]
    fn a_row_whose_lineage_and_tally_columns_disagree_is_an_error() {
        for (parent_id, tally_session_id, batch) in [
            (Some("parent"), None, None),
            (Some("parent"), Some("session"), None),
            (Some("parent"), None, Some(7)),
            (None, Some("session"), None),
            (None, None, Some(7)),
            (None, Some("session"), Some(7)),
        ] {
            let row = board(parent_id, tally_session_id, batch);
            let error = format!("{:#}", row.as_tally().unwrap_err());
            assert!(error.contains("board_name"), "{error}");
        }
    }
}
