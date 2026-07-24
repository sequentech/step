// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The v0.6 session runtime: the trustee-side protocol engine.
//!
//! This is the new (M1) replacement for `crate::protocol::{trustee, action}`.
//! A [`SessionTrustee`] holds only this trustee's identity and secrets; the board
//! state lives in the board client (`crate::board`). Its [`SessionTrustee::step`]
//! is a **pure** function of the board's [`MessageStore`] read view:
//!
//! 1. read the board-sourced predicate set and add this trustee's own
//!    `ConfigurationValid` fact (§9.7), forming the datalog EDB;
//! 2. **run the datalog** engine ([`crate::datalog::composed::run`], §7.4) to
//!    derive the enabled [`Action`]s;
//! 3. **execute** each action — the cryptography ported from the old
//!    `protocol::action` modules, minus channels/symmetric wrapping/batches
//!    (§9.4) — producing signed [`WireMessage`]s, which are returned (never
//!    stored or posted here).
//!
//! Per the loop-back rule (§6) the trustee never advances on its own output: a
//! produced message only takes effect once the board client posts it and fetches
//! it back. The action layer picks up the ciphertext width and threshold/trustee
//! counts from the view's configuration and lowers them to const generics via the
//! dispatch macros.
//!
//! [`MessageStore`]: crate::messages::store::MessageStore

use anyhow::{anyhow, Result};

use cryptography::context::Context;
use cryptography::cryptosystem::elgamal::{Ciphertext, KeyPair};
use cryptography::traits::groups::CryptographicGroup;
use cryptography::utils::serialization::{VDeserializable, VSerializable};
use cryptography::utils::signatures::SignatureScheme;

use b4::messages::artifact::{
    Ballots, Configuration, DkgPublicKey, Mix, PartialDecryption, Plaintexts, Shares,
};
use b4::messages::message::Signer;
use b4::messages::newtypes::{
    CiphertextsHash, ConfigurationHash, DecryptionFactorsHash, PublicKeyHash, SharesHash,
    Timestamp, TrusteeIndex, PROTOCOL_MANAGER_INDEX,
};
use b4::messages::wire::WireMessage;

use crate::datalog::{self, Action, MixSource};
use crate::messages::predicate::ConfigurationValid;
use crate::messages::store::MessageStore;

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
pub struct SessionTrustee<C: Context> {
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

impl<C: Context> Signer<C> for SessionTrustee<C> {
    fn get_signing_key(&self) -> &<C::SignatureScheme as SignatureScheme<C::Rng>>::Signer {
        &self.signing_key
    }

    fn get_name(&self) -> String {
        self.name.clone()
    }
}

impl<C: Context> SessionTrustee<C> {
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
    pub fn step(&self, view: &MessageStore<C>) -> Result<Vec<WireMessage<C>>> {
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
    fn execute(&self, action: &Action, view: &MessageStore<C>) -> Result<Vec<WireMessage<C>>> {
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
            Action::ComputeBallots(..) => Err(anyhow!(
                "ComputeBallots is authored by the protocol manager, not executed by a trustee"
            )),
        }
    }

    /// `ComputeShares` (§7): deal a fresh Pedersen sharing and post the encrypted
    /// shares. Each trustee's share is ElGamal-encrypted directly to that
    /// trustee's configured share-encryption public key (§9.4) — no channel,
    /// symmetric wrapping, or PoK.
    fn compute_shares(
        &self,
        view: &MessageStore<C>,
        cfg_hash: &ConfigurationHash,
        _self_index: TrusteeIndex,
    ) -> Result<Vec<WireMessage<C>>> {
        use cryptography::dkgd::dealer::Dealer;

        let cfg = view.configuration();
        let num_trustees = cfg.trustees.len();
        let threshold = cfg.threshold;

        if cfg.share_encryption_keys.len() != num_trustees {
            return Err(anyhow!(
                "configuration has {} share-encryption keys but {} trustees",
                cfg.share_encryption_keys.len(),
                num_trustees
            ));
        }

        crate::dispatch_threshold_trustees!(threshold, num_trustees, {
            let dealer = Dealer::<C, T, P>::generate();
            let dealer_shares = dealer.get_verifiable_shares();

            let mut encrypted_shares = Vec::with_capacity(num_trustees);
            for i in 0..num_trustees {
                let share = dealer_shares.shares[i].clone();
                let recipient_pk = &cfg.share_encryption_keys[i];
                let share_bytes = C::G::encrypt_scalar(&share, recipient_pk).map_err(|e| {
                    anyhow!("failed to encrypt share for trustee {}: {:?}", i + 1, e)
                })?;
                encrypted_shares.push(share_bytes);
            }

            let shares = Shares::<C> {
                commitments: dealer_shares.checking_values.to_vec(),
                encrypted_shares,
            };
            let message = WireMessage::<C>::shares(self, WIRE_DATE, *cfg_hash, &shares);
            Ok(vec![message])
        })
    }

