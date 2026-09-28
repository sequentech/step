// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! How a board and its protocol domain are named.

use std::fmt;

use uuid::Uuid;

/// Board names of a keys ceremony's distributed key generation start with
/// this.
const DKG_BOARD_PREFIX: &str = "dkg_";

/// The name of one board on the board service.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoardName(String);

impl BoardName {
    /// The DKG board of a key ceremony.
    pub fn for_dkg(keys_ceremony_id: &Uuid) -> BoardName {
        BoardName(format!(
            "{DKG_BOARD_PREFIX}{}",
            keys_ceremony_id.as_simple()
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
}
