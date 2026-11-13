// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! DKG phase actions (§7): `ComputeShares` and `ComputePublicKey`.

use anyhow::{anyhow, Result};

use cryptography::context::Context;
use cryptography::traits::groups::CryptographicGroup;
use cryptography::utils::serialization::Deserializable;

use crate::messages::artifact::{DkgPublicKey, Shares};
use crate::messages::newtypes::{ConfigurationHash, SharesHash, TrusteeIndex};
use crate::messages::wire::ProtocolMessage;

use crate::board::store::MessageStore;

use super::{domain_label, encrypted_share, Trustee, DKG_CHECKING_VALUE_PURPOSE, WIRE_DATE};

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

                let share_scalar = C::G::decrypt_scalar(
                    encrypted_share(&shares, self_slot)?,
                    &self.share_encryption.skey,
                )
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

            let public_key = DkgPublicKey::<C>::new(joint_pk.y, verification_keys.to_vec());
            let message = ProtocolMessage::<C>::public_key(self, WIRE_DATE, *cfg_hash, &public_key);
            Ok(vec![message])
        })
    }
}

#[cfg(test)]
mod tests {
    use std::marker::PhantomData;

    use cryptography::context::{Context, RistrettoCtx};
    use cryptography::cryptosystem::elgamal::KeyPair;
    use cryptography::utils::signatures::SignatureScheme;

    use crate::board::store::MessageStore;
    use crate::board::verify::verify;
    use crate::messages::artifact::{Configuration, Shares};
    use crate::messages::newtypes::{hash_bytes, ConfigurationHash, SharesHash};
    use crate::messages::wire::ProtocolMessage;
    use crate::protocol_manager::ProtocolManager;
    use crate::trustee::Trustee;

    /// What trustee 2 of three answers when it derives its key from a
    /// dealing of trustee 1 that carries no shares.
    fn derive_from_a_dealing_without_shares<C: Context>() -> anyhow::Result<()> {
        let mut rng = C::get_rng();
        let pm = ProtocolManager::<C>::new(C::SignatureScheme::gen_signing_key(&mut rng));
        let signing_keys: Vec<_> = (0..3)
            .map(|_| C::SignatureScheme::gen_signing_key(&mut rng))
            .collect();
        let share_keys: Vec<KeyPair<C>> = (0..3).map(|_| KeyPair::<C>::generate()).collect();
        let cfg = Configuration::<C>::new(
            0,
            C::SignatureScheme::verifying_key(&pm.signing_key),
            signing_keys
                .iter()
                .map(C::SignatureScheme::verifying_key)
                .collect(),
            2,
            1,
            share_keys.iter().map(|keys| keys.pkey.y.clone()).collect(),
            PhantomData,
        )
        .expect("a valid configuration");
        let cfg_hash = ConfigurationHash::from_configuration(&cfg).expect("a configuration hash");
        let configuration = ProtocolMessage::<C>::configuration(&pm, 0, &cfg);
        let mut store = MessageStore::<C>::from_configuration_message(&configuration)
            .expect("an accepted configuration");
        let mut trustees =
            signing_keys
                .into_iter()
                .zip(share_keys)
                .map(|(signing_key, share_keys)| {
                    Trustee::<C>::new("trustee".into(), signing_key, share_keys, &cfg)
                        .expect("a trustee of the configuration")
                });
        let dealer = trustees.next().expect("trustee 1");
        let recipient = trustees.next().expect("trustee 2");
        let no_shares = Shares::<C> {
            commitments: vec![],
            encrypted_shares: vec![],
        };
        let dealing = ProtocolMessage::<C>::shares(&dealer, 0, cfg_hash, &no_shares);
        let shares_hash = SharesHash(hash_bytes(dealing.body.as_ref().expect("a body")));
        let (predicate, body) = verify(&dealing, &cfg).expect("a signed dealing");
        store.insert(predicate, body).expect("a stored dealing");

        recipient
            .compute_public_key(&store, &cfg_hash, &[shares_hash], 2)
            .map(|_| ())
    }

    /// A dealing without the trustee's share is an error; it used to panic
    /// on the index.
    #[test]
    fn a_dealing_without_the_trustees_share_is_an_error() {
        let result = derive_from_a_dealing_without_shares::<RistrettoCtx>();

        let err = match result {
            Ok(()) => panic!("a dealing without the share must error"),
            Err(err) => err,
        };
        assert!(
            err.to_string()
                .contains("the dealing has no share for trustee 2"),
            "{err:#}"
        );
    }
}