    /// `ComputePublicKey` (§7): decrypt this trustee's share from every dealer,
    /// verify them against the commitments, and combine into this trustee's view
    /// of the joint public key plus the per-trustee verification keys.
    ///
    /// `shares_hashes` arrives in dealer-index order (1..=P), contiguous, because
    /// the action only fires once all dealers' shares are accumulated (§7).
    fn compute_public_key(
        &self,
        view: &MessageStore<C>,
        cfg_hash: &ConfigurationHash,
        shares_hashes: &[SharesHash],
        self_index: TrusteeIndex,
    ) -> Result<Vec<WireMessage<C>>> {
        use cryptography::dkgd::dealer::VerifiableShare;
        use cryptography::dkgd::recipient::{ParticipantPosition, Recipient};

        let cfg = view.configuration();
        let num_trustees = cfg.trustees.len();
        let threshold = cfg.threshold;
        // 1-based trustee index -> 0-based recipient slot in each dealer's shares.
        let self_slot = self_index - 1;

        crate::dispatch_threshold_trustees!(threshold, num_trustees, {
            let mut verifiable_shares: Vec<VerifiableShare<C, T>> =
                Vec::with_capacity(num_trustees);
            let mut all_checking_values: Vec<[C::Element; T]> = Vec::with_capacity(num_trustees);

            for shares_hash in shares_hashes {
                let body = view
                    .shares_body(shares_hash)
                    .ok_or_else(|| anyhow!("missing shares body for {:?}", shares_hash))?;
                let shares = Shares::<C>::deser(body)
                    .map_err(|e| anyhow!("failed to deserialize shares: {:?}", e))?;

                let encrypted_share = &shares.encrypted_shares[self_slot];
                let share_scalar =
                    C::G::decrypt_scalar(encrypted_share, &self.share_encryption.skey)
                        .map_err(|e| anyhow!("failed to decrypt share: {:?}", e))?;

                let checking_values: [C::Element; T] = shares
                    .commitments
                    .clone()
                    .try_into()
                    .map_err(|_| anyhow!("expected {} commitments", T))?;

                verifiable_shares.push(VerifiableShare::new(share_scalar, checking_values.clone()));
                all_checking_values.push(checking_values);
            }

            let all_cvs: [[C::Element; T]; P] =
                all_checking_values.try_into().map_err(|v: Vec<_>| {
                    anyhow!("expected {} checking-value sets, got {}", P, v.len())
                })?;
            let shares_array: [VerifiableShare<C, T>; P] = verifiable_shares
                .try_into()
                .map_err(|v: Vec<_>| anyhow!("expected {} shares, got {}", P, v.len()))?;

            let position = ParticipantPosition::from_usize(self_index);
            let (joint_pk, _verification_key, _sk) =
                Recipient::<C, T, P>::verify_shares(&position, &shares_array)
                    .map_err(|e| anyhow!("share verification failed: {:?}", e))?;

            let mut verification_keys = Vec::with_capacity(num_trustees);
            for j in 0..num_trustees {
                let pos_j = ParticipantPosition::from_usize(j + 1);
                verification_keys.push(Recipient::<C, T, P>::verification_key(&pos_j, &all_cvs));
            }

            let public_key = DkgPublicKey::<C>::new(joint_pk, verification_keys);
            let message = WireMessage::<C>::public_key(self, WIRE_DATE, *cfg_hash, &public_key);
            Ok(vec![message])
        })
    }

