#![allow(clippy::too_many_arguments)]

// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use crate::protocol::datalog;
use anyhow::Result;
use b4::messages::artifact::Channel;
use b4::messages::newtypes::zero_hash;
use cryptography::debug_log;
use cryptography::traits::groups::CryptographicGroup;

/// Generates a private communication channel for this trustee.
///
/// Used to send shares privately to this trustee. A trustee will
/// receive a share from each of its peers. These shares are elgamal
/// encrypted with the Channel's public key. The corresponding
/// private key is symmetrically encrypted with an private symmetric
/// key belonging to the trustee, and is also part of the Channel data.
/// This allows restoring all information from the bulletin board, as well
/// as securely downloading a Channel by its trustee during the key
/// ceremony.
///
/// Channels include a schnorr proof for knowledge of the secret
/// key corresponding to the public key.
///
/// Returns a Message of type Channel signed by this trustee.
pub(super) fn gen_channel<C: Context, S: crate::protocol::board::LocalBoardStorage>(
    configuration_h: &ConfigurationHash,
    trustee: &Trustee<C, S>,
) -> Result<Vec<Message<C>>, ProtocolError> {
    let cfg = trustee.get_configuration(configuration_h)?;

    // Generate a keypair for share transport
    let label = cfg.label(0, format!("channel pk proof"));
    let (keypair, proof) = b4::gen_elgamal_keypair_with_proof::<C>(&label)
        .map_err(|e| ProtocolError::InternalError(e))?;

    let ed = trustee.encrypt_share_sk(&keypair, &cfg)?;
    let channel = Channel::new(keypair.pkey.y.clone(), proof, ed);

    let m = Message::channel_msg(cfg, &channel, true, trustee)?;
    Ok(vec![m])
}

/// Verifies all the posted Channels.
///
/// Channel verification checks schnorr proofs for the
/// public keys. Additionally, each trustee self verifies
/// their own Channel's private key by decrypting it.
///
/// Returns a Message of type ChannelsAllSigned signed by this trustee.
pub(super) fn sign_channels<C: Context, S: crate::protocol::board::LocalBoardStorage>(
    configuration_h: &ConfigurationHash,
    channels_hs: &ChannelsHashes,
    self_pos: &TrusteePosition,
    num_trustees: &TrusteeCount,
    trustee: &Trustee<C, S>,
) -> Result<Vec<Message<C>>, ProtocolError> {
    let cfg = trustee.get_configuration(configuration_h)?;
    let g = C::generator();
    let label = cfg.label(0, format!("channel pk proof"));

    assert_eq!(
        datalog::hashes_count(&channels_hs.0),
        *num_trustees,
        "Unexpected number of channels"
    );

    for (i, h) in channels_hs
        .0
        .iter()
        .filter(|h| **h != zero_hash())
        .enumerate()
    {
        let hash = *h;
        let channel = trustee.get_channel(&ChannelHash(hash), i)?;
        let pk_element = channel.channel_pk.clone();
        let ok = channel.pk_proof.verify(&g, &pk_element, &label)
            .map_err(|e| ProtocolError::VerificationError(format!("Schnorr verify error: {:?}", e)))?;
        if !ok {
            return Err(ProtocolError::VerificationError(format!(
                "Failed to verify schnorr proof on channel"
            )));
        }

        // Check that our own Channel is at the correct posistion and decrypts correctly
        if i == *self_pos {
            let keypair = trustee.decrypt_share_sk(&channel, cfg)?;
            if keypair.pkey.y != pk_element {
                return Err(ProtocolError::VerificationError(format!(
                    "Failed to decrypt self channel"
                )));
            }
        }
    }

    let m = Message::channels_all_signed_msg(cfg, channels_hs, trustee)?;
    Ok(vec![m])
}

