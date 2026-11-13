// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The composed datalog program (§7.1, §7.6): splices the prelude and the
//! per-phase inference rules into a single ascent program, for use by the
//! trustee application.
//!
//! All types the rule templates reference (the [`Predicate`] enum and its
//! [`Slot`] trait, [`Action`], [`AccumulatorSet`], and the [`types`](super::types)
//! aliases) must be in scope at this expansion site.
//!
//! [`Predicate`]: crate::messages::predicate::Predicate
//! [`Slot`]: crate::messages::predicate::Slot

use super::accumulator::AccumulatorSet;
use super::types::*;
use super::{Action, MixSource};
use crate::messages::predicate::Predicate;

ascent::ascent! {
    include_source!(crate::datalog::prelude);
    include_source!(crate::datalog::dkg::infer::dkg_infer);
    include_source!(crate::datalog::mix::infer::mix_infer);
    include_source!(crate::datalog::decrypt::infer::decrypt_infer);
}

/// Run the datalog engine over a board's verified predicate set (§7.4).
///
/// Loads `predicates` as the EDB, runs the composed rules to fixpoint, and
/// returns the derived [`Action`]s. If any `error` fact was derived — an
/// equivocation, a configuration-domain violation, a public-key mismatch, a
/// malformed mix chain, ... — execution for this board must halt, so the
/// errors are returned as `Err` instead of actions.
pub fn run(predicates: &[Predicate]) -> Result<Vec<Action>, String> {
    let mut prog = AscentProgram {
        predicate: predicates.iter().map(|p| (p.clone(),)).collect(),
        ..Default::default()
    };
    prog.run();

    if !prog.error.is_empty() {
        let errors: Vec<String> = prog.error.into_iter().map(|e| e.0).collect();
        return Err(format!("datalog reported errors: {:?}", errors));
    }

    Ok(prog.action.into_iter().map(|a| a.0).collect())
}

#[cfg(test)]
mod tests {
    use super::run;
    use crate::datalog::Action;
    use crate::messages::newtypes::{
        hash_bytes, zero_hash, CiphertextsHash, ConfigurationHash, PartialDecryptionHash,
        PublicKeyHash, SharesHash, TrusteeIndex,
    };
    use crate::messages::predicate::{
        Ballots, ConfigurationValid, Mix, MixSignature, PartialDecryptions, Predicate, PublicKey,
        Shares,
    };

    /// A lone `ConfigurationValid` predicate should make the trustee compute its
    /// DKG shares: the first action of the protocol.
    #[test]
    fn configuration_valid_triggers_compute_shares() {
        let cfg = ConfigurationHash(zero_hash());
        let predicates = vec![Predicate::ConfigurationValid(ConfigurationValid {
            configuration: cfg,
            threshold: 2,
            trustee_count: 3,
            self_index: 1,
        })];

        let actions = run(&predicates).expect("well-formed input must not error");

        assert!(
            actions
                .iter()
                .any(|a| matches!(a, Action::ComputeShares(c, 1) if *c == cfg)),
            "expected ComputeShares(cfg, 1), got {:?}",
            actions
        );
    }

    /// A `ConfigurationValid` (threshold 2 of 3, self index 1) plus a `Ballots`
    /// naming the given mixing trustees.
    fn config_and_ballots(trustees: Vec<TrusteeIndex>) -> Vec<Predicate> {
        let cfg = ConfigurationHash(zero_hash());
        vec![
            Predicate::ConfigurationValid(ConfigurationValid {
                configuration: cfg,
                threshold: 2,
                trustee_count: 3,
                self_index: 1,
            }),
            Predicate::Ballots(Ballots {
                configuration: cfg,
                public_key: PublicKeyHash(zero_hash()),
                ciphertexts: CiphertextsHash(zero_hash()),
                trustees,
                tally_id: 1,
            }),
        ]
    }

    /// The ballots mixing-trustee list is the decryption quorum, so its size
    /// must be exactly the threshold: a shorter or longer list is a malformed
    /// manager input and must halt the protocol.
    #[test]
    fn mixing_set_size_must_match_threshold() {
        run(&config_and_ballots(vec![1, 2])).expect("a threshold-sized mixing set must not error");

        for trustees in [vec![1], vec![1, 2, 3]] {
            let err = run(&config_and_ballots(trustees.clone()))
                .expect_err(&format!("mixing set {:?} must error", trustees));
            assert!(
                err.contains("mixing set size"),
                "expected a mixing-set size error, got: {err}"
            );
        }
    }

    /// Every entry of the ballots mixing-trustee list must name an existing
    /// trustee (1-based, at most trustee_count): a nonexistent index is a
    /// malformed manager input and must halt rather than stall the mix chain.
    #[test]
    fn mixing_trustee_index_must_be_in_range() {
        for trustees in [vec![0, 1], vec![1, 9]] {
            let err = run(&config_and_ballots(trustees.clone()))
                .expect_err(&format!("mixing set {:?} must error", trustees));
            assert!(
                err.contains("out of range"),
                "expected an out-of-range error, got: {err}"
            );
        }
    }