    /// The input ciphertexts of a mix, fetched directly from the store named by
    /// `source` (§8) and keyed by `input_hash`. The view accessors are
    /// content-addressed by `input_hash`, so if `source` and `input_hash`
    /// disagree the lookup returns nothing and this errors — the sanity check
    /// that replaces the old ballots-first fall-through.
    fn mix_input_ciphertexts<const W: usize>(
        &self,
        view: &MessageStore<C>,
        source: &MixSource,
        input_hash: &CiphertextsHash,
    ) -> Result<Vec<Ciphertext<C, W>>> {
        match source {
            MixSource::Ballots => {
                let body = view.ballots_body(input_hash).ok_or_else(|| {
                    anyhow!(
                        "MixSource::Ballots named for input {:?}, but no ballots have that hash",
                        input_hash
                    )
                })?;
                Ok(Ballots::<C, W>::deser(body)
                    .map_err(|e| anyhow!("failed to deserialize ballots: {:?}", e))?
                    .ciphertexts)
            }
            MixSource::PriorMix => {
                let body = view.mix_body_by_output(input_hash).ok_or_else(|| {
                    anyhow!(
                        "MixSource::PriorMix named for input {:?}, but no mix has that output",
                        input_hash
                    )
                })?;
                Ok(Mix::<C, W>::deser(body)
                    .map_err(|e| anyhow!("failed to deserialize source mix: {:?}", e))?
                    .ciphertexts)
            }
        }
    }

    /// `ComputeMix` (§8): re-encrypt and permute this trustee's input
    /// ciphertexts, then prove the shuffle. `source` names where the input comes
    /// from — the manager's `Ballots` (first mixer) or the previous mixer's `Mix`
    /// output — so it is fetched directly from that store (keyed by `input_hash`,
    /// which also cross-checks the source). The shuffle's Fiat-Shamir domain is
    /// bound to `cfg_hash` + `input_hash` (§9.4), which every verifier reproduces.
    fn compute_mix(
        &self,
        view: &MessageStore<C>,
        cfg_hash: &ConfigurationHash,
        pk_hash: &PublicKeyHash,
        source: &MixSource,
        input_hash: &CiphertextsHash,
        _self_index: TrusteeIndex,
    ) -> Result<Vec<WireMessage<C>>> {
        use cryptography::cryptosystem::elgamal::PublicKey;
        use cryptography::zkp::shuffle::Shuffler;

        let cfg = view.configuration();
        let pk_body = view
            .public_key_body(pk_hash)
            .ok_or_else(|| anyhow!("missing public key body for {:?}", pk_hash))?;
        let dkg_pk = DkgPublicKey::<C>::deser(pk_body)
            .map_err(|e| anyhow!("failed to deserialize public key: {:?}", e))?;

        crate::dispatch_ciphertext_width!(cfg.ciphertext_width, {
            let input_ciphertexts: Vec<Ciphertext<C, W>> =
                self.mix_input_ciphertexts::<W>(view, source, input_hash)?;

            // An empty input yields a null mix: no shuffle, no proof (§8).
            if input_ciphertexts.is_empty() {
                let mix = Mix::<C, W>::null();
                let message = WireMessage::<C>::mix(
                    self,
                    WIRE_DATE,
                    *cfg_hash,
                    *pk_hash,
                    *input_hash,
                    &mix,
                );
                return Ok(vec![message]);
            }

            let pk = PublicKey::new(dkg_pk.pk.clone());
            let seed = shuffle_generators_seed(cfg_hash, input_hash);
            let generators = C::G::ind_generators(input_ciphertexts.len(), &seed)
                .map_err(|e| anyhow!("failed to derive shuffle generators: {:?}", e))?;
            let shuffler = Shuffler::new(generators, pk);
            let label = shuffle_proof_label(cfg_hash, input_hash);
            let (shuffled, proof) = shuffler
                .shuffle(&input_ciphertexts, &label)
                .map_err(|e| anyhow!("shuffle failed: {:?}", e))?;

            let mix = Mix::new(shuffled, proof);
            let message =
                WireMessage::<C>::mix(self, WIRE_DATE, *cfg_hash, *pk_hash, *input_hash, &mix);
            Ok(vec![message])
        })
    }

