// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The v0.6 trustee: the trustee-side protocol engine.
//!
//! A [`Trustee`] holds only this trustee's identity and secrets — no session or
//! board state at all, which is why it is no longer called `SessionTrustee`: the
//! board state lives in the board client (`crate::board`). Its [`Trustee::step`]
//! is a **pure** function of the board's [`MessageStore`] read view:
//!
//! 1. read the board-sourced predicate set and add this trustee's own
//!    `ConfigurationValid` fact (§9.7), forming the datalog EDB;
//! 2. **run the datalog** engine ([`crate::datalog::composed::run`], §7.4) to
//!    derive the enabled [`Action`]s;
//! 3. **execute** each action — the core cryptography, minus channels/symmetric
//!    wrapping/batches (§9.4) — producing signed [`ProtocolMessage`]s, which are
//!    returned (never stored or posted here). The implementations are grouped by
//!    protocol phase, mirroring [`crate::datalog`]'s own layout: [`dkg`], [`mix`],
//!    [`decrypt`].
//!
//! Per the loop-back rule (§6) the trustee never advances on its own output: a
//! produced message only takes effect once the board client posts it and fetches
//! it back. The action layer picks up the ciphertext width and threshold/trustee
//! counts from the view's configuration and lowers them to const generics via the
//! dispatch macros.
//!
//! [`MessageStore`]: crate::board::store::MessageStore

mod decrypt;
mod dkg;
mod mix;

use anyhow::{anyhow, Result};

use cryptography::context::Context;
use cryptography::cryptosystem::elgamal::KeyPair;
use cryptography::utils::serialization::VSerializable;
use cryptography::utils::signatures::SignatureScheme;

use crate::messages::artifact::Configuration;
use crate::messages::newtypes::{
    ConfigurationHash, Timestamp, TrusteeIndex, PROTOCOL_MANAGER_INDEX,
};
use crate::messages::wire::{ProtocolMessage, Signer};

use crate::board::store::MessageStore;
use crate::datalog::{self, Action};
use crate::messages::predicate::ConfigurationValid;

/// Wire `date` stamped on every message this trustee produces. Timestamps are
/// purely informational (§10.2) — nothing in the protocol consumes them — so a
/// fixed placeholder is correct, not merely expedient.
const WIRE_DATE: Timestamp = 0;

/// A trustee driving a single board through the v0.6 protocol.
///
/// A **pure** protocol engine: it owns only its own identity and secrets — the
/// `signing_key` (authenticates every message it posts), the `share_encryption`
/// ElGamal keypair (its public element is in the configuration; its secret
/// decrypts the DKG shares dealt to it, replacing the old `Channel`, §9.4), and
/// the derived self-scoped `configuration_valid` fact (§9.7). The board state
/// lives in the board client; [`step`](Self::step) reads it through the board
/// client's [`MessageStore`] and returns messages with no side effect (§6 loop-back).
pub struct Trustee<C: Context> {
    /// Human-readable sender name, stamped into every posted message.
    name: String,
    /// Signing key for this trustee's messages.
    signing_key: <C::SignatureScheme as SignatureScheme<C::Rng>>::Signer,
    /// Keypair whose secret decrypts shares dealt to this trustee (§9.4).
    share_encryption: KeyPair<C>,
    /// This trustee's self-scoped configuration fact (§9.7), derived once at
    /// construction and injected into the datalog EDB at every `step`.
    configuration_valid: ConfigurationValid,
}

impl<C: Context> Signer<C> for Trustee<C> {
    fn get_signing_key(&self) -> &<C::SignatureScheme as SignatureScheme<C::Rng>>::Signer {
        &self.signing_key
    }

    fn get_name(&self) -> String {
        self.name.clone()
    }
}

impl<C: Context> Trustee<C> {
    /// Construct a trustee against the board's accepted `configuration` (held by
    /// the board client — §9.8: constructing the trustee requires a constructed
    /// board client). This trustee's 1-based index is derived from `signing_key`'s
    /// public side, and its self-scoped `ConfigurationValid` fact (§9.7) is cached
    /// for injection at `step`.
    pub fn new(
        name: String,
        signing_key: <C::SignatureScheme as SignatureScheme<C::Rng>>::Signer,
        share_encryption: KeyPair<C>,
        configuration: &Configuration<C>,
    ) -> Result<Self> {
        let self_pk = C::SignatureScheme::verifying_key(&signing_key);
        let position = configuration
            .get_trustee_position(&self_pk)
            .ok_or_else(|| anyhow!("this trustee's key is not part of the configuration"))?;
        if position == PROTOCOL_MANAGER_INDEX as usize {
            return Err(anyhow!("the protocol manager does not run a trustee"));
        }
        // 0-based configuration position -> 1-based trustee index (§4.3).
        let self_index: TrusteeIndex = position + 1;
        let configuration_valid = ConfigurationValid {
            configuration: ConfigurationHash::from_configuration(configuration)?,
            threshold: configuration.threshold,
            trustee_count: configuration.trustees.len(),
            self_index,
        };
        Ok(Self {
            name,
            signing_key,
            share_encryption,
            configuration_valid,
        })
    }

