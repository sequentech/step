// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! What a cast vote carries for the tally: one Naor-Yung ciphertext and the
//! contests it answers.
//!
//! A cast vote's `contests` entries are each the standard base64 (padded) of
//! the canonical bytes of one [`BallotCiphertext`]: one entry per contest when
//! every contest is encrypted on its own, one entry naming every contest of
//! the ballot when the ballot is encrypted whole. The voter's client produces
//! them and the tally reads them back; the ciphertext is encrypted under the
//! joint key augmented with the ballot encryption context, and its
//! well-formedness proof is checked by the trustees (i.e. not here).

use anyhow::{Context as _, Result};
use base64::engine::general_purpose::{GeneralPurpose, STANDARD};
use base64::Engine as _;
use cryptography::cryptosystem::naoryung;
use cryptography::utils::serialization::{Deserializable, Serializable};
use cryptography::Canonical;

use crate::configuration::CIPHERTEXT_WIDTH;
use crate::Ctx;

/// A cast vote's contest entry.
const BALLOT_BASE64: GeneralPurpose = STANDARD;

/// One encrypted ballot, or one contest of it, as a cast vote stores it.
#[derive(Debug, Clone, PartialEq, Canonical)]
pub struct BallotCiphertext {
    /// The contests the ciphertext answers.
    pub contest_ids: Vec<String>,
    ciphertext: naoryung::Ciphertext<Ctx, CIPHERTEXT_WIDTH>,
}

impl BallotCiphertext {
    pub fn new(
        contest_ids: Vec<String>,
        ciphertext: naoryung::Ciphertext<Ctx, CIPHERTEXT_WIDTH>,
    ) -> BallotCiphertext {
        BallotCiphertext {
            contest_ids,
            ciphertext,
        }
    }

    /// The entry as a cast vote stores it.
    pub fn encode(&self) -> String {
        BALLOT_BASE64.encode(self.ser())
    }

    /// A cast vote's stored entry.
    pub fn parse(encoded: &str) -> Result<BallotCiphertext> {
        let bytes = BALLOT_BASE64
            .decode(encoded)
            .context("a ballot ciphertext that is not base64")?;
        BallotCiphertext::deser(&bytes)
            .context("a ballot ciphertext that cannot be deserialized")
    }

    pub(crate) fn into_ciphertext(
        self,
    ) -> naoryung::Ciphertext<Ctx, CIPHERTEXT_WIDTH> {
        self.ciphertext
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::engine::general_purpose::STANDARD_NO_PAD;
    use cryptography::context::Context as _;
    use cryptography::cryptosystem::elgamal;

    fn ballot(contest_ids: &[&str]) -> BallotCiphertext {
        let key = naoryung::PublicKey::augment(
            &elgamal::PublicKey::new(Ctx::random_element()),
            b"ballot encryption context",
        )
        .unwrap();
        let ciphertext = key
            .encrypt(&[Ctx::random_element()], b"ballot encryption context")
            .unwrap();
        BallotCiphertext::new(
            contest_ids.iter().map(|id| id.to_string()).collect(),
            ciphertext,
        )
    }

    #[test]
    fn a_stored_entry_reads_back_as_the_ballot_it_was_written_from() {
        for contest_ids in [vec!["contest"], vec!["one", "two"], vec![]] {
            let ballot = ballot(&contest_ids);
            let stored = ballot.encode();
            assert_eq!(BallotCiphertext::parse(&stored).unwrap(), ballot);
        }
    }

    #[test]
    fn the_stored_entry_is_padded_base64_of_the_canonical_bytes() {
        // The contract the voter's client meets without this crate's encoder.
        let ballot = ballot(&["contest"]);
        let canonical = ballot.ser();
        assert_eq!(ballot.encode(), STANDARD.encode(&canonical));
        assert_eq!(
            BallotCiphertext::parse(&STANDARD.encode(&canonical)).unwrap(),
            ballot
        );
    }

    #[test]
    fn an_entry_that_is_not_one_ballot_is_refused() {
        // A length that needs padding, so that the unpadded form differs.
        let canonical = ["c", "cc"]
            .into_iter()
            .map(|contest_id| ballot(&[contest_id]).ser())
            .find(|bytes| bytes.len() % 3 != 0)
            .unwrap();
        let mut longer = canonical.clone();
        longer.push(0);
        for refused in [
            String::new(),
            "not base64!".to_string(),
            STANDARD_NO_PAD.encode(&canonical),
            STANDARD.encode(&canonical[..canonical.len() - 1]),
            STANDARD.encode(longer),
        ] {
            assert!(BallotCiphertext::parse(&refused).is_err(), "{refused:?}");
        }
    }
}