    /// `SignMix` (§8): verify a mix's shuffle proof against its input and, on
    /// success, post the signature that advances the mixing chain. `source` names
    /// where the mix's input comes from (the ballots for the first mix, a prior
    /// mixer's output otherwise), fetched directly by `input_hash`; the mix itself
    /// is fetched by (`input_hash`, `output_hash`). The Fiat-Shamir domain is
    /// re-derived from `cfg_hash` + `input_hash` exactly as the mixer derived it.
    fn sign_mix(
        &self,
        view: &MessageStore<C>,
        cfg_hash: &ConfigurationHash,
        pk_hash: &PublicKeyHash,
        source: &MixSource,
        input_hash: &CiphertextsHash,
        output_hash: &CiphertextsHash,
        _self_index: TrusteeIndex,
    ) -> Result<Vec<WireMessage<C>>> {
        use cryptography::cryptosystem::elgamal::PublicKey;
        use cryptography::zkp::shuffle::Shuffler;

        let cfg = view.configuration();
        let pk_body = view
            .public_key_body(pk_hash)
            .ok_or_else(|| anyhow!("missing public key body for {:?}", pk_hash))?;
        let dkg_pk = DkgPublicKey::<C>::deser(pk_body)
            .map_err(|e| anyhow!("failed to deserialize public key: {:?}", e))?;

        crate::dispatch_ciphertext_width!(cfg.ciphertext_width, {
            // The mix's input, drawn from the store named by `source`.
            let source_ciphertexts: Vec<Ciphertext<C, W>> =
                self.mix_input_ciphertexts::<W>(view, source, input_hash)?;

            let mix_body = view
                .mix_body(input_hash, output_hash)
                .ok_or_else(|| anyhow!("missing mix {:?} -> {:?}", input_hash, output_hash))?;
            let mix = Mix::<C, W>::deser(mix_body)
                .map_err(|e| anyhow!("failed to deserialize mix: {:?}", e))?;

            // A null mix carries no ciphertexts and no proof; verify by shape.
            if source_ciphertexts.is_empty() {
                if !mix.ciphertexts.is_empty() || mix.proof.is_some() {
                    return Err(anyhow!("null mix must have empty output and no proof"));
                }
                let message = WireMessage::<C>::mix_signature(
                    self,
                    WIRE_DATE,
                    *cfg_hash,
                    *pk_hash,
                    *input_hash,
                    *output_hash,
                );
                return Ok(vec![message]);
            }

            let proof = mix
                .proof
                .ok_or_else(|| anyhow!("non-null mix is missing its shuffle proof"))?;
            let pk = PublicKey::new(dkg_pk.pk.clone());
            let seed = shuffle_generators_seed(cfg_hash, input_hash);
            let generators = C::G::ind_generators(source_ciphertexts.len(), &seed)
                .map_err(|e| anyhow!("failed to derive shuffle generators: {:?}", e))?;
            let shuffler = Shuffler::new(generators, pk);
            let label = shuffle_proof_label(cfg_hash, input_hash);
            let verified = shuffler
                .verify(&source_ciphertexts, &mix.ciphertexts, &proof, &label)
                .map_err(|e| anyhow!("mix verification errored: {:?}", e))?;
            if !verified {
                return Err(anyhow!(
                    "mix {:?} -> {:?} failed verification",
                    input_hash,
                    output_hash
                ));
            }

            let message = WireMessage::<C>::mix_signature(
                self,
                WIRE_DATE,
                *cfg_hash,
                *pk_hash,
                *input_hash,
                *output_hash,
            );
            Ok(vec![message])
        })
    }