/// Computes the shares for all trustees.
///
/// Each trustee computes a share and commitments for all trustees
/// including itself. These shares are encrypted with the recipient's public
/// key as present in their Channel. Shares are verified by their
/// recipient trustees as part of their public key verification.
///
/// Returns a Message of type Shares signed by this trustee.
///
/// As described in Cortier et al.; based on Pedersen.
pub(super) fn compute_shares<C: Context, S: crate::protocol::board::LocalBoardStorage>(
    configuration_h: &ConfigurationHash,
    channels_hs: &ChannelsHashes,
    num_trustees: &TrusteeCount,
    threshold: &TrusteeCount,
    trustee: &Trustee<C, S>,
) -> Result<Vec<Message<C>>, ProtocolError> {
    use cryptography::dkgd::dealer::Dealer;
    
    let cfg = trustee.get_configuration(configuration_h)?;

    // Use dispatch macro to convert runtime values to const generics
    crate::dispatch_threshold_trustees!(*threshold, *num_trustees, {
        // Generate dealer with shares and checking values
        let dealer = Dealer::<C, T, P>::generate();
        let dealer_shares = dealer.get_verifiable_shares();
        
        let mut encrypted_shares = vec![];

        for i in 0..*num_trustees {
            let share = dealer_shares.shares[i].clone();

            // Obtain the public key for the recipient of the share
            let target_channel_h = channels_hs.0.get(i).ok_or(ProtocolError::InternalError(
                "Could not retrieve channel hash".to_string(),
            ))?;

            let target_hash = *target_channel_h;
            let target_channel = trustee.get_channel(&ChannelHash(target_hash), i)?;

            // Encrypt share for target trustee using scalar encryption
            let share_bytes = C::G::encrypt_scalar(&share, &target_channel.channel_pk)
                .map_err(|e| ProtocolError::InternalError(format!("Failed to encrypt share: {:?}", e)))?;

            encrypted_shares.push(share_bytes);
        }

        let shares = Shares {
            commitments: dealer_shares.checking_values.to_vec(),
            encrypted_shares,
        };
        let m = Message::shares_msg(cfg, &shares, trustee)?;
        Ok(vec![m])
    })
}

/// Computes the public key corresponding to the shares.
///
/// Includes verifying this trustee's shares.
///
/// Returns a Message of type PublicKey signed by
/// this trustee.
pub(super) fn compute_pk<C: Context, S: crate::protocol::board::LocalBoardStorage>(
    cfg_h: &ConfigurationHash,
    shares_hs: &SharesHashes,
    channels_hs: &ChannelsHashes,
    self_pos: &TrusteePosition,
    num_t: &TrusteeCount,
    threshold: &TrusteeCount,
    trustee: &Trustee<C, S>,
) -> Result<Vec<Message<C>>, ProtocolError> {
    let cfg = trustee.get_configuration(cfg_h)?;
    let pk = compute_pk_(
        cfg_h,
        shares_hs,
        channels_hs,
        self_pos,
        num_t,
        threshold,
        trustee,
    )
    .add_context("Computing pk")?;

    let public_key: DkgPublicKey<C> = DkgPublicKey::new(pk.0, pk.1);

    let m = Message::public_key_msg(cfg, &public_key, shares_hs, channels_hs, true, trustee)?;
    Ok(vec![m])
}

/// Verifies the public key re-computing it independently.
///
/// Includes verifying this trustee's shares.
///
/// Returns a Message of type PublicKeySigned signed by
/// this trustee.
pub(super) fn sign_pk<C: Context, S: crate::protocol::board::LocalBoardStorage>(
    cfg_h: &ConfigurationHash,
    pk_h: &PublicKeyHash,
    shares_hs: &SharesHashes,
    channels_hs: &ChannelsHashes,
    self_pos: &TrusteePosition,
    num_t: &TrusteeCount,
    threshold: &TrusteeCount,
    trustee: &Trustee<C, S>,
) -> Result<Vec<Message<C>>, ProtocolError> {
    let cfg = trustee.get_configuration(cfg_h)?;
    info!(
        "SignPk verifying public key [{}] ({})..",
        dbg_hash(&pk_h.0),
        num_t,
    );

    let expected = compute_pk_(
        cfg_h,
        shares_hs,
        channels_hs,
        self_pos,
        num_t,
        threshold,
        trustee,
    )?;

    let actual = trustee
        .get_dkg_public_key(pk_h, 0)
        .add_context("Signing pk")?;

    if (expected.0 == actual.pk) && (expected.1 == actual.verification_keys) {
        info!(
            "SignPk verifying public key [{}] ({}), ok",
            dbg_hash(&pk_h.0),
            num_t,
        );
        let m = Message::public_key_msg(cfg, &actual, shares_hs, channels_hs, false, trustee)?;
        Ok(vec![m])
    } else {
        Err(ProtocolError::VerificationError(format!(
            "Mismatch when comparing computed public key with retrieved one"
        )))
    }
}

