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
pub mod action;
pub mod composed;
pub mod decrypt;
pub mod dkg;
pub mod mix;

pub use action::{Action, MixSource};

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