    /// `ComputePartialDecryptions` (§7): decrypt this trustee's DKG share from
    /// every dealer to rebuild its secret, then produce a decryption factor (and
    /// proof) for each of the final mixed ciphertexts. The dealer shares are
    /// named explicitly by `shares_hashes` (carried in the action) — the action
    /// is a self-contained, hash-bound description of its inputs, even though the
    /// shares are also held in the store.
    fn compute_partial_decryptions(
        &self,
        view: &MessageStore<C>,
        cfg_hash: &ConfigurationHash,
        pk_hash: &PublicKeyHash,
        ciphertexts_hash: &CiphertextsHash,
        shares_hashes: &[SharesHash],
        self_index: TrusteeIndex,
    ) -> Result<Vec<WireMessage<C>>> {
        use cryptography::traits::groups::GroupScalar;

        let cfg = view.configuration();
        let num_trustees = cfg.trustees.len();
        let threshold = cfg.threshold;
        // 1-based trustee index -> 0-based recipient slot / verification-key index.
        let self_slot = self_index - 1;

        let pk_body = view
            .public_key_body(pk_hash)
            .ok_or_else(|| anyhow!("missing public key body for {:?}", pk_hash))?;
        let dkg_pk = DkgPublicKey::<C>::deser(pk_body)
            .map_err(|e| anyhow!("failed to deserialize public key: {:?}", e))?;
        let verification_key = dkg_pk.verification_keys[self_slot].clone();

        // Rebuild this trustee's secret by summing the share it decrypts from
        // every dealer (§9.4): secret = Σ_d decrypt(shares_d[self_slot]).
        let mut secret = C::Scalar::zero();
        for shares_hash in shares_hashes {
            let body = view
                .shares_body(shares_hash)
                .ok_or_else(|| anyhow!("missing shares body for {:?}", shares_hash))?;
            let shares = Shares::<C>::deser(body)
                .map_err(|e| anyhow!("failed to deserialize shares: {:?}", e))?;
            let share = C::G::decrypt_scalar(
                &shares.encrypted_shares[self_slot],
                &self.share_encryption.skey,
            )
            .map_err(|e| anyhow!("failed to decrypt share: {:?}", e))?;
            secret = secret.add(&share);
        }

        // The width×threshold cryptography is lowered to const generics via a
        // monomorphized helper CALL per dispatch arm (see below), NOT an inlined
        // body: inlined, the ~27×8 nested match arms each reserve stack for their
        // large fixed-size locals in this single frame, overflowing the default
        // (debug / wasm) stack. A call keeps only the selected arm's frame live.
        crate::dispatch_threshold_trustees!(threshold, num_trustees, {
            crate::dispatch_ciphertext_width!(cfg.ciphertext_width, {
                self.compute_partial_decryptions_inner::<W, T, P>(
                    view,
                    cfg_hash,
                    pk_hash,
                    ciphertexts_hash,
                    self_index,
                    &secret,
                    &verification_key,
                )
            })
        })
    }