/// Computes the public key from the shares.
///
/// First this trustee's Channel is retrieved, and the private
/// key is decrypted. This key is then used to decrypts the shares
/// sent to this trustee, which are verified using the commitments.
/// The share commitments are then used to compute the public key as
/// well as the all trustee's verification keys (used to verify
/// decryptions).
///
/// Returns the public key and the verification keys for
/// all trustees.
///
/// As described in Cortier et al.; based on Pedersen.
fn compute_pk_<C: Context, S: crate::protocol::board::LocalBoardStorage>(
    cfg_h: &ConfigurationHash,
    shares_hs: &SharesHashes,
    channels_hs: &ChannelsHashes,
    self_pos: &TrusteePosition,
    num_t: &TrusteeCount,
    threshold: &TrusteeCount,
    trustee: &Trustee<C, S>,
) -> Result<(C::Element, Vec<C::Element>), ProtocolError> {
    use cryptography::dkgd::recipient::{Recipient, ParticipantPosition};
    use cryptography::dkgd::dealer::VerifiableShare;
    
    let cfg = trustee.get_configuration(cfg_h)?;

    // Get our channel to decrypt our shares
    let my_channel_h = channels_hs.0.get(*self_pos)
        .ok_or(ProtocolError::InternalError(
            "Could not retrieve channel hash for self".to_string(),
        ))?;
    let my_channel = trustee.get_channel(&ChannelHash(*my_channel_h), *self_pos)
        .add_context("Retrieving channel for self")?;
    let my_sk = trustee.decrypt_share_sk(&my_channel, &cfg)?;

    // Use dispatch macro to work with const generics
    crate::dispatch_threshold_trustees!(*threshold, *num_t, {
        // Collect all shares sent to us from all dealers
        let mut verifiable_shares = Vec::new();

        // Collect all checking values from all dealers
        let mut all_checking_values = Vec::new();
        
        for (i, _h) in shares_hs.0.iter().filter(|h| **h != zero_hash()).enumerate() {
            let share_h = shares_hs.0[i];
            let share_msg = trustee.get_shares(&SharesHash(share_h), i)?;

            // Decrypt our share from this dealer
            let encrypted_share = &share_msg.encrypted_shares[*self_pos];
            let share_scalar = C::G::decrypt_scalar(encrypted_share, &my_sk.skey)
                .map_err(|e| ProtocolError::InternalError(format!("Failed to decrypt share: {:?}", e)))?;

            // Convert commitments Vec to array [C::Element; T]
            let checking_values: [C::Element; T] = share_msg.commitments.clone()
                .try_into()
                .map_err(|_| ProtocolError::InternalError(
                    format!("Expected {} commitments, got {}", T, share_msg.commitments.len())
                ))?;

            let verifiable_share = VerifiableShare::new(share_scalar, checking_values.clone());
            verifiable_shares.push(verifiable_share);

            all_checking_values.push(checking_values);
        }

        let all_cvs: [[C::Element; T]; P] = all_checking_values.try_into()
        .map_err(|v: Vec<_>| ProtocolError::InternalError(
            format!("Expected {} checking value sets, got {}", P, v.len())
        ))?;

        // Convert Vec to array [VerifiableShare; P]
        let shares_array: [VerifiableShare<C, T>; P] = verifiable_shares.try_into()
            .map_err(|v: Vec<_>| ProtocolError::InternalError(
                format!("Expected {} shares, got {}", P, v.len())
            ))?;

        // Create position (1-indexed)
        let position = ParticipantPosition::from_usize(*self_pos + 1);

        // Verify all shares and compute joint public key
        let (joint_pk, _verification_key, _sk) = Recipient::<C, T, P>::verify_shares(&position, &shares_array)
            .map_err(|e| ProtocolError::VerificationError(format!("Share verification failed: {:?}", e)))?;

        // Compute verification keys for all trustees
        let mut verification_keys = Vec::new();
        for j in 0..*num_t {
            let pos_j = ParticipantPosition::from_usize(j + 1);
            
            let vk = Recipient::<C, T, P>::verification_key(&pos_j, &all_cvs);
            verification_keys.push(vk);
        }

        trace!("Trustee {} verified all {} shares", self_pos, P);
        Ok((joint_pk, verification_keys))
    })
}

