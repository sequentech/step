// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Datalog engine (§7 of `crates/braid/v0.6_spec.md`): the trustee "brain".
//!
//! The engine consumes the set of verified [`Predicate`]s held by the board
//! (the EDB, produced by `MessageStore::get_predicates()`) and derives the
//! [`Action`]s the trustee must perform to advance the protocol — or an error
//! if the inputs are inconsistent (an equivocation, a public-key mismatch, a
//! malformed mix chain, ...).
//!
//! It is a faithful port of the vs_lift `ascent_logic` rules, with three
//! deliberate adaptations:
//! 1. the input relation carries our typed [`Predicate`] enum instead of the
//!    vs_lift `Message` enum, and the input-mapping rules destructure the
//!    named-field predicate structs (§7.2);
//! 2. the untyped `type X = CryptographicHash` aliases become the distinct,
//!    type-safe newtypes from `crate::messages::newtypes` (aliased in [`types`]
//!    purely as local shorthand for the rule templates);
//! 3. the `#[cfg(test)] mod stateright` model-checking harnesses are dropped —
//!    only the inference rules are ported.
//!
//! [`Predicate`]: crate::messages::predicate::Predicate

#![allow(dead_code)]

pub mod accumulator;
pub mod decrypt;
pub mod dkg;
pub mod mix;

/// Type shorthands used by the ascent rule templates.
///
/// The vs_lift rules were written against a family of `type … = CryptographicHash`
/// aliases; porting them verbatim would lose type safety. Here each alias points
/// at a *distinct* newtype from `crate::messages::newtypes`, so the rules keep their
/// original spelling while gaining compile-time type separation between, say, a
/// configuration hash and a public-key hash.
pub mod types {
    use super::accumulator::AccumulatorSet;

    pub use crate::messages::newtypes::{
        CiphertextsHash, ConfigurationHash, DecryptionFactorsHash, PlaintextsHash, PublicKeyHash,
        SharesHash, Threshold, TrusteeCount, TrusteeIndex,
    };

    /// Configuration hash (vs_lift `CfgHash`).
    pub type CfgHash = ConfigurationHash;
    /// Trustee shares hash (vs_lift `TrusteeSharesHash`).
    pub type TrusteeSharesHash = SharesHash;
    /// Partial decryptions hash (vs_lift `PartialDecryptionsHash`).
    pub type PartialDecryptionsHash = DecryptionFactorsHash;
    /// Message sender, a 1-based trustee index (vs_lift `Sender`).
    pub type Sender = TrusteeIndex;
    /// Ordered sequence of trustee shares hashes.
    pub type SharesHashes = Vec<SharesHash>;
    /// Accumulator of trustee shares hashes.
    pub type SharesHashesAcc = AccumulatorSet<SharesHash>;
    /// Ordered sequence of partial decryptions hashes.
    pub type PartialDecryptionsHashes = Vec<DecryptionFactorsHash>;
    /// Accumulator of partial decryptions hashes.
    pub type PartialDecryptionsHashesAcc = AccumulatorSet<DecryptionFactorsHash>;
}

use types::*;

/// Where a mix's input ciphertexts come from (§8). The datalog — which knows the
/// mixing position — tags each mix action with its source so the trustee fetches
/// from the correct store directly, instead of probing both. Because the store
/// accessors are content-addressed by the action's input hash, naming the wrong
/// source simply yields no body (an explicit error), which doubles as a sanity
/// check that the source and the input hash agree.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum MixSource {
    /// The manager's `Ballots` ciphertexts — the first mixer's input.
    Ballots,
    /// A previous mixer's `Mix` output ciphertexts — a later mixer's input.
    PriorMix,
}

