#![allow(clippy::too_many_arguments)]

// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use crate::protocol::datalog;
use anyhow::Result;
use cryptography::traits::groups::{GroupScalar, CryptographicGroup};

/// Computes the decryption factors using this trustee's secret share.
///
/// The plaintexts can be calculated from a threshold number of
/// decryption factors. Each ciphertext produces one decryption
/// factor and one proof of discrete log equality.
///
/// Returns a Message of type DecryptionFactors signed by
/// this trustee.
///
/// As described in Cortier et al.; based on Pedersen.
pub(super) fn compute_decryption_factors<C: Context, S: crate::protocol::board::LocalBoardStorage>(
    cfg_h: &ConfigurationHash,
    batch: &BatchNumber,
    channels_hs: &ChannelsHashes,
    ciphertexts_h: &CiphertextsHash,
    mix_signer: &TrusteePosition,
    pk_h: &PublicKeyHash,
    shares_hs: &SharesHashes,
    self_p: &TrusteePosition,
    num_t: &TrusteeCount,
    threshold: &TrusteeCount,
    trustee: &Trustee<C, S>,
) -> Result<Vec<Message<C>>, ProtocolError> {
    use cryptography::dkgd::recipient::{Recipient, ParticipantPosition};
    
    let cfg = trustee.get_configuration(cfg_h)?;

    let pk = trustee
        .get_dkg_public_key(pk_h, 0)
        .add_context("Computing decryption factors")?;
    let vk = pk.verification_keys[*self_p].clone();

    let my_channel = trustee
        .get_channel(&ChannelHash(channels_hs.0[*self_p]), *self_p)
        .add_context("Computing decryption factors")?;
    let my_sk_keypair = trustee.decrypt_share_sk(&my_channel, &cfg)?;

    // Decrypt and sum all shares to compute our secret key
    let mut secret = C::Scalar::zero();
    for sender in 0..*num_t {
        let share_h = shares_hs.0[sender];
        let share_msg = trustee
            .get_shares(&SharesHash(share_h), sender)
            .add_context("Computing decryption factors")?;

        let share = C::G::decrypt_scalar(&share_msg.encrypted_shares[*self_p], &my_sk_keypair.skey)
            .map_err(|e| ProtocolError::InternalError(format!("Failed to decrypt share: {:?}", e)))?;

        secret = secret.add(&share);
    }

    let suffix = format!("decryption proof");
    let label = cfg.label(*batch, suffix);

    // Use nested dispatch macros to create Recipient with const generics for both T/P and W
    crate::dispatch_threshold_trustees!(*threshold, *num_t, {
        crate::dispatch_ciphertext_width!(cfg.ciphertext_width, {
            use cryptography::dkgd::recipient::DkgCiphertext;
            
            let ciphertexts = trustee
                .get_mix(ciphertexts_h, *batch, *mix_signer)
                .add_context("Computing decryption factors")?;

            info!(
                "ComputeDecryptionFactors [{}] ({})..",
                dbg_hash(&ciphertexts_h.0),
                ciphertexts.ciphertexts.len(),
            );
            
            let position = ParticipantPosition::from_usize(*self_p + 1);
            let recipient = Recipient::<C, T, P>::new(position, vk, secret);
            
            // Wrap plain Ciphertexts into DkgCiphertext<C, W, T> for decryption_factor
            let wrapped_ciphertexts: Vec<DkgCiphertext<C, W, T>> = ciphertexts.ciphertexts
                .iter()
                .map(|c| DkgCiphertext(c.clone()))
                .collect();
            
            let dfactors_with_source = recipient.decryption_factor(&wrapped_ciphertexts, &label)
                .map_err(|e| ProtocolError::InternalError(format!("Failed to compute decryption factors: {:?}", e)))?;
            
            // Create message-layer PartialDecryption (without source) from DecryptionFactors
            let partial_decryption = b4::messages::artifact::PartialDecryption::new(dfactors_with_source.factors);
            let m = b4::messages::message::Message::decryption_factors_msg(
                &cfg,
                *batch,
                partial_decryption,
                *ciphertexts_h,
                *shares_hs,
                trustee,
            )?;
            
            Ok(vec![m])
        })
    })
}

