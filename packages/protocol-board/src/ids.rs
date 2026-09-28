// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! How a board and its protocol domain are named.

use std::fmt;

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Board names of a keys ceremony's distributed key generation start with
/// this.
const DKG_BOARD_PREFIX: &str = "dkg_";

/// The longest board name the board service accepts, in bytes.
const MAX_BOARD_NAME_BYTES: usize = 255;

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

    /// A board name as the board service accepts it: not empty, at most 255
    /// bytes, and nothing but alphanumerics, `-` and `_`.
    pub fn parse(name: &str) -> Result<BoardName> {
        if name.is_empty() {
            bail!("a board name cannot be empty");
        }
        if name.len() > MAX_BOARD_NAME_BYTES {
            bail!(
                "board name {name:?} is longer than {MAX_BOARD_NAME_BYTES} \
                 bytes"
            );
        }
        if !name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            bail!(
                "board name {name:?} holds characters other than \
                 alphanumerics, '-' and '_'"
            );
        }
        Ok(BoardName(name.to_string()))
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
    fn a_configuration_id_names_its_ceremony() {
        let id = Uuid::new_v4();
        assert_eq!(Uuid::from_u128(configuration_id(&id)), id);
    }

    #[test]
    fn a_board_name_is_what_the_board_service_accepts() {
        // The board service counts the length in bytes, not in characters.
        let longest = "a".repeat(255);
        let longest_accented = "é".repeat(127);
        for accepted in [
            "dkg-1_tally",
            "a",
            longest.as_str(),
            longest_accented.as_str(),
            "tallyÑ2",
        ] {
            assert_eq!(BoardName::parse(accepted).unwrap().as_str(), accepted);
        }

        let too_long = "a".repeat(256);
        let too_long_accented = "é".repeat(128);
        for refused in [
            "",
            too_long.as_str(),
            too_long_accented.as_str(),
            "a/b",
            "a b",
            "a.b",
            "../x",
            "dkg\n",
            "a%2Fb",
        ] {
            assert!(BoardName::parse(refused).is_err(), "{refused:?}");
        }

        let dkg = BoardName::for_dkg(&Uuid::new_v4());
        assert_eq!(BoardName::parse(dkg.as_str()).unwrap(), dkg);
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
