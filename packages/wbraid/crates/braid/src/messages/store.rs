// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The message store (§6.1 of `crates/braid/v0.6_spec.md`): the in-memory,
//! type-safe home for verified messages and the source of the datalog EDB.
//!
//! This is the pure, I/O-free core of the board client (`crate::board`), which
//! wraps it with persistence and b4 transport. The store is **identity-agnostic**
//! about the local trustee: it holds the board's `Configuration` and the verified
//! per-type collections, and exposes read accessors the trustee's `step` consumes.
//! The trustee-scoped `ConfigurationValid` fact is NOT held here — it depends on
//! the trustee's own `self_index`, so the trustee derives and injects it (§9.7).
//!
//! # Collections (§6.1)
//! - the six **bodied** types are `HashMap<FullPredicate, Vec<u8>>` — key = the
//!   full predicate (content-addressed), value = the body bytes only;
//! - the **bodyless** [`MixSignature`] type is a `HashSet<MixSignature>`.
//!
//! # No collision check here
//! [`insert`](MessageStore::insert) is a pure content-addressed set operation.
//! Equivocating messages get distinct keys, so both land; the authoritative
//! `collides()` runs in the **datalog** over the whole predicate set (§5.3).

use std::collections::{HashMap, HashSet};

use anyhow::{anyhow, Result};
use cryptography::context::Context;

use super::artifact::Configuration;
use super::newtypes::{
    CiphertextsHash, DecryptionFactorsHash, PublicKeyHash, SharesHash, TrusteeIndex,
};
use super::wire::ProtocolMessage;

use crate::messages::predicate::{
    Ballots, Mix, MixSignature, PartialDecryptions, Plaintexts, Predicate, PublicKey, Shares,
};

/// The verified-message store for a single board (§6.1). Owned by the board
/// client; read by the trustee through [`BoardView`].
pub struct MessageStore<C: Context> {
    /// The accepted configuration — the board's domain (§9.8).
    configuration: Configuration<C>,

    shares: HashMap<Shares, Vec<u8>>,
    public_key: HashMap<PublicKey, Vec<u8>>,
    ballots: HashMap<Ballots, Vec<u8>>,
    mix: HashMap<Mix, Vec<u8>>,
    mix_signature: HashSet<MixSignature>,
    partial_decryptions: HashMap<PartialDecryptions, Vec<u8>>,
    plaintexts: HashMap<Plaintexts, Vec<u8>>,
}

impl<C: Context> MessageStore<C> {
    /// Build the store by **accepting** a `Configuration` message (§9.8): verify
    /// its manager self-signature and hold the resulting `Configuration`.
    ///
    /// Identity-agnostic about the local trustee: no `self_pk` is needed. The
    /// board client is the acceptance boundary — a store cannot exist without an
    /// accepted configuration.
    pub fn from_configuration_message(message: &ProtocolMessage<C>) -> Result<Self> {
        let configuration = message.verify_configuration()?;
        Ok(Self {
            configuration,
            shares: HashMap::new(),
            public_key: HashMap::new(),
            ballots: HashMap::new(),
            mix: HashMap::new(),
            mix_signature: HashSet::new(),
            partial_decryptions: HashMap::new(),
            plaintexts: HashMap::new(),
        })
    }

    /// Store a verified predicate and its body in the matching per-type
    /// collection (§6.1). The exhaustive `match` gives compile-time totality: a
    /// new predicate type cannot be silently forgotten. Called by the board
    /// client after verifying + persisting the digest (the update-first gate).
    pub(crate) fn insert(&mut self, predicate: Predicate, body: Option<Vec<u8>>) -> Result<()> {
        match predicate {
            Predicate::ConfigurationValid(_) => Err(anyhow!(
                "ConfigurationValid is derived by the trustee, not admitted to the store"
            )),
            Predicate::Shares(p) => {
                self.shares.insert(p, Self::expect_body(body)?);
                Ok(())
            }
            Predicate::PublicKey(p) => {
                self.public_key.insert(p, Self::expect_body(body)?);
                Ok(())
            }
            Predicate::Ballots(p) => {
                self.ballots.insert(p, Self::expect_body(body)?);
                Ok(())
            }
            Predicate::Mix(p) => {
                self.mix.insert(p, Self::expect_body(body)?);
                Ok(())
            }
            Predicate::MixSignature(p) => {
                if body.is_some() {
                    return Err(anyhow!("MixSignature is bodyless but a body was provided"));
                }
                self.mix_signature.insert(p);
                Ok(())
            }
            Predicate::PartialDecryptions(p) => {
                self.partial_decryptions.insert(p, Self::expect_body(body)?);
                Ok(())
            }
            Predicate::Plaintexts(p) => {
                self.plaintexts.insert(p, Self::expect_body(body)?);
                Ok(())
            }
        }
    }

