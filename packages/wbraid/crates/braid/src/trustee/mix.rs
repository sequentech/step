// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Mixing phase actions (§8): `ComputeMix` and `SignMix`.

use anyhow::{anyhow, Result};

use cryptography::context::Context;
use cryptography::cryptosystem::elgamal::Ciphertext;
use cryptography::traits::groups::CryptographicGroup;
use cryptography::utils::serialization::{VDeserializable, VSerializable};

use crate::messages::artifact::{Ballots, DkgPublicKey, Mix};
use crate::messages::newtypes::{CiphertextsHash, ConfigurationHash, PublicKeyHash, TrusteeIndex};
use crate::messages::wire::ProtocolMessage;

use crate::board::store::MessageStore;
use crate::datalog::MixSource;

use super::{domain_label, Trustee, WIRE_DATE};

impl<C: Context> Trustee<C> {
    /// The input ciphertexts of a mix, fetched directly from the store named by
    /// `source` (§8) and keyed by `input_hash`. The view accessors are
    /// content-addressed by `input_hash`, so if `source` and `input_hash`
    /// disagree the lookup returns nothing and this errors — the sanity check
    /// that replaces the old ballots-first fall-through.
    pub(super) fn mix_input_ciphertexts<const W: usize>(
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
    pub(super) fn compute_mix(
        &self,
        view: &MessageStore<C>,
        cfg_hash: &ConfigurationHash,
        pk_hash: &PublicKeyHash,
        source: &MixSource,
        input_hash: &CiphertextsHash,
        _self_index: TrusteeIndex,
    ) -> Result<Vec<ProtocolMessage<C>>> {
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
                let message =
                    ProtocolMessage::<C>::mix(self, WIRE_DATE, *cfg_hash, *pk_hash, *input_hash, &mix);
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
                ProtocolMessage::<C>::mix(self, WIRE_DATE, *cfg_hash, *pk_hash, *input_hash, &mix);
            Ok(vec![message])
        })
    }

    /// `SignMix` (§8): verify a mix's shuffle proof against its input and, on
    /// success, post the signature that advances the mixing chain. `source` names
    /// where the mix's input comes from (the ballots for the first mix, a prior
    /// mixer's output otherwise), fetched directly by `input_hash`; the mix itself
    /// is fetched by (`input_hash`, `output_hash`). The Fiat-Shamir domain is
    /// re-derived from `cfg_hash` + `input_hash` exactly as the mixer derived it.
    pub(super) fn sign_mix(
        &self,
        view: &MessageStore<C>,
        cfg_hash: &ConfigurationHash,
        pk_hash: &PublicKeyHash,
        source: &MixSource,
        input_hash: &CiphertextsHash,
        output_hash: &CiphertextsHash,
        _self_index: TrusteeIndex,
    ) -> Result<Vec<ProtocolMessage<C>>> {
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
                let message = ProtocolMessage::<C>::mix_signature(
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

            let message = ProtocolMessage::<C>::mix_signature(
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
