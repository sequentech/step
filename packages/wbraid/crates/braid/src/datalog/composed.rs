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
        Ballots, ConfigurationValid, PartialDecryptions, Predicate, Shares,
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

    /// Two trustees posting the same dealing body share one content hash, which
    /// the shares accumulator cannot hold twice. The slots differ, so `collides`
    /// does not catch it, and it must still halt the protocol.
    #[test]
    fn copied_shares_halt_the_protocol() {
        let cfg = ConfigurationHash(zero_hash());
        let copied = SharesHash(hash_bytes(b"dealing"));
        let mut predicates = config_and_ballots(vec![1, 2]);
        predicates.extend((1..=2).map(|sender| {
            Predicate::Shares(Shares {
                configuration: cfg,
                shares: copied,
                sender,
            })
        }));

        let err = run(&predicates).expect_err("copied shares must error");

        assert!(
            err.contains("shares accumulator"),
            "expected a shares accumulator error, got: {err}"
        );
    }

    /// A mixing set that names the same trustee twice puts that trustee's one
    /// partial decryption at two positions. The mixing-set rules report it, and
    /// accumulating the partial decryptions must report it too rather than abort.
    #[test]
    fn repeated_partial_decryption_halts_the_protocol() {
        let cfg = ConfigurationHash(zero_hash());
        let mut predicates = config_and_ballots(vec![1, 1]);
        predicates.push(Predicate::PartialDecryptions(PartialDecryptions {
            configuration: cfg,
            public_key: PublicKeyHash(zero_hash()),
            ciphertexts: CiphertextsHash(zero_hash()),
            decryptions: PartialDecryptionHash(hash_bytes(b"factors")),
            sender: 1,
        }));

        let err = run(&predicates).expect_err("a repeated partial decryption must error");

        assert!(
            err.contains("partial decryptions accumulator"),
            "expected a partial decryptions accumulator error, got: {err}"
        );
    }
}
