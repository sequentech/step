// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Predicates: the typed, content-addressed statements that cross the trust
//! boundary after verification and feed the datalog engine.
//!
//! A [`Predicate`] is the projection of a verified statement onto its
//! wire-independent "head" (§4.2 of `crates/braid/v0.6_spec.md`): it identifies
//! *what was said* (sender plus the content hashes) without carrying the body.
//! Predicates are the keys of the per-type collections (§6.1) and, as a set,
//! they *are* the ascent EDB: the engine consumes `predicate(Predicate)` facts.
//!
//! Two predicates *collide* when they occupy the same protocol "slot" — i.e.
//! they are mutually exclusive statements a correct trustee must never both
//! make. Collision is detected by [`Predicate::collides`], the slot projection
//! (§5.1) restricted to *distinct* predicates: identical predicates are
//! idempotent re-statements, not equivocations.

use enum_dispatch::enum_dispatch;

use b4::messages::newtypes::{
    CiphertextsHash, ConfigurationHash, DecryptionFactorsHash, PlaintextsHash, PublicKeyHash,
    SharesHash, Threshold, TrusteeCount, TrusteeIndex,
};

///////////////////////////////////////////////////////////////////////////
// Predicate structs
//
// Each struct is a distinct `FullPredicate` type: it doubles as the key of its
// per-type collection (§6.1). Named fields make the head layout self-documenting
// and remove the positional ambiguity of the datalog tuples.
///////////////////////////////////////////////////////////////////////////

/// `ConfigurationValid`: the domain anchor derived from the accepted
/// configuration (§9.8).
///
/// Unlike the other predicates it is never received on the wire; the board
/// client derives it once from its stored configuration and emits it via
/// `get_predicates()` so the datalog always has the configuration facts
/// (threshold, trustee count, and this trustee's own index).
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct ConfigurationValid {
    pub configuration: ConfigurationHash,
    pub threshold: Threshold,
    pub trustee_count: TrusteeCount,
    pub self_index: TrusteeIndex,
}

/// `Shares`: trustee `sender` published its DKG shares (content hash `shares`).
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Shares {
    pub configuration: ConfigurationHash,
    pub shares: SharesHash,
    pub sender: TrusteeIndex,
}

/// `PublicKey`: trustee `sender` published its view of the joint public key.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct PublicKey {
    pub configuration: ConfigurationHash,
    pub public_key: PublicKeyHash,
    pub sender: TrusteeIndex,
}

/// `Ballots`: the protocol manager published the input ciphertexts for the set
/// of active mixing `trustees`.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Ballots {
    pub configuration: ConfigurationHash,
    pub public_key: PublicKeyHash,
    pub ciphertexts: CiphertextsHash,
    pub trustees: Vec<TrusteeIndex>,
}

/// `Mix`: trustee `sender` shuffled the `input` ciphertexts into `output`.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Mix {
    pub configuration: ConfigurationHash,
    pub public_key: PublicKeyHash,
    pub input: CiphertextsHash,
    pub output: CiphertextsHash,
    pub sender: TrusteeIndex,
}

/// `MixSignature`: trustee `sender` signed a mix (`input` -> `output`). Same
/// shape as [`Mix`] but a distinct — and bodyless (§4.4) — predicate.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct MixSignature {
    pub configuration: ConfigurationHash,
    pub public_key: PublicKeyHash,
    pub input: CiphertextsHash,
    pub output: CiphertextsHash,
    pub sender: TrusteeIndex,
}

/// `PartialDecryptions`: trustee `sender` published its decryption factors for
/// the `ciphertexts`.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct PartialDecryptions {
    pub configuration: ConfigurationHash,
    pub public_key: PublicKeyHash,
    pub ciphertexts: CiphertextsHash,
    pub decryptions: DecryptionFactorsHash,
    pub sender: TrusteeIndex,
}

/// `Plaintexts`: trustee `sender` published the combined plaintexts for the
/// `ciphertexts`.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Plaintexts {
    pub configuration: ConfigurationHash,
    pub public_key: PublicKeyHash,
    pub ciphertexts: CiphertextsHash,
    pub plaintexts: PlaintextsHash,
    pub sender: TrusteeIndex,
}

///////////////////////////////////////////////////////////////////////////
// Slot projection
///////////////////////////////////////////////////////////////////////////