/// Actions a trustee can take during protocol execution.
///
/// Each variant corresponds to a computation the trustee must perform; they are
/// derived by the ascent rules and consumed by the action layer, which performs
/// the underlying cryptography and posts the resulting message to the board,
/// advancing the protocol.
///
/// Unlike the vs_lift original this does **not** derive `Ord`: ascent relations
/// only require `Clone + Eq + Hash`, and dropping `Ord` avoids imposing an
/// ordering requirement on the configuration/public-key/ciphertexts hashes that
/// appear as fields (only the *accumulated* hashes need `Ord`, for the
/// [`accumulator::AccumulatorSet`]).
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Action {
    /// Compute and post this trustee's DKG shares.
    ComputeShares(CfgHash, TrusteeIndex),
    /// Compute and post this trustee's view of the joint public key.
    ComputePublicKey(CfgHash, SharesHashes, TrusteeIndex),
    /// Compute and post a mix of the input ciphertexts drawn from `MixSource`.
    ComputeMix(
        CfgHash,
        PublicKeyHash,
        MixSource,
        CiphertextsHash,
        TrusteeIndex,
    ),
    /// Verify and sign a mix (`input` -> `output`); `input` is drawn from `MixSource`.
    SignMix(
        CfgHash,
        PublicKeyHash,
        MixSource,
        CiphertextsHash,
        CiphertextsHash,
        TrusteeIndex,
    ),
    /// Compute and post partial decryptions of the given ciphertexts.
    ///
    /// Carries the accumulated DKG shares hashes explicitly (like
    /// [`Action::ComputePublicKey`]) so the action is a self-contained,
    /// hash-bound description of every input the trustee decrypts its own share
    /// from, even though they are also recoverable from the message store.
    ComputePartialDecryptions(
        CfgHash,
        PublicKeyHash,
        CiphertextsHash,
        SharesHashes,
        TrusteeIndex,
    ),
    /// Combine partial decryptions into plaintexts and post them.
    ComputePlaintexts(
        CfgHash,
        PublicKeyHash,
        CiphertextsHash,
        PartialDecryptionsHashes,
        TrusteeIndex,
    ),
    /// Produce the initial ballot set (test-only composition).
    ComputeBallots(CfgHash, PublicKeyHash),
}

///////////////////////////////////////////////////////////////////////////
// Prelude rules
//
// Relations and rules common to every protocol phase. Ascent's `ascent_source!`
// stores these as a reusable token template (`crate::datalog::prelude`) that the
// composed program below splices in via `include_source!`. Relations are of
// three kinds: inputs (facts injected from the outside world), intermediates
// (derived facts feeding other rules), and outputs (actions and errors read by
// the trustee application).
///////////////////////////////////////////////////////////////////////////

ascent::ascent_source! { prelude:

    // Input: the verified predicates held by the board (the EDB). These inject
    // facts from the outside world into the engine.
    relation predicate(Predicate);

    // Output: computations the trustee must perform. Actions produce side
    // effects — messages posted to the board — that advance the protocol.
    relation action(Action);

    // Output: an error condition was raised. If any error is derived, protocol
    // execution for this board must halt.
    relation error(String);

    // The executing trustee accepts the given configuration (hash) as valid,
    // with the given threshold, trustee count, and its own trustee index.
    relation configuration_valid(CfgHash, Threshold, TrusteeCount, TrusteeIndex);

    // Map the bootstrapping ConfigurationValid predicate to the configuration_valid fact.
    configuration_valid(c.configuration, c.threshold, c.trustee_count, c.self_index) <--
        predicate(p),
        if let Predicate::ConfigurationValid(c) = p;

    // Equivocation halt (§5.3): two distinct predicates that occupy the same
    // slot collide. `Predicate::collides` already excludes equal predicates, so
    // idempotent re-statements do not trigger this.
    error(format!("colliding predicates {:?}, {:?}", p1, p2)) <--
        predicate(p1),
        predicate(p2),
        if p1.collides(p2);

    // Configuration-domain halt (§7.3): the single point that enforces that
    // every predicate is scoped to this execution's configuration. A predicate
    // whose configuration hash differs from the accepted one is rejected here
    // (which is why `verify` does not check the domain).
    error(format!("predicate cfg does not match context {:?}", p1)) <--
        predicate(p1),
        configuration_valid(cfg_hash, _, _, _),
        if p1.get_configuration() != *cfg_hash;
}

///////////////////////////////////////////////////////////////////////////
// Composed program
///////////////////////////////////////////////////////////////////////////

/// The composed datalog program (for use by the trustee application).
///
/// It splices the prelude and the per-phase inference rules into a single
/// ascent program. All types the rule templates reference (the [`Predicate`]
/// enum and its [`Slot`] trait, [`Action`], [`AccumulatorSet`], and the
/// [`types`] aliases) must be in scope at this expansion site.
///
/// [`Predicate`]: crate::messages::predicate::Predicate
/// [`Slot`]: crate::messages::predicate::Slot
pub mod composed {
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
}

#[cfg(test)]
mod tests {
    use super::composed::run;
    use super::Action;
    use crate::messages::newtypes::{zero_hash, ConfigurationHash};
    use crate::messages::predicate::{ConfigurationValid, Predicate};

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
}