    /// Monomorphized body of [`Self::compute_partial_decryptions`] for a fixed
    /// ciphertext width `W`, threshold `T`, and trustee count `P`. Kept as a
    /// separate `#[inline(never)]` function so each dispatch arm is a call rather
    /// than an inlined copy, bounding the caller's stack frame (see the note in
    /// `compute_partial_decryptions`).
    #[inline(never)]
    fn compute_partial_decryptions_inner<const W: usize, const T: usize, const P: usize>(
        &self,
        view: &MessageStore<C>,
        cfg_hash: &ConfigurationHash,
        pk_hash: &PublicKeyHash,
        ciphertexts_hash: &CiphertextsHash,
        self_index: TrusteeIndex,
        secret: &C::Scalar,
        verification_key: &C::Element,
    ) -> Result<Vec<WireMessage<C>>> {
        use cryptography::dkgd::recipient::{DkgCiphertext, ParticipantPosition, Recipient};

        let label = domain_label(cfg_hash, "decryption proof");

        let mix_body = view
            .mix_body_by_output(ciphertexts_hash)
            .ok_or_else(|| anyhow!("missing final mix output {:?}", ciphertexts_hash))?;
        let mix = Mix::<C, W>::deser(mix_body)
            .map_err(|e| anyhow!("failed to deserialize final mix: {:?}", e))?;

        let position = ParticipantPosition::from_usize(self_index);
        let recipient =
            Recipient::<C, T, P>::new(position, verification_key.clone(), secret.clone());

        let wrapped: Vec<DkgCiphertext<C, W, T>> = mix
            .ciphertexts
            .iter()
            .map(|c| DkgCiphertext(c.clone()))
            .collect();
        let dfactors = recipient
            .decryption_factor(&wrapped, &label)
            .map_err(|e| anyhow!("failed to compute decryption factors: {:?}", e))?;

        let partial_decryption = PartialDecryption::new(dfactors.factors);
        let message = WireMessage::<C>::partial_decryptions(
            self,
            WIRE_DATE,
            *cfg_hash,
            *pk_hash,
            *ciphertexts_hash,
            &partial_decryption,
        );
        Ok(vec![message])
    }

    /// `ComputePlaintexts` (§7): verify the `threshold` partial decryptions named
    /// by `decryptions_hashes` and combine them (with Lagrange interpolation)
    /// into the final plaintexts. Each partial decryption's source position — and
    /// hence the verification key it is checked against — is recovered from the
    /// producing trustee's index (the message body carries no position), not from
    /// the order of the hashes, which may skip non-participating trustees.
    fn compute_plaintexts(
        &self,
        view: &MessageStore<C>,
        cfg_hash: &ConfigurationHash,
        pk_hash: &PublicKeyHash,
        ciphertexts_hash: &CiphertextsHash,
        decryptions_hashes: &[DecryptionFactorsHash],
        _self_index: TrusteeIndex,
    ) -> Result<Vec<WireMessage<C>>> {
        let cfg = view.configuration();
        let num_trustees = cfg.trustees.len();
        let threshold = cfg.threshold;

        let pk_body = view
            .public_key_body(pk_hash)
            .ok_or_else(|| anyhow!("missing public key body for {:?}", pk_hash))?;
        let dkg_pk = DkgPublicKey::<C>::deser(pk_body)
            .map_err(|e| anyhow!("failed to deserialize public key: {:?}", e))?;

        // Monomorphized helper CALL per dispatch arm (not an inlined body) to
        // bound the caller's stack frame across the ~27×8 nested match arms; see
        // the note on `compute_partial_decryptions`.
        crate::dispatch_threshold_trustees!(threshold, num_trustees, {
            crate::dispatch_ciphertext_width!(cfg.ciphertext_width, {
                self.compute_plaintexts_inner::<W, T, P>(
                    view,
                    cfg_hash,
                    pk_hash,
                    ciphertexts_hash,
                    decryptions_hashes,
                    &dkg_pk,
                )
            })
        })
    }

