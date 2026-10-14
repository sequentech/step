// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! How a board and its protocol domain are named.

use std::fmt;

use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Board names of a keys ceremony's distributed key generation start with
/// this.
const DKG_BOARD_PREFIX: &str = "dkg_";

/// Board names of a tally session's boards start with this.
const TALLY_BOARD_PREFIX: &str = "tally_";

/// Separates a tally board name's session from its batch.
const TALLY_BATCH_SEPARATOR: char = '_';

/// The name of one board on the board service.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(try_from = "String", into = "String")]
pub struct BoardName(String);

impl BoardName {
    /// The DKG board of a key ceremony.
    pub fn for_dkg(keys_ceremony_id: &Uuid) -> BoardName {
        BoardName(format!(
            "{DKG_BOARD_PREFIX}{}",
            keys_ceremony_id.as_simple()
        ))
    }

    /// The board of one weight batch of a tally session.
    pub fn for_tally(tally_session_id: &Uuid, batch: i64) -> BoardName {
        BoardName(format!(
            "{TALLY_BOARD_PREFIX}{}{TALLY_BATCH_SEPARATOR}{batch}",
            tally_session_id.as_simple()
        ))
    }

    /// A name `for_dkg` or `for_tally` would have minted. The trustee turns
    /// board names into file names and URL path segments, so names read from
    /// rows or JSON are held to the platform's own naming.
    pub fn parse(name: &str) -> Result<BoardName> {
        BoardName::remint_dkg(name)
            .or_else(|| BoardName::remint_tally(name))
            .filter(|board| board.as_str() == name)
            .with_context(|| {
                format!("{name:?} is not a board name the platform mints")
            })
    }

    /// The DKG board name built from what `name` claims to carry.
    fn remint_dkg(name: &str) -> Option<BoardName> {
        let id = Uuid::parse_str(name.strip_prefix(DKG_BOARD_PREFIX)?).ok()?;
        Some(BoardName::for_dkg(&id))
    }