/// Slot projection (§5.1). Each predicate maps to a coarser "slot"; two
/// *distinct* predicates collide when they share a slot. Rather than materialize
/// the slot as a value, each predicate implements the pairwise test directly —
/// this keeps the non-transitive [`MixSignature`] rule expressible.
///
/// This trait is `enum_dispatch`ed onto [`Predicate`], giving compile-time
/// totality: adding a `Predicate` variant without a `Slot` impl is a compile
/// error, so no slot can be silently forgotten.
#[enum_dispatch]
pub trait Slot {
    /// Whether `self` and `other` occupy the same slot. This is the *raw* slot
    /// test: it does **not** exclude equal predicates. Use
    /// [`Predicate::collides`] for the equivocation test.
    fn slot_collides(&self, other: &Predicate) -> bool;
}

impl Slot for ConfigurationValid {
    fn slot_collides(&self, other: &Predicate) -> bool {
        matches!(other, Predicate::ConfigurationValid(o) if self.self_index == o.self_index)
    }
}

impl Slot for Shares {
    fn slot_collides(&self, other: &Predicate) -> bool {
        matches!(other, Predicate::Shares(o) if self.sender == o.sender)
    }
}

impl Slot for PublicKey {
    fn slot_collides(&self, other: &Predicate) -> bool {
        matches!(other, Predicate::PublicKey(o) if self.sender == o.sender)
    }
}

impl Slot for Ballots {
    fn slot_collides(&self, other: &Predicate) -> bool {
        // There is a single ballots slot per configuration.
        matches!(other, Predicate::Ballots(_))
    }
}

impl Slot for Mix {
    fn slot_collides(&self, other: &Predicate) -> bool {
        matches!(other, Predicate::Mix(o) if self.sender == o.sender)
    }
}

impl Slot for MixSignature {
    fn slot_collides(&self, other: &Predicate) -> bool {
        // A signature collides with another from the same sender that shares
        // either endpoint of the mix; this relation is intentionally
        // non-transitive.
        matches!(
            other,
            Predicate::MixSignature(o)
                if self.sender == o.sender
                    && (self.input == o.input || self.output == o.output)
        )
    }
}

impl Slot for PartialDecryptions {
    fn slot_collides(&self, other: &Predicate) -> bool {
        matches!(other, Predicate::PartialDecryptions(o) if self.sender == o.sender)
    }
}

impl Slot for Plaintexts {
    fn slot_collides(&self, other: &Predicate) -> bool {
        matches!(other, Predicate::Plaintexts(o) if self.sender == o.sender)
    }
}

///////////////////////////////////////////////////////////////////////////
// Predicate enum
///////////////////////////////////////////////////////////////////////////

/// The verified, content-addressed head of a statement (§4.2). The set of all
/// predicates held by the board *is* the ascent EDB (§6.1); the datalog engine
/// consumes `predicate(Predicate)` facts.
#[enum_dispatch(Slot)]
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Predicate {
    ConfigurationValid(ConfigurationValid),
    Shares(Shares),
    PublicKey(PublicKey),
    Ballots(Ballots),
    Mix(Mix),
    MixSignature(MixSignature),
    PartialDecryptions(PartialDecryptions),
    Plaintexts(Plaintexts),
}

impl Predicate {
    /// Whether two predicates *collide* in the equivocation sense: they occupy
    /// the same slot but are not identical (§5.1). Identical predicates are
    /// idempotent re-statements, not equivocations, so they never collide.
    pub fn collides(&self, other: &Predicate) -> bool {
        self != other && self.slot_collides(other)
    }

    /// The configuration hash this predicate is scoped to.
    ///
    /// Uniform across every variant — each head structurally carries
    /// `configuration` — so it lives here rather than on [`Slot`]. Should
    /// predicates later gain genuine per-variant behaviour worth dispatching, a
    /// dedicated predicate trait can absorb this as a trivial case.
    pub fn get_configuration(&self) -> ConfigurationHash {
        match self {
            Predicate::ConfigurationValid(p) => p.configuration,
            Predicate::Shares(p) => p.configuration,
            Predicate::PublicKey(p) => p.configuration,
            Predicate::Ballots(p) => p.configuration,
            Predicate::Mix(p) => p.configuration,
            Predicate::MixSignature(p) => p.configuration,
            Predicate::PartialDecryptions(p) => p.configuration,
            Predicate::Plaintexts(p) => p.configuration,
        }
    }
}
