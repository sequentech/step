// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The message store (§6.1 of `crates/braid/v0.6_spec.md`): the in-memory,
//! type-safe home for verified messages and the source of the datalog EDB.
//!
//! This is the pure, I/O-free core of the "board client". The full board client
//! (the `Board` trait, b4 fetch/post, and predicate persistence) is a braid-side
//! wrapper around this store and is an M2 concern; for M1 the store *is* the
//! whole in-memory surface the harness drives.
//!
//! # Collections (§6.1)
//! Storage is a set of **per-type collections**, so each collection picks the
//! shape that fits its predicate:
//! - the six **bodied** types are `HashMap<FullPredicate, Vec<u8>>` — key = the
//!   full predicate (the datalog relation tuple, content-addressed), value = the
//!   body bytes only (deserialized on demand by the W-generic action layer; the
//!   store itself stays `C`-only and **W-agnostic**);
//! - the **bodyless** [`MixSignature`] type is a `HashSet<MixSignature>` — no
//!   body, so no filler value type is needed.
//!
//! [`ConfigurationValid`] is never *received*: the store derives it once from the
//! accepted [`Configuration`] and emits it from [`get_predicates`] so the datalog
//! always has the configuration facts.
//!
//! # No collision check here
//! `insert` is a pure content-addressed set operation. Equivocating messages get
//! distinct keys, so both land; the authoritative `collides()` runs in the
//! **datalog** over the whole predicate set (§5.3), where it halts. The old
//! storage-layer overwrite check is intentionally gone (§6.1).
//!
//! [`get_predicates`]: MessageStore::get_predicates

use std::collections::{HashMap, HashSet};

use anyhow::{anyhow, Result};
use cryptography::context::Context;
use cryptography::utils::signatures::SignatureScheme;

use b4::messages::artifact::Configuration;
use b4::messages::newtypes::{
    CiphertextsHash, ConfigurationHash, DecryptionFactorsHash, PublicKeyHash, SharesHash,
    TrusteeIndex, PROTOCOL_MANAGER_INDEX,
};
use b4::messages::wire::WireMessage;

use crate::messages::predicate::{
    Ballots, ConfigurationValid, Mix, MixSignature, PartialDecryptions, Plaintexts, Predicate,
    PublicKey, Shares,
};
use crate::messages::verify::verify;

/// The verified-message store for a single trustee on a single board (§6.1).
pub struct MessageStore<C: Context> {
    /// The accepted configuration — the board's domain (§9.8).
    configuration: Configuration<C>,
    /// The configuration facts, derived once at construction and emitted as a
    /// predicate by [`MessageStore::get_predicates`].
    configuration_valid: ConfigurationValid,

    shares: HashMap<Shares, Vec<u8>>,
    public_key: HashMap<PublicKey, Vec<u8>>,
    ballots: HashMap<Ballots, Vec<u8>>,
    mix: HashMap<Mix, Vec<u8>>,
    mix_signature: HashSet<MixSignature>,
    partial_decryptions: HashMap<PartialDecryptions, Vec<u8>>,
    plaintexts: HashMap<Plaintexts, Vec<u8>>,
}

impl<C: Context> MessageStore<C> {
    /// Construct the store by **accepting** a `Configuration` message (§9.8):
    /// verify its manager self-signature, then derive and cache the
    /// [`ConfigurationValid`] fact for `self_pk`'s trustee.
    ///
    /// `self_pk` is this trustee's own verifying key; its 1-based position in the
    /// configuration becomes `ConfigurationValid.self_index`. Construction is the
    /// acceptance boundary: a store cannot exist without an accepted
    /// configuration.
    pub fn from_configuration_message(
        message: &WireMessage<C>,
        self_pk: &<C::SignatureScheme as SignatureScheme<C::Rng>>::Verifier,
    ) -> Result<Self> {
        let configuration = message.verify_configuration()?;

        let position = configuration
            .get_trustee_position(self_pk)
            .ok_or_else(|| anyhow!("this trustee's key is not part of the configuration"))?;
        if position == PROTOCOL_MANAGER_INDEX as usize {
            return Err(anyhow!(
                "the protocol manager does not run a trustee message store"
            ));
        }
        // 0-based configuration position -> 1-based trustee index (§4.3).
        let self_index: TrusteeIndex = position + 1;

        let configuration_valid = ConfigurationValid {
            configuration: ConfigurationHash::from_configuration(&configuration)?,
            threshold: configuration.threshold,
            trustee_count: configuration.trustees.len(),
            self_index,
        };

        Ok(Self {
            configuration,
            configuration_valid,
            shares: HashMap::new(),
            public_key: HashMap::new(),
            ballots: HashMap::new(),
            mix: HashMap::new(),
            mix_signature: HashSet::new(),
            partial_decryptions: HashMap::new(),
            plaintexts: HashMap::new(),
        })
    }

    /// The accepted configuration.
    pub fn configuration(&self) -> &Configuration<C> {
        &self.configuration
    }

    /// The derived configuration facts (including this trustee's `self_index`).
    pub fn configuration_valid(&self) -> &ConfigurationValid {
        &self.configuration_valid
    }