// Verifier mode functions //////////////////////////////////

/// Verifies the public key re-computing it independently.
/// 
/// This function is only used in verification mode.
///
/// Returns an empty vector on success.
pub(super) fn verify_pk<C: Context, S: crate::protocol::board::LocalBoardStorage>(
    cfg_h: &ConfigurationHash,
    pk_h: &PublicKeyHash,
    shares_hs: &SharesHashes,
    channels_hs: &ChannelsHashes,
    num_t: &TrusteeCount,
    threshold: &TrusteeCount,
    trustee: &Trustee<C, S>,
) -> Result<Vec<Message<C>>, ProtocolError> {
    debug_log!(
        "VerifyPk(Verifier) verifying public key [{}] ({})..",
        dbg_hash(&pk_h.0),
        num_t,
    );
    let cfg = trustee.get_configuration(cfg_h)?;
    let expected = verify_pk_(
        shares_hs,
        num_t,
        threshold,
        trustee,
    )?;

    let actual = trustee
        .get_dkg_public_key(pk_h, 0)
        .add_context("Signing pk")?;

    if (expected.0 == actual.pk) && (expected.1 == actual.verification_keys) {
        debug_log!(
            "VerifyPk(Verifier) verifying public key [{}] ({}), ok",
            dbg_hash(&pk_h.0),
            num_t,
        );
        let m = Message::public_key_msg(cfg, &actual, shares_hs, channels_hs, false, trustee)?;
        Ok(vec![m])
    } else {
        debug_log!(
            "VerifyPk(Verifier) verifying public key [{:?}] ({:?}), failed",
            expected.0,
            actual.pk,
        );
        Err(ProtocolError::VerificationError(format!(
            "Mismatch when comparing computed public key with retrieved one"
        )))
    }
}

/// Computes the public key from the shares.
/// 
/// This function is only used in verification mode.
///
/// The share commitments are used to compute the public key as
/// well as the all trustee's verification keys (used to verify
/// decryptions).
///
/// Returns the public key and the verification keys for
/// all trustees.
fn verify_pk_<C: Context, S: crate::protocol::board::LocalBoardStorage>(
    shares_hs: &SharesHashes,
    num_t: &TrusteeCount,
    threshold: &TrusteeCount,
    trustee: &Trustee<C, S>,
) -> Result<(C::Element, Vec<C::Element>), ProtocolError> {
    use cryptography::dkgd::recipient::{Recipient, ParticipantPosition};
    
    // Use dispatch macro to work with const generics
    crate::dispatch_threshold_trustees!(*threshold, *num_t, {

        // Collect all checking values from all dealers
        let mut all_checking_values = Vec::new();
        for (i, _h) in shares_hs.0.iter().filter(|h| **h != zero_hash()).enumerate() {
            let share_h = shares_hs.0[i];
            let share_msg = trustee.get_shares(&SharesHash(share_h), i)?;
            
            let checking_values: [C::Element; T] = share_msg.commitments.clone()
                .try_into()
                .map_err(|_| ProtocolError::InternalError(
                    format!("Expected {} commitments, got {}", T, share_msg.commitments.len())
                ))?;
            
            all_checking_values.push(checking_values);
        }
        
        let all_cvs: [[C::Element; T]; P] = all_checking_values.try_into()
            .map_err(|v: Vec<_>| ProtocolError::InternalError(
                format!("Expected {} checking value sets, got {}", P, v.len())
            ))?;

        let joint_pk = Recipient::<C, T, P>::joint_public_key(&all_cvs);

        // Compute verification keys for all trustees
        let mut verification_keys = Vec::new();
        for j in 0..*num_t {
            let pos_j = ParticipantPosition::from_usize(j + 1);
            
            let vk = Recipient::<C, T, P>::verification_key(&pos_j, &all_cvs);
            verification_keys.push(vk);
        }

        Ok((joint_pk.inner.y, verification_keys))
    })
}
