// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::ballot::Contest;
use phf::phf_map;
use std::str;

impl Contest {
    /// Returns the appropriate character map for this contest.
    ///
    /// # Returns
    /// A boxed trait object implementing `CharacterMap`.
    #[must_use]
    pub fn get_char_map(&self) -> Box<dyn CharacterMap> {
        if self.base32_writeins() {
            Box::new(Base32Map)
        } else {
            Box::new(Utf8Map)
        }
    }
}

/// Trait for mapping between characters and bytes for ballot encoding.
pub trait CharacterMap {
    /// Converts a string to a vector of bytes according to the map.
    ///
    /// # Errors
    /// Returns an error if the string contains characters that cannot be mapped.
    fn to_bytes(&self, s: &str) -> Result<Vec<u8>, String>;

    /// Converts a byte slice to a string according to the map.
    ///
    /// # Errors
    /// Returns an error if the byte slice contains bytes that cannot be mapped.
    fn to_string(&self, bytes: &[u8]) -> Result<String, String>;

    /// Returns the base (number of possible values) for this character map.
    fn base(&self) -> u64;
}

/// UTF-8 character map for ballot encoding.
pub struct Utf8Map;
/// Base32 character map for ballot encoding.
pub struct Base32Map;

impl CharacterMap for Utf8Map {
    /// Converts a string to a vector of UTF-8 bytes.
    ///
    /// # Errors
    /// This implementation never returns an error.
    fn to_bytes(&self, s: &str) -> Result<Vec<u8>, String> {
        Ok(s.as_bytes().to_vec())
    }

    /// Converts a UTF-8 byte slice to a string.
    ///
    /// # Errors
    /// Returns an error if the bytes are not valid UTF-8.
    fn to_string(&self, bytes: &[u8]) -> Result<String, String> {
        str::from_utf8(bytes)
            .map_err(|e| e.to_string())
            .map(str::to_string)
    }

    /// Returns the base for UTF-8 (256).
    fn base(&self) -> u64 {
        256u64
    }
}

impl CharacterMap for Base32Map {
    /// Converts a string to a vector of Base32-mapped bytes.
    ///
    /// # Errors
    /// Returns an error if the string contains characters that cannot be mapped.
    fn to_bytes(&self, s: &str) -> Result<Vec<u8>, String> {
        s.to_uppercase()
            .chars()
            .map(|c| {
                TO_BYTE.get(&c).copied().ok_or_else(|| {
                    format!("Character '{c}' cannot be mapped to byte")
                })
            })
            .collect()
    }

    /// Converts a Base32-mapped byte slice to a string.
    ///
    /// # Errors
    /// Returns an error if the bytes cannot be mapped to characters.
    fn to_string(&self, bytes: &[u8]) -> Result<String, String> {
        let chars: Result<Vec<char>, String> = bytes
            .iter()
            .map(|&b| {
                TO_CHAR.get(&b).copied().ok_or_else(|| {
                    format!("Byte '{b}' cannot be mapped to char")
                })
            })
            .collect();
        Ok(String::from_iter(chars?))
    }

    /// Returns the base for Base32 (32).
    fn base(&self) -> u64 {
        32u64
    }
}

/// PHF map from characters to bytes for Base32 encoding.
pub static TO_BYTE: phf::Map<char, u8> = phf_map! {
    // 0 is reserved for null terminator
    'A' => 1u8,
    'B' => 2u8,
    'C' => 3u8,
    'D' => 4u8,
    'E' => 5u8,
    'F' => 6u8,
    'G' => 7u8,
    'H' => 8u8,
    'I' => 9u8,
    'J' => 10u8,
    'K' => 11u8,
    'L' => 12u8,
    'M' => 13u8,
    'N' => 14u8,
    'O' => 15u8,
    'P' => 16u8,
    'Q' => 17u8,
    'R' => 18u8,
    'S' => 19u8,
    'T' => 20u8,
    'U' => 21u8,
    'V' => 22u8,
    'W' => 23u8,
    'X' => 24u8,
    'Y' => 25u8,
    'Z' => 26u8,
    ' ' => 27u8,
    '(' => 28u8,
    ')' => 29u8,
    '.' => 30u8,
    ',' => 31u8,
};
/// PHF map from bytes to characters for Base32 encoding.
pub static TO_CHAR: phf::Map<u8, char> = phf_map! {
    // 0 is reserved for null terminator
    1u8 => 'A',
    2u8 => 'B',
    3u8 => 'C',
    4u8 => 'D',
    5u8 => 'E',
    6u8 => 'F',
    7u8 => 'G',
    8u8 => 'H',
    9u8 => 'I',
    10u8 => 'J',
    11u8 => 'K',
    12u8 => 'L',
    13u8 => 'M',
    14u8 => 'N',
    15u8 => 'O',
    16u8 => 'P',
    17u8 => 'Q',
    18u8 => 'R',
    19u8 => 'S',
    20u8 => 'T',
    21u8 => 'U',
    22u8 => 'V',
    23u8 => 'W',
    24u8 => 'X',
    25u8 => 'Y',
    26u8 => 'Z',
    27u8 => ' ',
    28u8 => '(',
    29u8 => ')',
    30u8 => '.',
    31u8 => ',',
};