/// Computes the plaintexts from a threshold number of decryption factors.
///
/// Includes verification of decryption proofs. Returns a Message of type
/// Plaintexts signed by this trustee.
pub(super) fn compute_plaintexts<C: Context, S: crate::protocol::board::LocalBoardStorage>(
    cfg_h: &ConfigurationHash,
    batch: &BatchNumber,
    pk_h: &PublicKeyHash,
    dfactors_hs: &DecryptionFactorsHashes,
    ciphertexts_h: &CiphertextsHash,
    mix_signer: &TrusteePosition,
    ts: &TrusteeSet,
    threshold: &TrusteeCount,
    trustee: &Trustee<C, S>,
) -> Result<Vec<Message<C>>, ProtocolError>
where
    Trustee<C, S>: b4::messages::message::Signer<C>,
{
    let cfg = trustee.get_configuration(cfg_h)?;
    let num_trustees = cfg.trustees.len();
    
    crate::dispatch_threshold_trustees!(*threshold, num_trustees, {
        crate::dispatch_ciphertext_width!(cfg.ciphertext_width, {
            let plaintexts = compute_plaintexts_::<C, S, W, T, P>(
                cfg_h,
                batch,
                pk_h,
                dfactors_hs,
                ciphertexts_h,
                mix_signer,
                ts,
                threshold,
                trustee,
            )?;
            let m = Message::plaintexts_msg(
                cfg,
                *batch,
                plaintexts,
                *dfactors_hs,
                *ciphertexts_h,
                *pk_h,
                trustee,
            )?;

            Ok(vec![m])
        })
    })
}

/// Verifies the plaintexts by re-computing the plaintexts independently.
///
/// Includes verification of decryption proofs. Returns a Message of type
/// PlaintextsSigned signed by this trustee.
pub(super) fn sign_plaintexts<C: Context, S: crate::protocol::board::LocalBoardStorage>(
    cfg_h: &ConfigurationHash,
    batch: &BatchNumber,
    pk_h: &PublicKeyHash,
    plaintexts_h: &PlaintextsHash,
    dfactors_hs: &DecryptionFactorsHashes,
    ciphertexts_h: &CiphertextsHash,
    mix_signer: &TrusteePosition,
    trustees: &TrusteeSet,
    threshold: &TrusteeCount,
    trustee: &Trustee<C, S>,
) -> Result<Vec<Message<C>>, ProtocolError>
where
    Trustee<C, S>: b4::messages::message::Signer<C>,
{
    let cfg = trustee.get_configuration(cfg_h)?;
    let num_trustees = cfg.trustees.len();
    
    info!(
        "SignPlaintexts verifying decryption [{}] => [{}]",
        dbg_hash(&ciphertexts_h.0),
        dbg_hash(&plaintexts_h.0),
    );

    crate::dispatch_threshold_trustees!(*threshold, num_trustees, {
        crate::dispatch_ciphertext_width!(cfg.ciphertext_width, {
            let expected = compute_plaintexts_::<C, S, W, T, P>(
                cfg_h,
                batch,
                pk_h,
                dfactors_hs,
                ciphertexts_h,
                mix_signer,
                trustees,
                threshold,
                trustee,
            )?;
            let actual = trustee
                .get_plaintexts::<W>(plaintexts_h, *batch, trustees[0] - 1)
                .add_context("Signing plaintexts")?;

            if expected.0 == actual.0 {
                info!(
                    "SignPlaintexts verifying decryption [{}] => [{}], ok",
                    dbg_hash(&ciphertexts_h.0),
                    dbg_hash(&plaintexts_h.0),
                );
                let m = Message::plaintexts_signed_msg(
                    cfg,
                    *batch,
                    *plaintexts_h,
                    *dfactors_hs,
                    *ciphertexts_h,
                    *pk_h,
                    trustee,
                )
                .add_context("Signing plaintexts")?;

                Ok(vec![m])
            } else {
                Err(ProtocolError::VerificationError(format!(
                    "Mismatch when comparing plaintexts with retrieved ones"
                )))
            }
        })
    })
}