    /// The tally board name built from what `name` claims to carry.
    fn remint_tally(name: &str) -> Option<BoardName> {
        let (id, batch) = name
            .strip_prefix(TALLY_BOARD_PREFIX)?
            .split_once(TALLY_BATCH_SEPARATOR)?;
        Some(BoardName::for_tally(
            &Uuid::parse_str(id).ok()?,
            batch.parse().ok()?,
        ))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for BoardName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl TryFrom<String> for BoardName {
    type Error = anyhow::Error;

    fn try_from(name: String) -> Result<BoardName> {
        BoardName::parse(&name)
    }
}

impl From<BoardName> for String {
    fn from(name: BoardName) -> String {
        name.0
    }
}

/// The `Configuration.id` of a keys ceremony: the ceremony's UUID read as a
/// big-endian 128-bit number, which loses nothing, so a party holding only a
/// `Configuration` can say which ceremony it belongs to.
pub(crate) fn configuration_id(keys_ceremony_id: &Uuid) -> u128 {
    keys_ceremony_id.as_u128()
}

/// The `tally_id` a tally board's `Ballots` message declares: the board row's
/// UUID read as a big-endian 128-bit number. Row ids are unique across
/// sessions, batches and re-runs, which is what keeps the proof transcripts of
/// sibling tallies over one key apart.
pub(crate) fn tally_id(board_row_id: &Uuid) -> u128 {
    board_row_id.as_u128()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dkg_board_name_prefixes_the_dashless_uuid() {
        let id =
            Uuid::parse_str("d9792af0-71b8-4952-8aac-94bc0fead5f7").unwrap();
        assert_eq!(
            BoardName::for_dkg(&id).as_str(),
            "dkg_d9792af071b849528aac94bc0fead5f7"
        );
    }

    #[test]
    fn tally_board_name_joins_the_dashless_session_uuid_and_the_batch() {
        let id =
            Uuid::parse_str("d9792af0-71b8-4952-8aac-94bc0fead5f7").unwrap();
        assert_eq!(
            BoardName::for_tally(&id, 42).as_str(),
            "tally_d9792af071b849528aac94bc0fead5f7_42"
        );
    }

    #[test]
    fn a_configuration_id_names_its_ceremony() {
        let id = Uuid::new_v4();
        assert_eq!(Uuid::from_u128(configuration_id(&id)), id);
    }

    #[test]
    fn a_tally_id_names_its_board_row() {
        let id = Uuid::new_v4();
        assert_eq!(Uuid::from_u128(tally_id(&id)), id);
        assert_ne!(tally_id(&id), tally_id(&Uuid::new_v4()));
    }

    #[test]
    fn a_minted_board_name_parses_back_to_itself() {
        for id in [Uuid::nil(), Uuid::from_u128(u128::MAX), Uuid::new_v4()] {
            let dkg = BoardName::for_dkg(&id);
            assert_eq!(BoardName::parse(dkg.as_str()).unwrap(), dkg);
            for batch in [0, 1, 10, i64::MAX, -1, i64::MIN] {
                let tally = BoardName::for_tally(&id, batch);
                assert_eq!(BoardName::parse(tally.as_str()).unwrap(), tally);
            }
        }
    }

    #[test]
    fn a_session_and_a_batch_name_one_tally_board() {
        let id = Uuid::new_v4();
        assert_ne!(BoardName::for_tally(&id, 1), BoardName::for_tally(&id, 2));
        assert_ne!(
            BoardName::for_tally(&id, 1),
            BoardName::for_tally(&Uuid::new_v4(), 1)
        );
        assert_ne!(
            BoardName::for_tally(&id, 1).as_str(),
            BoardName::for_dkg(&id).as_str()
        );
    }

    #[test]
    fn only_a_minted_board_name_parses() {
        let id =
            Uuid::parse_str("d9792af0-71b8-4952-8aac-94bc0fead5f7").unwrap();
        let simple = id.as_simple().to_string();
        for refused in [
            String::new(),
            DKG_BOARD_PREFIX.to_string(),
            simple.clone(),
            format!("tally_{simple}"),
            format!("DKG_{simple}"),
            format!("dkg_{}", id.hyphenated()),
            format!("dkg_{}", id.braced()),
            format!("dkg_{}", simple.to_uppercase()),
            format!("dkg_{}", &simple[1..]),
            format!("dkg_{simple}0"),
            format!("dkg_{simple}\n"),
            format!("dkg_{simple}/../x"),
            "../x".to_string(),
            format!("dkg_{simple}_1"),
            format!("tally_{simple}_"),
            format!("tally__{simple}"),
            format!("tally_{simple}_+1"),
            format!("tally_{simple}_01"),
            format!("tally_{simple}_-0"),
            format!("tally_{simple}_1_2"),
            format!("tally_{simple}_1 "),
            format!("tally_{simple}_1\n"),
            format!("tally_{simple}_one"),
            format!("tally_{simple}_99999999999999999999"),
            format!("tally_{}_1", id.hyphenated()),
            format!("tally_{}_1", simple.to_uppercase()),
            format!("tally_{}_1", &simple[1..]),
            format!("TALLY_{simple}_1"),
            format!("tally_{simple}_1/../x"),
        ] {
            assert!(BoardName::parse(&refused).is_err(), "{refused:?}");
        }
    }

    #[test]
    fn a_board_name_is_parsed_on_its_way_in_from_json() {
        let dkg = BoardName::for_dkg(&Uuid::new_v4());
        let json = serde_json::to_value(&dkg).unwrap();
        assert_eq!(json, serde_json::json!(dkg.as_str()));
        assert_eq!(serde_json::from_value::<BoardName>(json).unwrap(), dkg);

        for refused in ["", "../x"] {
            let json = serde_json::json!(refused);
            assert!(serde_json::from_value::<BoardName>(json).is_err());
        }
    }
}