    /// Verify a received [`WireMessage`] against the accepted configuration and
    /// store the resulting predicate and body (§3.4, §6.1). This is the receive
    /// pipeline: `verify` → `insert`.
    ///
    /// `Configuration` messages are rejected by [`verify`] — they are handled
    /// only at construction ([`Self::from_configuration_message`]).
    pub fn accept(&mut self, message: &WireMessage<C>) -> Result<()> {
        let (predicate, body) = verify(message, &self.configuration)?;
        self.insert(predicate, body)
    }

    /// All predicates held by the store — the ascent EDB (§6.1): the derived
    /// [`ConfigurationValid`] plus every collection key.
    pub fn get_predicates(&self) -> Vec<Predicate> {
        let mut predicates: Vec<Predicate> = Vec::new();
        predicates.push(self.configuration_valid.clone().into());
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

    /// The stored body bytes for `predicate`, if any (§6.3). Bodyless predicates
    /// ([`ConfigurationValid`], [`MixSignature`]) have no body.
    pub fn get_body(&self, predicate: &Predicate) -> Option<&[u8]> {
        match predicate {
            Predicate::ConfigurationValid(_) | Predicate::MixSignature(_) => None,
            Predicate::Shares(p) => self.shares.get(p).map(Vec::as_slice),
            Predicate::PublicKey(p) => self.public_key.get(p).map(Vec::as_slice),
            Predicate::Ballots(p) => self.ballots.get(p).map(Vec::as_slice),
            Predicate::Mix(p) => self.mix.get(p).map(Vec::as_slice),
            Predicate::PartialDecryptions(p) => {
                self.partial_decryptions.get(p).map(Vec::as_slice)
            }
            Predicate::Plaintexts(p) => self.plaintexts.get(p).map(Vec::as_slice),
        }
    }

    /// The body bytes of the `Shares` message whose out-hash (`H(body)`) is
    /// `hash`, if held. `hash` content-addresses the body, so at most one entry
    /// matches. Used by the action layer to fetch each dealer's shares by the
    /// hash carried in a `ComputePublicKey`/decryption action.
    pub fn shares_body(&self, hash: &SharesHash) -> Option<&[u8]> {
        self.shares
            .iter()
            .find(|(predicate, _)| predicate.shares == *hash)
            .map(|(_, body)| body.as_slice())
    }

    /// The body bytes of the `PublicKey` message whose out-hash is `hash`, if
    /// held. Used by the mix/decryption actions to recover the DKG public key.
    pub fn public_key_body(&self, hash: &PublicKeyHash) -> Option<&[u8]> {
        self.public_key
            .iter()
            .find(|(predicate, _)| predicate.public_key == *hash)
            .map(|(_, body)| body.as_slice())
    }

    /// The body bytes of the `Ballots` message whose ciphertexts out-hash is
    /// `hash`, if held. The manager posts exactly one `Ballots` per public key,
    /// so at most one entry matches. Used by the first mixer to load its input.
    pub fn ballots_body(&self, hash: &CiphertextsHash) -> Option<&[u8]> {
        self.ballots
            .iter()
            .find(|(predicate, _)| predicate.ciphertexts == *hash)
            .map(|(_, body)| body.as_slice())
    }

    /// The body bytes of the `Mix` message whose output out-hash is `output`,
    /// if held. Used to resolve a mix input hash to the previous mixer's output
    /// ciphertexts (each output is unique, so at most one entry matches).
    pub fn mix_body_by_output(&self, output: &CiphertextsHash) -> Option<&[u8]> {
        self.mix
            .iter()
            .find(|(predicate, _)| predicate.output == *output)
            .map(|(_, body)| body.as_slice())
    }

    /// The body bytes of the `Mix` message from `input` to `output`, if held.
    /// Used by a signer to fetch the exact mix it is asked to verify.
    pub fn mix_body(
        &self,
        input: &CiphertextsHash,
        output: &CiphertextsHash,
    ) -> Option<&[u8]> {
        self.mix
            .iter()
            .find(|(predicate, _)| predicate.input == *input && predicate.output == *output)
            .map(|(_, body)| body.as_slice())
    }

    /// The sender (1-based trustee index) and body bytes of the
    /// `PartialDecryptions` message whose decryptions out-hash is `hash`, if
    /// held. The sender is the participant position needed to rebuild the
    /// cryptography-layer `DecryptionFactors` (the message-layer body carries no
    /// position) and to index the verification keys during combination.
    pub fn partial_decryptions_by_hash(
        &self,
        hash: &DecryptionFactorsHash,
    ) -> Option<(TrusteeIndex, &[u8])> {
        self.partial_decryptions
            .iter()
            .find(|(predicate, _)| predicate.decryptions == *hash)
            .map(|(predicate, body)| (predicate.sender, body.as_slice()))
    }

    /// Store a verified predicate and its body in the matching per-type
    /// collection. The exhaustive `match` gives compile-time totality (like
    /// [`Predicate::collides`]): a new predicate type cannot be silently
    /// forgotten.
    fn insert(&mut self, predicate: Predicate, body: Option<Vec<u8>>) -> Result<()> {
        match predicate {
            Predicate::ConfigurationValid(_) => Err(anyhow!(
                "ConfigurationValid is derived from the configuration, not inserted"
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
