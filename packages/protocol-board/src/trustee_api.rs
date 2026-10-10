// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! What the platform and a trustee tell each other out of band: which boards
//! are the trustee's and how they relate, and what the trustee reports about
//! one of them. harvest serves and takes these as JSON, and the trustee reads
//! and sends the same types.

use anyhow::{Context as _, Result};
use sequent_core::types::protocol_board::ProtocolBoardKind;
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
    /// The entry of board `name`, with its parent's name if it has one: a
    /// board with a parent is a tally board, one without is a DKG board.
    pub fn of(name: &str, parent: Option<&str>) -> Result<TrusteeBoard> {
        let name = BoardName::parse(name)?;
        let (kind, parent) =
            match parent {
                None => (ProtocolBoardKind::DKG, None),
                Some(parent) => (
                    ProtocolBoardKind::TALLY,
                    Some(BoardName::parse(parent).with_context(|| {
                        format!("the parent of board {name}")
                    })?),
                ),
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

    fn board_name(name: &str) -> BoardName {
        BoardName::parse(name).unwrap()
    }

    fn refusal(name: &str, parent: Option<&str>) -> String {
        match TrusteeBoard::of(name, parent) {
            Ok(board) => panic!("expected an error, got {board:?}"),
            Err(err) => format!("{err:#}"),
        }
    }

    #[test]
    fn a_board_without_a_parent_is_listed_as_a_dkg_board() {
        assert_eq!(
            TrusteeBoard::of(DKG_NAME, None).unwrap(),
            TrusteeBoard {
                name: board_name(DKG_NAME),
                kind: ProtocolBoardKind::DKG,
                parent: None,
            }
        );
    }

    #[test]
    fn a_board_with_a_parent_is_listed_as_a_tally_board_of_it() {
        assert_eq!(
            TrusteeBoard::of(TALLY_NAME, Some(DKG_NAME)).unwrap(),
            TrusteeBoard {
                name: board_name(TALLY_NAME),
                kind: ProtocolBoardKind::TALLY,
                parent: Some(board_name(DKG_NAME)),
            }
        );
    }

    #[test]
    fn a_name_the_platform_does_not_mint_is_an_error() {
        let error = refusal("../dkg", None);
        assert!(error.contains("../dkg"), "{error}");

        let error = refusal(TALLY_NAME, Some("dkg board"));
        assert!(error.contains(TALLY_NAME), "{error}");
        assert!(error.contains("dkg board"), "{error}");
    }

    #[test]
    fn the_board_list_and_the_report_have_the_routes_json_shapes() {
        let boards = TrusteeBoardsResponse {
            boards: vec![
                TrusteeBoard::of(DKG_NAME, None).unwrap(),
                TrusteeBoard::of(TALLY_NAME, Some(DKG_NAME)).unwrap(),
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