    /// The body bytes of a bodied predicate, or an error if absent.
    fn expect_body(body: Option<Vec<u8>>) -> Result<Vec<u8>> {
        body.ok_or_else(|| anyhow!("a bodied predicate was accepted without its body"))
    }
}

/// Read accessors (the surface the trustee's `step` consumes): the board's
/// configuration, the board-sourced predicate set, and the content-addressed
/// body lookups the action layer uses.
impl<C: Context> MessageStore<C> {
    pub fn configuration(&self) -> &Configuration<C> {
        &self.configuration
    }

    /// All board-sourced predicates — the ascent EDB (§6.1) minus the
    /// trustee-scoped `ConfigurationValid` (injected by the trustee, §9.7).
    pub fn get_predicates(&self) -> Vec<Predicate> {
        let mut predicates: Vec<Predicate> = Vec::new();
        predicates.extend(self.shares.keys().cloned().map(Predicate::from));
        predicates.extend(self.public_key.keys().cloned().map(Predicate::from));
        predicates.extend(self.ballots.keys().cloned().map(Predicate::from));
        predicates.extend(self.mix.keys().cloned().map(Predicate::from));
        predicates.extend(self.mix_signature.iter().cloned().map(Predicate::from));
        predicates.extend(
            self.partial_decryptions
                .keys()
                .cloned()
                .map(Predicate::from),
        );
        predicates.extend(self.plaintexts.keys().cloned().map(Predicate::from));
        predicates
    }

    /// The body bytes of the `Shares` message whose out-hash (`H(body)`) is
    /// `hash`, if held. Content-addressed, so at most one entry matches.
    pub fn shares_body(&self, hash: &SharesHash) -> Option<&[u8]> {
        self.shares
            .iter()
            .find(|(predicate, _)| predicate.shares == *hash)
            .map(|(_, body)| body.as_slice())
    }

    /// The body bytes of the `PublicKey` message whose out-hash is `hash`.
    pub fn public_key_body(&self, hash: &PublicKeyHash) -> Option<&[u8]> {
        self.public_key
            .iter()
            .find(|(predicate, _)| predicate.public_key == *hash)
            .map(|(_, body)| body.as_slice())
    }

    /// The body bytes of the `Ballots` message whose ciphertexts out-hash is
    /// `hash`.
    pub fn ballots_body(&self, hash: &CiphertextsHash) -> Option<&[u8]> {
        self.ballots
            .iter()
            .find(|(predicate, _)| predicate.ciphertexts == *hash)
            .map(|(_, body)| body.as_slice())
    }

    /// The body bytes of the `Mix` message whose output out-hash is `output`.
    pub fn mix_body_by_output(&self, output: &CiphertextsHash) -> Option<&[u8]> {
        self.mix
            .iter()
            .find(|(predicate, _)| predicate.output == *output)
            .map(|(_, body)| body.as_slice())
    }

    /// The body bytes of the `Mix` message from `input` to `output`, if held.
    pub fn mix_body(&self, input: &CiphertextsHash, output: &CiphertextsHash) -> Option<&[u8]> {
        self.mix
            .iter()
            .find(|(predicate, _)| predicate.input == *input && predicate.output == *output)
            .map(|(_, body)| body.as_slice())
    }

    /// The sender (1-based trustee index) and body bytes of the
    /// `PartialDecryptions` message whose decryptions out-hash is `hash`. The
    /// sender is the participant position needed to rebuild the crypto-layer
    /// `DecryptionFactors` and index the verification keys.
    pub fn partial_decryptions_by_hash(
        &self,
        hash: &DecryptionFactorsHash,
    ) -> Option<(TrusteeIndex, &[u8])> {
        self.partial_decryptions
            .iter()
            .find(|(predicate, _)| predicate.decryptions == *hash)
            .map(|(predicate, body)| (predicate.sender, body.as_slice()))
    }
}
