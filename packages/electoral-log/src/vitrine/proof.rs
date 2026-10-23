// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Receipts, promises and receipt chains (Vitrine's `specs/receipt.md`), as
//! far as step reads and writes them. Step never verifies them: the booth and
//! the verifier do, with `@vitrine/verify`.

use anyhow::{anyhow, bail, ensure, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use std::collections::BTreeSet;
use std::fmt::Write as _;

use super::entry::{leaf_hash, Hash};
use super::statement::check_policy_id;

pub const RECEIPT_TYPE: &str = "c2sp.org/tlog-proof@v1";
pub const PROMISE_TYPE: &str = "vitrine/promise/v1";
pub const CHAIN_TYPE: &str = "vitrine/receipt-chain/v1";
pub const MAX_BLOCK_SIZE: usize = 1 << 20;
pub const MAX_CHAIN_SIZE: usize = 4 << 20;

pub const BLOCK_POLICY_ACK: &str = "policy-ack";
pub const BLOCK_PROMISE: &str = "promise";
pub const BLOCK_ISSUED: &str = "issued";
pub const BLOCK_RECORD: &str = "record";

/// What a voter's portal holds after a cast: a receipt, whose `extra` line
/// is the entry, or a promise, which names the leaf hash.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Submitted {
    Receipt { entry: Vec<u8> },
    Promise { leaf: Hash },
}

fn line_value<'a>(line: Option<&'a str>, key: &str) -> Result<&'a str> {
    line.and_then(|l| l.strip_prefix(key))
        .and_then(|l| l.strip_prefix(' '))
        .ok_or_else(|| anyhow!("expected a {key} line"))
}

impl Submitted {
    pub fn parse(text: &str) -> Result<Self> {
        let mut lines = text.split('\n');
        match lines.next() {
            Some(RECEIPT_TYPE) => {
                let extra = line_value(lines.next(), "extra")?;
                let entry = STANDARD.decode(extra).context("extra is base64")?;
                Ok(Submitted::Receipt { entry })
            }
            Some(PROMISE_TYPE) => {
                let leaf = line_value(lines.next(), "leaf")?;
                let leaf = STANDARD.decode(leaf).context("leaf is base64")?;
                let leaf: Hash = leaf
                    .try_into()
                    .map_err(|_| anyhow!("a leaf hash has 32 bytes"))?;
                Ok(Submitted::Promise { leaf })
            }
            _ => bail!("neither a receipt nor a promise"),
        }
    }

    pub fn leaf(&self) -> Hash {
        match self {
            Submitted::Receipt { entry } => leaf_hash(entry),
            Submitted::Promise { leaf } => *leaf,
        }
    }

    /// Whether this is for one of `entries`: a receipt whose entry is one of
    /// them byte for byte, or a promise for one of their leaf hashes.
    pub fn is_for_one_of(&self, entries: &[Vec<u8>]) -> bool {
        match self {
            Submitted::Receipt { entry } => entries.iter().any(|e| e == entry),
            Submitted::Promise { leaf } => entries.iter().any(|e| leaf_hash(e) == *leaf),
        }
    }
}

/// One block of a receipt chain.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChainBlock<'a> {
    pub kind: &'a str,
    pub args: Vec<&'a str>,
    pub data: &'a str,
}