    /// Run inference over the board `view` and return the messages this trustee
    /// should post — a **pure** function (§6): nothing is stored or posted here,
    /// and the trustee does not advance on its own output (that takes effect only
    /// once it loops back through the board client, §6).
    ///
    /// The EDB is the board-sourced predicates plus this trustee's own
    /// `ConfigurationValid` fact (§9.7), which only it can compute.
    pub fn step(&self, view: &MessageStore<C>) -> Result<Vec<ProtocolMessage<C>>> {
        let mut predicates = view.get_predicates();
        predicates.push(self.configuration_valid.clone().into());

        let actions = datalog::composed::run(&predicates).map_err(|e| anyhow!(e))?;

        let mut outgoing = Vec::new();
        for action in &actions {
            outgoing.extend(self.execute(action, view)?);
        }
        Ok(outgoing)
    }

    /// Execute a single datalog-derived action, producing the message(s) to post.
    /// Dispatches to the phase-specific implementation ([`dkg`], [`mix`],
    /// [`decrypt`]).
    fn execute(&self, action: &Action, view: &MessageStore<C>) -> Result<Vec<ProtocolMessage<C>>> {
        match action {
            Action::ComputeShares(cfg, self_index) => self.compute_shares(view, cfg, *self_index),
            Action::ComputePublicKey(cfg, shares_hashes, self_index) => {
                self.compute_public_key(view, cfg, shares_hashes, *self_index)
            }
            Action::ComputeMix(cfg, public_key, source, input, self_index) => {
                self.compute_mix(view, cfg, public_key, source, input, *self_index)
            }
            Action::SignMix(cfg, public_key, source, input, output, self_index) => {
                self.sign_mix(view, cfg, public_key, source, input, output, *self_index)
            }
            Action::ComputePartialDecryptions(
                cfg,
                public_key,
                ciphertexts,
                shares_hashes,
                self_index,
            ) => self.compute_partial_decryptions(
                view,
                cfg,
                public_key,
                ciphertexts,
                shares_hashes,
                *self_index,
            ),
            Action::ComputePlaintexts(
                cfg,
                public_key,
                ciphertexts,
                decryptions_hashes,
                self_index,
            ) => self.compute_plaintexts(
                view,
                cfg,
                public_key,
                ciphertexts,
                decryptions_hashes,
                *self_index,
            ),
        }
    }
}

/// Purpose string of the domain label under which dealers prove knowledge of
/// their checking-value exponents (§7; PROTOCOL.md §4.3). Used wherever
/// dealings are verified: at key derivation ([`dkg`]) and at decrypt-time
/// re-derivation ([`decrypt`]).
pub(crate) const DKG_CHECKING_VALUE_PURPOSE: &str = "dkg_checking_value";

/// Domain-separation prefix for an execution-scoped Fiat–Shamir transcript,
/// bound to this execution's configuration hash (the per-execution domain,
/// §3.3) rather than the numeric `Configuration.id`. Mirrors the byte layout
/// of the former `Configuration::label` — a length-delimited `suffix` — but
/// keyed on `cfg_hash`, so two executions cannot share a proof transcript
/// domain even if they reuse a configuration `id`. Used by the [`dkg`] phase;
/// the tally phases use [`tally_label`], which additionally binds the tally
/// execution.
fn domain_label(cfg_hash: &ConfigurationHash, suffix: &str) -> Vec<u8> {
    let mut bytes = cfg_hash.ser();
    // platform-independent length (cannot use usize as it may differ);
    // big-endian, like every integer entering a hash transcript
    bytes.extend((suffix.len() as u64).to_be_bytes());
    bytes.extend(suffix.as_bytes());
    bytes
}

/// Domain-separation prefix for a tally-scoped Fiat–Shamir transcript
/// (PROTOCOL.md §2.4): [`domain_label`] with the tally-execution identifier
/// (the `Ballots` head's `tally_id`, big-endian) inserted after `cfg_hash`.
/// Sibling tallies over one DKG share `cfg_hash`, the public key, and possibly
/// the ciphertext lists (a re-run), so their proof transcripts are separated
/// by the tally identifier alone. Used by the [`mix`] and [`decrypt`] phases.
fn tally_label(cfg_hash: &ConfigurationHash, tally_id: u128, suffix: &str) -> Vec<u8> {
    let mut bytes = cfg_hash.ser();
    bytes.extend(tally_id.to_be_bytes());
    bytes.extend((suffix.len() as u64).to_be_bytes());
    bytes.extend(suffix.as_bytes());
    bytes
}

/// The ballot encryption context `ctx_enc` (PROTOCOL.md §5.2): the Naor-Yung
/// auxiliary key is `z = H2G(ctx_enc, "naor_yung_public_key_a")`, and every
/// ballot's well-formedness proof is bound to `ctx_enc`. It binds the election
/// execution (`Configuration.id`) and the election public key `y` — both known
/// to the voting client from the signed election configuration, and to the
/// trustees from their own `Configuration` and the DKG output. Deliberately
/// tally-agnostic: ballots survive a tally re-run without re-encryption.
pub fn ballot_encryption_context<C: Context>(configuration_id: u128, y: &C::Element) -> Vec<u8> {
    let mut bytes = configuration_id.to_be_bytes().to_vec();
    bytes.extend(y.ser());
    bytes
}
