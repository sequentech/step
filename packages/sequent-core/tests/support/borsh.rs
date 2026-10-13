// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Stream contracts shared by generated policy and ballot tests. Expected wire
//! bytes belong in each test case; error expectations come from this test sink.

use borsh::{BorshDeserialize, BorshSerialize};
use std::fmt::Debug;
use std::io::{self, Write};

struct FailingWriter {
    remaining: usize,
}

impl Write for FailingWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.is_empty() {
            return Ok(0);
        }
        if self.remaining == 0 {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "synthetic sink failure",
            ));
        }
        let written = self.remaining.min(bytes.len());
        self.remaining -= written;
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub fn assert_writer_contract<T: BorshSerialize>(value: &T, bytes: &[u8]) {
    assert!(!bytes.is_empty(), "fixture must exercise a nonempty record");
    assert_eq!(borsh::to_vec(value).unwrap(), bytes);
    for boundary in 0..bytes.len() {
        let error = value
            .serialize(&mut FailingWriter {
                remaining: boundary,
            })
            .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
        assert_eq!(error.to_string(), "synthetic sink failure");
    }
    // The same sink accepts the complete record when it has enough capacity.
    value
        .serialize(&mut FailingWriter {
            remaining: bytes.len(),
        })
        .unwrap();
}

pub fn assert_stream_contract<T>(value: &T, bytes: &[u8])
where
    T: BorshSerialize + BorshDeserialize + PartialEq + Debug,
{
    assert_writer_contract(value, bytes);
    assert_eq!(&borsh::from_slice::<T>(bytes).unwrap(), value);
    // Every proper prefix is incomplete, including cuts inside strings,
    // optional values and nested records. A default must not repair truncation.
    for boundary in 0..bytes.len() {
        assert!(
            borsh::from_slice::<T>(&bytes[..boundary]).is_err(),
            "accepted a record truncated at byte {boundary}"
        );
    }
    let mut trailing = bytes.to_vec();
    trailing.push(0);
    assert!(borsh::from_slice::<T>(&trailing).is_err());
}

/// Crypto values use wbraid canonical encoding and reject extra or missing bytes.
pub fn assert_canonical_contract<T>(value: &T, bytes: &[u8])
where
    T: cryptography::utils::serialization::Deserializable + PartialEq + Debug,
{
    assert_eq!(&T::deser(bytes).unwrap(), value);
    let mut extended = bytes.to_vec();
    extended.push(0);
    assert!(T::deser(&extended).is_err());
    assert!(T::deser(&bytes[..bytes.len() - 1]).is_err());
}
