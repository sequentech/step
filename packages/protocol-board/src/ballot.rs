// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The cast-vote ciphertext contract shared with the voter.

pub type BallotCiphertext = voting_crypto::ballot::BallotCiphertext<crate::Ctx>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Ctx;
    use base64::engine::general_purpose::STANDARD_NO_PAD;
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    use cryptography::context::Context as _;
    use cryptography::cryptosystem::elgamal;
    use cryptography::cryptosystem::naoryung;
    use cryptography::utils::serialization::Serializable;

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
    fn the_stored_entry_is_padded_base64_of_the_contests_then_the_ciphertext() {
        // The contract the voter's client meets without this crate's encoder.
        let ballot = ballot(&["contest"]);
        let expected =
            [ballot.contest_ids.ser(), ballot.ciphertext.ser()].concat();
        assert_eq!(ballot.encode(), STANDARD.encode(&expected));
        assert_eq!(
            BallotCiphertext::parse(&STANDARD.encode(&expected)).unwrap(),
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
