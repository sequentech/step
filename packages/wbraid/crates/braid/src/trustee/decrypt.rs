// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Decryption phase actions (§7): `ComputePartialDecryptions` and
//! `ComputePlaintexts`.

use anyhow::{anyhow, Result};

use cryptography::context::Context;
use cryptography::traits::groups::GroupScalar;
use cryptography::utils::serialization::VDeserializable;

use crate::messages::artifact::{DkgPublicKey, Mix, PartialDecryption, Plaintexts, Shares};
use crate::messages::newtypes::{
    CiphertextsHash, ConfigurationHash, PartialDecryptionHash, PublicKeyHash, SharesHash,
    TrusteeIndex,
};
use crate::messages::wire::ProtocolMessage;

use crate::board::store::MessageStore;

use super::{domain_label, Trustee, WIRE_DATE};

impl<C: Context> Trustee<C> {
    /// `ComputePartialDecryptions` (§7): decrypt this trustee's DKG share from
    /// every dealer to rebuild its secret, then produce a decryption factor for
    /// each of the final mixed ciphertexts, with a **single** proof covering all
    /// of them. The dealer shares are named explicitly by `shares_hashes`
    /// (carried in the action) — the action is a self-contained, hash-bound
    /// description of its inputs, even though the shares are also held in the
    /// store.
    pub(super) fn compute_partial_decryptions(
        &self,
        view: &MessageStore<C>,
        cfg_hash: &ConfigurationHash,
        pk_hash: &PublicKeyHash,
        ciphertexts_hash: &CiphertextsHash,
        shares_hashes: &[SharesHash],
        self_index: TrusteeIndex,
    ) -> Result<Vec<ProtocolMessage<C>>> {
        use cryptography::traits::groups::CryptographicGroup;

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
    ) -> Result<Vec<ProtocolMessage<C>>> {
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
        let partial_decryption = recipient
            .partial_decrypt(&wrapped, &label)
            .map_err(|e| anyhow!("failed to compute decryption factors: {:?}", e))?;

        let message = ProtocolMessage::<C>::partial_decryptions(
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
    pub(super) fn compute_plaintexts(
        &self,
        view: &MessageStore<C>,
        cfg_hash: &ConfigurationHash,
        pk_hash: &PublicKeyHash,
        ciphertexts_hash: &CiphertextsHash,
        decryptions_hashes: &[PartialDecryptionHash],
        _self_index: TrusteeIndex,
    ) -> Result<Vec<ProtocolMessage<C>>> {
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
        decryptions_hashes: &[PartialDecryptionHash],
        dkg_pk: &DkgPublicKey<C>,
    ) -> Result<Vec<ProtocolMessage<C>>> {
        use cryptography::dkgd::recipient::{
            combine, AttributedDecryption, DkgCiphertext, ParticipantPosition,
        };

        let label = domain_label(cfg_hash, "decryption proof");

        let mix_body = view
            .mix_body_by_output(ciphertexts_hash)
            .ok_or_else(|| anyhow!("missing final mix output {:?}", ciphertexts_hash))?;
        let mix = Mix::<C, W>::deser(mix_body)
            .map_err(|e| anyhow!("failed to deserialize final mix: {:?}", e))?;

        // Each contribution is attributed here, from the producing trustee's
        // index rather than from the message body — the body carries no position
        // precisely so that a trustee cannot claim another's. The verification
        // key follows from that index, so the two cannot be misaligned.
        let mut contributions: Vec<AttributedDecryption<C, W, P>> =
            Vec::with_capacity(decryptions_hashes.len());

        for df_hash in decryptions_hashes {
            let (sender, body) = view
                .partial_decryptions_by_hash(df_hash)
                .ok_or_else(|| anyhow!("missing partial decryptions body for {:?}", df_hash))?;
            let partial = PartialDecryption::<C, W>::deser(body)
                .map_err(|e| anyhow!("failed to deserialize partial decryptions: {:?}", e))?;
            contributions.push(AttributedDecryption::new(
                partial,
                ParticipantPosition::from_usize(sender),
                dkg_pk.verification_keys[sender - 1].clone(),
            ));
        }

        let wrapped: Vec<DkgCiphertext<C, W, T>> = mix
            .ciphertexts
            .iter()
            .map(|c| DkgCiphertext(c.clone()))
            .collect();

        let contributions: [AttributedDecryption<C, W, P>; T] =
            contributions.try_into().map_err(|v: Vec<_>| {
                anyhow!("expected {} partial decryptions, got {}", T, v.len())
            })?;

        let plaintexts = combine(&wrapped, &contributions, &label)
            .map_err(|e| anyhow!("failed to combine decryption factors: {:?}", e))?;

        let plaintexts = Plaintexts::<C, W>(plaintexts);
        let message = ProtocolMessage::<C>::plaintexts(
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
