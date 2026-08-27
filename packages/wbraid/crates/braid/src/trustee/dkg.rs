// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! DKG phase actions (§7): `ComputeShares` and `ComputePublicKey`.

use anyhow::{anyhow, Result};

use cryptography::context::Context;
use cryptography::traits::groups::CryptographicGroup;
use cryptography::utils::serialization::VDeserializable;

use crate::messages::artifact::{DkgPublicKey, Shares};
use crate::messages::newtypes::{ConfigurationHash, SharesHash, TrusteeIndex};
use crate::messages::wire::ProtocolMessage;

use crate::board::store::MessageStore;

use super::{domain_label, Trustee, DKG_CHECKING_VALUE_PURPOSE, WIRE_DATE};

impl<C: Context> Trustee<C> {
    /// `ComputeShares` (§7): deal a fresh Pedersen sharing and post the encrypted
    /// shares. Each checking value carries a Schnorr proof of knowledge of its
    /// exponent, bound to this execution via `domain_label` (PROTOCOL.md §4.3).
    /// Each trustee's share is ElGamal-encrypted directly to that trustee's
    /// configured share-encryption public key (§9.4) — no channel, symmetric
    /// wrapping, or PoK on the encryption itself.
    pub(super) fn compute_shares(
        &self,
        view: &MessageStore<C>,
        cfg_hash: &ConfigurationHash,
        _self_index: TrusteeIndex,
    ) -> Result<Vec<ProtocolMessage<C>>> {
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
            let proof_context = domain_label(cfg_hash, DKG_CHECKING_VALUE_PURPOSE);
            let dealer = Dealer::<C, T, P>::generate();
            let dealer_shares = dealer
                .get_verifiable_shares(&proof_context)
                .map_err(|e| anyhow!("failed to prove checking values: {:?}", e))?;

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
            let message = ProtocolMessage::<C>::shares(self, WIRE_DATE, *cfg_hash, &shares);
            Ok(vec![message])
        })
    }

    /// `ComputePublicKey` (§7): verify every dealer's checking-value proofs,
    /// decrypt this trustee's share from every dealer, verify each share against
    /// the commitments, and combine into this trustee's view of the joint public
    /// key plus the per-trustee verification keys.
    ///
    /// `shares_hashes` arrives in dealer-index order (1..=P), contiguous, because
    /// the action only fires once all dealers' shares are accumulated (§7).
    pub(super) fn compute_public_key(
        &self,
        view: &MessageStore<C>,
        cfg_hash: &ConfigurationHash,
        shares_hashes: &[SharesHash],
        self_index: TrusteeIndex,
    ) -> Result<Vec<ProtocolMessage<C>>> {
        use cryptography::dkgd::dealer::{CheckingValue, VerifiableShare};
        use cryptography::dkgd::recipient::{ParticipantPosition, Recipient};

        let cfg = view.configuration();
        let num_trustees = cfg.trustees.len();
        let threshold = cfg.threshold;
        // 1-based trustee index -> 0-based recipient slot in each dealer's shares.
        let self_slot = self_index - 1;

        crate::dispatch_threshold_trustees!(threshold, num_trustees, {
            let mut verifiable_shares: Vec<VerifiableShare<C, T>> =
                Vec::with_capacity(num_trustees);
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

                let checking_values: [CheckingValue<C>; T] = shares
                    .commitments
                    .try_into()
                    .map_err(|_| anyhow!("expected {} commitments", T))?;
                verifiable_shares.push(VerifiableShare::new(share_scalar, checking_values));
            }
            let shares_array: [VerifiableShare<C, T>; P] = verifiable_shares
                .try_into()
                .map_err(|v: Vec<_>| anyhow!("expected {} shares, got {}", P, v.len()))?;

            // §7 round 2 in one call: every dealer's every checking-value proof
            // and every share are verified, then the joint key and all
            // verification keys derived. Any failure halts the trustee.
            let proof_context = domain_label(cfg_hash, DKG_CHECKING_VALUE_PURPOSE);
            let position = ParticipantPosition::from_usize(self_index);
            let (_recipient, joint_pk, verification_keys) =
                Recipient::<C, T, P>::from_shares(position, &shares_array, &proof_context)
                    .map_err(|e| anyhow!("dealing verification failed: {:?}", e))?;

            let public_key =
                DkgPublicKey::<C>::new(joint_pk.inner.y, verification_keys.to_vec());
            let message = ProtocolMessage::<C>::public_key(self, WIRE_DATE, *cfg_hash, &public_key);
            Ok(vec![message])
        })
    }
}