#[cfg(test)]
mod tests {
    use crate::ballot_codec::*;
    use crate::fixtures::ballot_codec::get_configurable_contest;
    use crate::fixtures::ballot_codec::get_contest_candidates_n;
    use crate::plaintext::{DecodedVoteChoice, DecodedVoteContest};
    use crate::types::ceremonies::CountingAlgType;
    use rand::Rng;

    #[test]
    fn test_character_maps() {
        let map = Base32Map;
        for key in TO_BYTE.keys() {
            let s = key.to_string();
            let forward = map.to_bytes(&s).unwrap();
            let backward = map.to_string(&forward).unwrap();

            assert_eq!(s, backward);
        }
        let map = Utf8Map;
        // arbitrary range
        for i in 0u32..1024u32 {
            let char = char::from_u32(i);
            if char.is_some() {
                let char = char.unwrap().to_string();
                let forward = map.to_bytes(&char).unwrap();
                let backward = map.to_string(&forward).unwrap();

                assert_eq!(char, backward);
            }
        }
    }

    #[test]
    fn test_write_in_available() {
        const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ )(.,";
        const MAX_LEN: usize = 40;
        let mut rng = rand::thread_rng();

        let writein: String = (0..MAX_LEN)
            .map(|_| {
                let idx = rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect();

        let mut contest = get_contest_candidates_n(190);

        let choices: Vec<DecodedVoteChoice> = (0..190)
            .map(|i| DecodedVoteChoice {
                id: i.to_string(),
                selected: -1,
                write_in_text: None,
            })
            .collect();

        let vote = DecodedVoteContest {
            contest_id: contest.id.clone(),
            is_explicit_invalid: false,
            invalid_errors: vec![],
            invalid_alerts: vec![],
            choices,
        };

        let c = contest.available_write_in_characters(&vote);
        assert_eq!(c, Ok(232));

        let result = contest.encode_plaintext_contest_to_bytes(&vote).unwrap();
        let raw_ballot = contest.encode_to_raw_ballot(&vote).unwrap();

        println!("{:?} {:?}", raw_ballot.bases, raw_ballot.choices)
    }

    #[test]
    fn test_write_in_base32() {
        const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ )(.,";
        const MAX_LEN: usize = 40;
        let mut rng = rand::thread_rng();

        let writein: String = (0..MAX_LEN)
            .map(|_| {
                let idx = rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect();

        let mut contest = get_configurable_contest(
            1,
            3,
            CountingAlgType::PluralityAtLarge,
            true,
            Some(vec![2]),
            false,
        );

        let ballot = DecodedVoteContest {
            contest_id: contest.id.clone(),
            is_explicit_invalid: false,
            invalid_errors: vec![],
            invalid_alerts: vec![],
            choices: vec![
                DecodedVoteChoice {
                    id: 0.to_string(),
                    selected: 1,
                    write_in_text: None,
                },
                DecodedVoteChoice {
                    id: 1.to_string(),
                    selected: 1,
                    write_in_text: None,
                },
                /*DecodedVoteChoice {
                    id: 2.to_string(),
                    selected: 5,
                    write_in_text: None,
                },
                DecodedVoteChoice {
                    id: 3.to_string(),
                    selected: 3,
                    write_in_text: None,
                },*/
                DecodedVoteChoice {
                    id: 2.to_string(),
                    selected: 1,
                    //                   123456789012345679012345678901234567890
                    // write_in_text:
                    // Some("ZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZ".
                    // to_string()),
                    write_in_text: Some(writein.clone()),
                },
            ],
        };
        let result =
            contest.encode_plaintext_contest_to_bytes(&ballot).unwrap();
        let bytes_large = result.len();

        let mut presentation = contest.presentation.as_ref().unwrap().clone();
        presentation.base32_writeins = Some(true);
        contest.presentation = Some(presentation);

        let result =
            contest.encode_plaintext_contest_to_bytes(&ballot).unwrap();
        let bytes_small = result.len();

        let result = contest
            .decode_plaintext_contest_from_bytes(&result)
            .unwrap();
        println!("************* {:?} ************", result);
        let back = result.choices[2].write_in_text.as_ref().unwrap();
        assert_eq!(*back, writein);
        assert!(bytes_small < 27);
        assert!(bytes_small < bytes_large);
    }
}
