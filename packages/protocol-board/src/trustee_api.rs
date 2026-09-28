// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! What the platform and a trustee tell each other out of band: which boards
//! are the trustee's and how they relate, and what the trustee reports about
//! one of them. harvest serves and takes these as JSON, and the trustee reads
//! and sends the same types.

use anyhow::{bail, Context as _, Result};
use sequent_core::types::protocol_board::{ProtocolBoard, ProtocolBoardKind};
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};

use crate::ids::BoardName;

/// A board the trustee has work on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrusteeBoard {
    pub name: BoardName,
    pub kind: ProtocolBoardKind,
    /// The DKG board a tally board is the child of.
    pub parent: Option<BoardName>,
}

impl TrusteeBoard {
    /// The entry of a board row, with its parent's name if it has a parent.
    pub fn of(
        row: &ProtocolBoard,
        parent: Option<&str>,
    ) -> Result<TrusteeBoard> {
        let name = BoardName::parse(&row.name)?;
        let kind = row.kind();
        let parent = match (kind, parent) {
            (ProtocolBoardKind::DKG, None) => None,
            (ProtocolBoardKind::TALLY, Some(parent)) => Some(
                BoardName::parse(parent)
                    .with_context(|| format!("the parent of board {name}"))?,
            ),
            (ProtocolBoardKind::DKG, Some(parent)) => {
                bail!("DKG board {name} has no parent, yet {parent} was given")
            }
            (ProtocolBoardKind::TALLY, None) => {
                bail!("tally board {name} was given no parent")
            }
        };
        Ok(TrusteeBoard { name, kind, parent })
    }
}

/// The answer to a trustee asking for its boards.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrusteeBoardsResponse {
    pub boards: Vec<TrusteeBoard>,
}

/// What a trustee reports about one of its boards.
#[derive(
    Display,
    EnumString,
    Serialize,
    Deserialize,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
)]
pub enum TrusteeReportKind {
    /// It halted its session over the board.
    HALTED,
}

/// A trustee's report about one of its boards. The trustee it is about is
/// the one that sends it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrusteeReport {
    pub board: BoardName,
    pub kind: TrusteeReportKind,
    /// braid's error, as the trustee logged it.
    pub detail: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const DKG_NAME: &str = "dkg_d9792af071b849528aac94bc0fead5f7";
    const TALLY_NAME: &str = "tally_0f6a3b2c9d8e4f10a1b2c3d4e5f60718_3";

    fn row(name: &str, parent_id: Option<&str>) -> ProtocolBoard {
        ProtocolBoard {
            id: "row-id".to_string(),
            tenant_id: "tenant-id".to_string(),
            election_event_id: "election-event-id".to_string(),
            parent_id: parent_id.map(str::to_string),
            keys_ceremony_id: "keys-ceremony-id".to_string(),
            name: name.to_string(),
            manager_message: vec![1, 2, 3],
            created_at: None,
            tally_session_id: parent_id.map(|_| "tally-session-id".to_string()),
            batch: parent_id.map(|_| 3),
        }
    }

    fn board_name(name: &str) -> BoardName {
        BoardName::parse(name).unwrap()
    }

    fn refusal(row: &ProtocolBoard, parent: Option<&str>) -> String {
        match TrusteeBoard::of(row, parent) {
            Ok(board) => panic!("expected an error, got {board:?}"),
            Err(err) => format!("{err:#}"),
        }
    }

    #[test]
    fn a_dkg_row_is_listed_without_a_parent() {
        assert_eq!(
            TrusteeBoard::of(&row(DKG_NAME, None), None).unwrap(),
            TrusteeBoard {
                name: board_name(DKG_NAME),
                kind: ProtocolBoardKind::DKG,
                parent: None,
            }
        );
    }

    #[test]
    fn a_tally_row_is_listed_with_its_parent_board_name() {
        let tally = row(TALLY_NAME, Some("parent-row-id"));
        assert_eq!(
            TrusteeBoard::of(&tally, Some(DKG_NAME)).unwrap(),
            TrusteeBoard {
                name: board_name(TALLY_NAME),
                kind: ProtocolBoardKind::TALLY,
                parent: Some(board_name(DKG_NAME)),
            }
        );
    }

    #[test]
    fn a_row_whose_lineage_and_parent_disagree_is_an_error_naming_it() {
        let error = refusal(&row(DKG_NAME, None), Some(TALLY_NAME));
        assert!(error.contains(DKG_NAME), "{error}");

        let error = refusal(&row(TALLY_NAME, Some("parent-row-id")), None);
        assert!(error.contains(TALLY_NAME), "{error}");
    }

    #[test]
    fn a_name_the_platform_does_not_mint_is_an_error() {
        let error = refusal(&row("../dkg", None), None);
        assert!(error.contains("../dkg"), "{error}");

        let tally = row(TALLY_NAME, Some("parent-row-id"));
        let error = refusal(&tally, Some("dkg board"));
        assert!(error.contains(TALLY_NAME), "{error}");
        assert!(error.contains("dkg board"), "{error}");
    }

    #[test]
    fn the_board_list_and_the_report_have_the_routes_json_shapes() {
        let boards = TrusteeBoardsResponse {
            boards: vec![
                TrusteeBoard::of(&row(DKG_NAME, None), None).unwrap(),
                TrusteeBoard::of(
                    &row(TALLY_NAME, Some("parent-row-id")),
                    Some(DKG_NAME),
                )
                .unwrap(),
            ],
        };
        let boards_json = json!({
            "boards": [
                { "name": DKG_NAME, "kind": "DKG", "parent": null },
                { "name": TALLY_NAME, "kind": "TALLY", "parent": DKG_NAME },
            ]
        });
        assert_eq!(serde_json::to_value(&boards).unwrap(), boards_json);
        assert_eq!(
            serde_json::from_value::<TrusteeBoardsResponse>(boards_json)
                .unwrap(),
            boards
        );

        let report = TrusteeReport {
            board: board_name(DKG_NAME),
            kind: TrusteeReportKind::HALTED,
            detail: "the board was rewritten".to_string(),
        };
        let report_json = json!({
            "board": DKG_NAME,
            "kind": "HALTED",
            "detail": "the board was rewritten",
        });
        assert_eq!(serde_json::to_value(&report).unwrap(), report_json);
        assert_eq!(
            serde_json::from_value::<TrusteeReport>(report_json).unwrap(),
            report
        );
    }

    #[test]
    fn a_report_about_a_name_the_platform_does_not_mint_is_not_read() {
        let report = json!({
            "board": "../dkg",
            "kind": "HALTED",
            "detail": "the board was rewritten",
        });
        assert!(serde_json::from_value::<TrusteeReport>(report).is_err());
    }
}