/// Computes the plaintexts from a threshold number of decryption factors.
///
/// For each ciphertext and trustee, verifies the decryption factors, then
/// combines them into a single divisor using the cryptography library's
/// Recipient::combine function which handles lagrange coefficients and proof verification.
///
/// Returns a Message of type Plaintexts signed by this trustee.
///
/// As described in Cortier et al.; based on Pedersen.
fn compute_plaintexts_<C: Context, S: crate::protocol::board::LocalBoardStorage, const W: usize, const T: usize, const P: usize>(
    cfg_h: &ConfigurationHash,
    batch: &BatchNumber,
    pk_h: &PublicKeyHash,
    dfactors_hs: &DecryptionFactorsHashes,
    ciphertexts_h: &CiphertextsHash,
    mix_signer: &TrusteePosition,
    ts: &TrusteeSet,
    threshold: &TrusteeCount,
    trustee: &Trustee<C, S>,
) -> Result<b4::messages::artifact::Plaintexts<C, W>, ProtocolError> {
    let cfg = trustee.get_configuration(cfg_h)?;
    let pk = trustee
        .get_dkg_public_key(pk_h, 0)
        .add_context("Computing plaintexts")?;

    let mix = trustee
        .get_mix::<W>(ciphertexts_h, *batch, *mix_signer)
        .add_context("Computing plaintexts")?;

    let num_ciphertexts = mix.ciphertexts.len();

    info!(
        "ComputePlaintexts [{}] ({})..",
        dbg_hash(&ciphertexts_h.0),
        num_ciphertexts,
    );

    assert_eq!(
        datalog::hashes_count(&dfactors_hs.0),
        *threshold,
        "Unexpected number of decryption factors"
    );

    // Collect decryption factors for the T trustees
    let mut all_dfactors = Vec::new();
    
    for (t, df_h) in dfactors_hs.0.iter().enumerate() {
        // Threshold is 1-based
        if t < *threshold {
            let dfactors_with_source = trustee
                .get_decryption_factors::<W, P>(&DecryptionFactorsHash(*df_h), *batch, ts[t] - 1)
                .add_context("Computing plaintexts")?;

            assert_eq!(num_ciphertexts, dfactors_with_source.factors.len());
            
            all_dfactors.push(dfactors_with_source);
        } else {
            debug!("Processed all decryption factors (t = {})", t);
            break;
        }
    }

    info!(
        "ComputePlaintexts combining decryption factors[{}] ({})..",
        dbg_hash(&ciphertexts_h.0),
        num_ciphertexts,
    );

    let suffix = format!("decryption proof");
    let label = cfg.label(*batch, suffix);
    
    use cryptography::dkgd::recipient::{combine, DkgCiphertext};
    
    // Wrap plain Ciphertexts into DkgCiphertext<C, W, T> for combine function
    let wrapped_ciphertexts: Vec<DkgCiphertext<C, W, T>> = mix.ciphertexts
        .iter()
        .map(|c| DkgCiphertext(c.clone()))
        .collect();
    
    // Extract verification keys from public key - indexed by source position
    let verification_keys_vec: Vec<C::Element> = all_dfactors
        .iter()
        .map(|df| pk.verification_keys[df.source.0 as usize - 1].clone())
        .collect();
    
    // Convert Vec to fixed-size arrays
    let dfactors_array: [cryptography::dkgd::recipient::DecryptionFactors<C, W, P>; T] = 
        all_dfactors.try_into()
            .map_err(|_| ProtocolError::InternalError("Failed to convert decryption factors to array".to_string()))?;
    let vkeys_array: [C::Element; T] = 
        verification_keys_vec.try_into()
            .map_err(|_| ProtocolError::InternalError("Failed to convert verification keys to array".to_string()))?;
    
    let plaintexts = combine(
        &wrapped_ciphertexts,
        &dfactors_array,
        &vkeys_array,
        &label,
    ).map_err(|e| ProtocolError::VerificationError(format!(
        "Failed to combine decryption factors: {:?}", e
    )))?;

    Ok(Plaintexts(plaintexts))
}
