// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use b3::messages::newtypes::{ChannelsHashes, DecryptionFactorsHashes, SharesHashes};
use borsh::{BorshDeserialize, BorshSerialize};
use std::io::{self, Write};

// The protocol encodes twelve length-prefixed, 64-byte hashes. Deliberately
// construct the expected wire bytes without the serializer being tested.
fn wire_hashes(count: u32, width: u32) -> Vec<u8> {
    let mut bytes = count.to_le_bytes().to_vec();
    for index in 0..count {
        bytes.extend(width.to_le_bytes());
        bytes.extend(vec![index as u8; width as usize]);
    }
    bytes
}

struct BoundedWriter(Vec<u8>, usize);
impl Write for BoundedWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.0.len() == self.1 {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "fixture full",
            ));
        }
        let count = bytes.len().min(self.1 - self.0.len());
        self.0.extend_from_slice(&bytes[..count]);
        Ok(count)
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

macro_rules! hash_contracts {
    ($module:ident, $ty:ident) => {
        mod $module {
            use super::*;
            #[test]
            fn fixed_wire_vector_and_full_capacity_writer_are_accepted() {
                let bytes = wire_hashes(12, 64);
                let expected = std::array::from_fn(|index| [index as u8; 64]);
                let value = $ty::try_from_slice(&bytes).unwrap();
                assert_eq!(value.0, expected);
                assert_eq!(borsh::to_vec(&value).unwrap(), bytes);
                let mut output = BoundedWriter(vec![], bytes.len());
                value.serialize(&mut output).unwrap();
                assert_eq!(output.0, bytes);
            }
            #[test]
            fn wrong_number_of_hashes_returns_an_error_without_panicking() {
                // Each input changes only the outer count of a valid wire shape.
                for count in [0, 11, 13] {
                    let result = $ty::try_from_slice(&wire_hashes(count, 64));
                    assert!(result.is_err(), "accepted {count} hashes");
                }
            }
            #[test]
            fn wrong_hash_width_truncation_and_trailing_bytes_are_rejected() {
                for width in [0, 63, 65] {
                    assert!($ty::try_from_slice(&wire_hashes(12, width)).is_err());
                }
                let bytes = wire_hashes(12, 64);
                for end in [0, 3, 7, bytes.len() - 1] {
                    assert!($ty::try_from_slice(&bytes[..end]).is_err());
                }
                let mut trailing = bytes;
                trailing.push(0);
                assert!($ty::try_from_slice(&trailing).is_err());
            }
            #[test]
            fn partial_writer_failure_is_preserved() {
                let value = $ty(std::array::from_fn(|index| [index as u8; 64]));
                let mut output = BoundedWriter(vec![], 9);
                let error = value.serialize(&mut output).unwrap_err();
                assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
                assert_eq!(output.0, wire_hashes(12, 64)[..9]);
            }
        }
    };
}
hash_contracts!(channels, ChannelsHashes);
hash_contracts!(shares, SharesHashes);
hash_contracts!(decryption_factors, DecryptionFactorsHashes);