    /// A `ConfigurationValid` for trustee 1 of 3, threshold 2.
    fn trustee_1_of_3() -> Predicate {
        Predicate::ConfigurationValid(ConfigurationValid {
            configuration: ConfigurationHash(zero_hash()),
            threshold: 2,
            trustee_count: 3,
            self_index: 1,
        })
    }

    /// The `Shares` of trustees 1, 2 and 3, with these hashes.
    fn dealings(hashes: [SharesHash; 3]) -> Vec<Predicate> {
        let mut predicates = vec![trustee_1_of_3()];
        for (sender, shares) in (1..).zip(hashes) {
            predicates.push(Predicate::Shares(Shares {
                configuration: ConfigurationHash(zero_hash()),
                shares,
                sender,
            }));
        }
        predicates
    }

    /// The trustees' shares are all in once every trustee has dealt.
    #[test]
    fn distinct_dealings_complete_the_shares() {
        let hashes = [b"a", b"b", b"c"].map(|label| SharesHash(hash_bytes(label)));

        let actions = run(&dealings(hashes)).expect("distinct dealings must not error");

        assert!(
            actions
                .iter()
                .any(|a| matches!(a, Action::ComputePublicKey(_, s, 1) if *s == hashes)),
            "expected ComputePublicKey over the three dealings, got {:?}",
            actions
        );
    }

    /// A trustee that posts another trustee's dealing as its own halts the
    /// DKG with an error; it used to panic in the shares accumulator.
    #[test]
    fn a_copied_dealing_halts_the_dkg() {
        let [first, second] = [b"a", b"b"].map(|label| SharesHash(hash_bytes(label)));

        let err = run(&dealings([first, second, first])).expect_err("a copied dealing must error");

        assert!(
            err.contains("posted by trustees 1 and 3"),
            "expected a copied-shares error, got: {err}"
        );
    }

    /// The predicates of a completed mix chain of ballots `M0` by trustees 1
    /// then 2 (M0 -> M1 -> M2), each mix signed by the other trustee, under a
    /// public key every trustee posted, plus the trustees' partial decryptions
    /// of M2 with these hashes.
    fn decrypted_mix(decryptions: [PartialDecryptionHash; 2]) -> Vec<Predicate> {
        let cfg = ConfigurationHash(zero_hash());
        let pk = PublicKeyHash(hash_bytes(b"pk"));
        let [ballots, first_mix, second_mix] =
            [b"M0", b"M1", b"M2"].map(|label| CiphertextsHash(hash_bytes(label)));
        let mut predicates = vec![trustee_1_of_3()];
        for sender in 1..=3 {
            predicates.push(Predicate::PublicKey(PublicKey {
                configuration: cfg,
                public_key: pk,
                sender,
            }));
        }
        predicates.push(Predicate::Ballots(Ballots {
            configuration: cfg,
            public_key: pk,
            ciphertexts: ballots,
            trustees: vec![1, 2],
            tally_id: 1,
        }));
        for (input, output, mixer, signer) in
            [(ballots, first_mix, 1, 2), (first_mix, second_mix, 2, 1)]
        {
            predicates.push(Predicate::Mix(Mix {
                configuration: cfg,
                public_key: pk,
                input,
                output,
                sender: mixer,
            }));
            predicates.push(Predicate::MixSignature(MixSignature {
                configuration: cfg,
                public_key: pk,
                input,
                output,
                sender: signer,
            }));
        }
        for (sender, decryptions) in (1..).zip(decryptions) {
            predicates.push(Predicate::PartialDecryptions(PartialDecryptions {
                configuration: cfg,
                public_key: pk,
                ciphertexts: second_mix,
                decryptions,
                sender,
            }));
        }
        predicates
    }

    /// The mixing trustees' partial decryptions are combined once both are in.
    #[test]
    fn distinct_partial_decryptions_are_combined() {
        let hashes = [b"d1", b"d2"].map(|label| PartialDecryptionHash(hash_bytes(label)));

        let actions =
            run(&decrypted_mix(hashes)).expect("distinct partial decryptions must not error");

        assert!(
            actions
                .iter()
                .any(|a| matches!(a, Action::ComputePlaintexts(_, _, _, d, 1) if *d == hashes)),
            "expected ComputePlaintexts over both partial decryptions, got {:?}",
            actions
        );
    }

    /// A mixing trustee that posts another trustee's partial decryptions as
    /// its own halts the tally with an error; it used to panic in the partial
    /// decryptions accumulator.
    #[test]
    fn copied_partial_decryptions_halt_the_tally() {
        let copied = PartialDecryptionHash(hash_bytes(b"d1"));

        let err = run(&decrypted_mix([copied, copied]))
            .expect_err("copied partial decryptions must error");

        assert!(
            err.contains("posted by trustees 1 and 2"),
            "expected a copied partial decryptions error, got: {err}"
        );
    }
}