/// A `vitrine/receipt-chain/v1` document for the log named `log` in the
/// policy `policy_id`.
pub fn receipt_chain(policy_id: &str, log: &str, blocks: &[ChainBlock]) -> Result<String> {
    check_policy_id(policy_id)?;
    ensure!(
        !log.is_empty() && !log.contains(char::is_whitespace),
        "a log name is one word"
    );
    ensure!(
        blocks
            .iter()
            .any(|b| matches!(b.kind, BLOCK_PROMISE | BLOCK_ISSUED | BLOCK_RECORD)),
        "a chain has a promise, an issued receipt or a record"
    );
    let mut seen = BTreeSet::new();
    let mut out = format!("{CHAIN_TYPE}\npolicy {policy_id}\nlog {log}\n");
    for b in blocks {
        ensure!(
            !b.kind.is_empty() && b.kind.bytes().all(|c| c.is_ascii_lowercase() || c == b'-'),
            "a block type is a-z and -"
        );
        ensure!(
            seen.insert((b.kind, b.args.first().copied())),
            "a chain has at most one {} block",
            b.kind
        );
        ensure!(b.data.len() <= MAX_BLOCK_SIZE, "a block has at most 1 MiB");
        out.push_str(b.kind);
        for arg in &b.args {
            ensure!(
                !arg.is_empty() && !arg.contains(char::is_whitespace),
                "a block argument is one word"
            );
            write!(out, " {arg}")?;
        }
        writeln!(out, " {}", b.data.len())?;
        out.push_str(b.data);
        out.push('\n');
    }
    ensure!(out.len() <= MAX_CHAIN_SIZE, "a chain has at most 4 MiB");
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vitrine::ElectionEntry;

    fn entry(ballot: u8) -> Vec<u8> {
        ElectionEntry {
            event_id: "e".into(),
            election_id: "mayor".into(),
            ballot_id: [ballot; 32],
            accepted_at: 10,
        }
        .encode()
        .unwrap()
    }

    fn receipt(entry: &[u8]) -> String {
        format!(
            "{RECEIPT_TYPE}\nextra {}\nindex 3\n{}\n\nlog.example/x\n4\nroot\n\n\u{2014} log.example/x sig\n",
            STANDARD.encode(entry),
            STANDARD.encode([9u8; 32])
        )
    }

    #[test]
    fn a_receipt_is_for_its_own_entry_only() {
        let mine = vec![entry(1), entry(2)];
        let ok = Submitted::parse(&receipt(&mine[1])).unwrap();
        assert!(ok.is_for_one_of(&mine));
        assert_eq!(ok.leaf(), leaf_hash(&mine[1]));
        let other = Submitted::parse(&receipt(&entry(3))).unwrap();
        assert!(!other.is_for_one_of(&mine));
    }

    #[test]
    fn a_promise_is_matched_by_leaf_hash() {
        let mine = vec![entry(1)];
        let promise = |leaf: Hash| {
            format!(
                "{PROMISE_TYPE}\nleaf {}\nindex 0\n\nvitrine/promise-batch/v1\n",
                STANDARD.encode(leaf)
            )
        };
        let ok = Submitted::parse(&promise(leaf_hash(&mine[0]))).unwrap();
        assert!(ok.is_for_one_of(&mine));
        let other = Submitted::parse(&promise([0; 32])).unwrap();
        assert!(!other.is_for_one_of(&mine));
    }

    #[test]
    fn other_documents_are_refused() {
        assert!(Submitted::parse("vitrine/receipt-chain/v1\npolicy x\n").is_err());
        assert!(Submitted::parse(&format!("{RECEIPT_TYPE}\nindex 3\n")).is_err());
        assert!(Submitted::parse(&format!("{PROMISE_TYPE}\nleaf AAAA\n")).is_err());
        assert!(Submitted::parse("").is_err());
    }

    #[test]
    fn chains_frame_each_block_with_its_length() {
        let policy = format!("sha256:{}", "0f".repeat(32));
        let chain = receipt_chain(
            &policy,
            "events",
            &[
                ChainBlock {
                    kind: BLOCK_POLICY_ACK,
                    args: vec![],
                    data: "ack\n",
                },
                ChainBlock {
                    kind: BLOCK_RECORD,
                    args: vec![],
                    data: "réc\n",
                },
            ],
        )
        .unwrap();
        assert_eq!(
            chain,
            format!(
                "{CHAIN_TYPE}\npolicy {policy}\nlog events\npolicy-ack 4\nack\n\nrecord 5\nréc\n\n"
            )
        );
        let ack_only = [ChainBlock {
            kind: BLOCK_POLICY_ACK,
            args: vec![],
            data: "ack\n",
        }];
        assert!(receipt_chain(&policy, "events", &ack_only).is_err());
        let twice = [
            ChainBlock {
                kind: BLOCK_RECORD,
                args: vec![],
                data: "a",
            },
            ChainBlock {
                kind: BLOCK_RECORD,
                args: vec![],
                data: "b",
            },
        ];
        assert!(receipt_chain(&policy, "events", &twice).is_err());
    }
}