    /// Monomorphized body of [`Self::compute_plaintexts`] for fixed `W`, `T`, `P`.
    /// Separate `#[inline(never)]` function so each dispatch arm is a call rather
    /// than an inlined copy (see the note on `compute_partial_decryptions`).
    #[inline(never)]
    fn compute_plaintexts_inner<const W: usize, const T: usize, const P: usize>(
        &self,
        view: &MessageStore<C>,
        cfg_hash: &ConfigurationHash,
        pk_hash: &PublicKeyHash,
        ciphertexts_hash: &CiphertextsHash,
        decryptions_hashes: &[DecryptionFactorsHash],
        dkg_pk: &DkgPublicKey<C>,
    ) -> Result<Vec<WireMessage<C>>> {
        use cryptography::dkgd::recipient::{
            combine, DecryptionFactors, DkgCiphertext, ParticipantPosition,
        };

        let label = domain_label(cfg_hash, "decryption proof");

        let mix_body = view
            .mix_body_by_output(ciphertexts_hash)
            .ok_or_else(|| anyhow!("missing final mix output {:?}", ciphertexts_hash))?;
        let mix = Mix::<C, W>::deser(mix_body)
            .map_err(|e| anyhow!("failed to deserialize final mix: {:?}", e))?;

        let mut dfactors_vec: Vec<DecryptionFactors<C, W, P>> =
            Vec::with_capacity(decryptions_hashes.len());
        let mut vkeys_vec: Vec<C::Element> = Vec::with_capacity(decryptions_hashes.len());

        for df_hash in decryptions_hashes {
            let (sender, body) = view
                .partial_decryptions_by_hash(df_hash)
                .ok_or_else(|| anyhow!("missing partial decryptions body for {:?}", df_hash))?;
            let partial = PartialDecryption::<C, W>::deser(body)
                .map_err(|e| anyhow!("failed to deserialize partial decryptions: {:?}", e))?;
            let source = ParticipantPosition::from_usize(sender);
            dfactors_vec.push(DecryptionFactors::new(partial.factors, source));
            vkeys_vec.push(dkg_pk.verification_keys[sender - 1].clone());
        }

        let wrapped: Vec<DkgCiphertext<C, W, T>> = mix
            .ciphertexts
            .iter()
            .map(|c| DkgCiphertext(c.clone()))
            .collect();

        let dfactors_array: [DecryptionFactors<C, W, P>; T] =
            dfactors_vec.try_into().map_err(|v: Vec<_>| {
                anyhow!("expected {} decryption factor sets, got {}", T, v.len())
            })?;
        let vkeys_array: [C::Element; T] = vkeys_vec
            .try_into()
            .map_err(|v: Vec<_>| anyhow!("expected {} verification keys, got {}", T, v.len()))?;

        let plaintexts = combine(&wrapped, &dfactors_array, &vkeys_array, &label)
            .map_err(|e| anyhow!("failed to combine decryption factors: {:?}", e))?;

        let plaintexts = Plaintexts::<C, W>(plaintexts);
        let message = WireMessage::<C>::plaintexts(
            self,
            WIRE_DATE,
            *cfg_hash,
            *pk_hash,
            *ciphertexts_hash,
            &plaintexts,
        );
        Ok(vec![message])
    }
}

/// Domain-separation prefix for a Fiat–Shamir transcript, bound to this
/// execution's configuration hash (the per-execution domain, §3.3) rather than
/// the numeric `Configuration.id`. Mirrors the byte layout of the former
/// `Configuration::label` — a length-delimited `suffix` — but keyed on `cfg_hash`,
/// so two executions cannot share a proof transcript domain even if they reuse a
/// configuration `id`.
fn domain_label(cfg_hash: &ConfigurationHash, suffix: &str) -> Vec<u8> {
    let mut bytes = cfg_hash.ser();
    // platform-independent length (cannot use usize as it may differ)
    bytes.extend((suffix.len() as u64).to_le_bytes());
    bytes.extend(suffix.as_bytes());
    bytes
}

/// Fiat–Shamir seed for a shuffle's independent generators (§9.4).
///
/// The old crypto keyed the shuffle's domain separation on the mixing position
/// (`mix_no`). The datalog actions no longer carry the position, so the seed is
/// bound to `cfg_hash` plus the mix's (unique) input ciphertexts hash instead:
/// both are known to the mixer and every verifier before the proof is fixed, and
/// the datalog errors on two mixes sharing an input, so this is a deterministic,
/// agreed, per-instance domain separator.
fn shuffle_generators_seed(cfg_hash: &ConfigurationHash, input: &CiphertextsHash) -> Vec<u8> {
    let mut seed = domain_label(cfg_hash, "shuffle_generators");
    seed.extend(input.ser());
    seed
}

/// Fiat–Shamir label for a shuffle proof, bound to `cfg_hash` and the input hash
/// like the generator seed so the mixer and every verifier derive an identical
/// context.
fn shuffle_proof_label(cfg_hash: &ConfigurationHash, input: &CiphertextsHash) -> Vec<u8> {
    let mut label = domain_label(cfg_hash, "shuffle");
    label.extend(input.ser());
    label
}
