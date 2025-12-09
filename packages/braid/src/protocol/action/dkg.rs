#![allow(clippy::too_many_arguments)]

// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use crate::protocol::datalog;
use anyhow::Result;
use b3::messages::artifact::Channel;
use strand::elgamal::PublicKey;
use strand::zkp::Zkp;

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
pub(super) fn gen_channel<C: Ctx>(
    configuration_hash: &ConfigurationHash,
    trustee: &Trustee<C>,
) -> Result<Vec<Message>, ProtocolError> {
    let ctx: C = Default::default();

    let cfg = trustee.get_configuration(configuration_hash)?;

    // Generate a keypair for share transport
    let sk = strand::elgamal::PrivateKey::gen(&ctx);
    let label = cfg.label(0, format!("channel pk proof"));
    let (pk, proof) = sk.get_pk_and_proof(&label)?;

    let ed = trustee.encrypt_share_sk(&sk, &cfg)?;
    let channel = Channel::new(pk.element().clone(), proof, ed);

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
pub(super) fn sign_channels<C: Ctx>(
    configuration_h: &ConfigurationHash,
    channels_hs: &ChannelsHashes,
    self_pos: &TrusteePosition,
    num_trustees: &TrusteeCount,
    trustee: &Trustee<C>,
) -> Result<Vec<Message>, ProtocolError> {
    let ctx: C = Default::default();
    let cfg = trustee.get_configuration(configuration_h)?;
    let zkp = Zkp::new(&ctx);
    let label = cfg.label(0, format!("channel pk proof"));

    assert_eq!(
        datalog::hashes_count(&channels_hs.0),
        *num_trustees,
        "Unexpected number of channels"
    );

    for (i, h) in channels_hs
        .0
        .iter()
        .filter(|h| **h != NULL_HASH)
        .enumerate()
    {
        let hash = *h;
        let channel = trustee.get_channel(&ChannelHash(hash), i)?;
        let pk_element = channel.channel_pk.clone();
        let ok = zkp.schnorr_verify(&pk_element, None, &channel.pk_proof, &label);
        if !ok {
            return Err(ProtocolError::VerificationError(format!(
                "Failed to verify schnorr proof on channel"
            )));
        }

        // Check that our own Channel is at the correct posistion and decrypts correctly
        if i == *self_pos {
            let sk = trustee.decrypt_share_sk(&channel, cfg)?;
            if *sk.pk_element() != pk_element {
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
pub(super) fn compute_shares<C: Ctx>(
    configuration_h: &ConfigurationHash,
    channels_hs: &ChannelsHashes,
    num_trustees: &TrusteeCount,
    threshold: &TrusteeCount,
    trustee: &Trustee<C>,
) -> Result<Vec<Message>, ProtocolError> {
    let ctx = C::default();
    let cfg = trustee.get_configuration(configuration_h)?;

    let (coeffs, commitments) = strand::threshold::gen_coefficients(*threshold, &ctx);

    let mut s = vec![];

    for i in 0..*num_trustees {
        let share = strand::threshold::eval_poly(i + 1, *threshold, &coeffs, &ctx);

        // Obtain the public key for the recipient of the share
        let target_channel_h = channels_hs.0.get(i).ok_or(ProtocolError::InternalError(
            "Could not retrieve channel hash".to_string(),
        ))?;

        let target_hash = *target_channel_h;

        let target_channel = trustee.get_channel(&ChannelHash(target_hash), i)?;

        // Encrypt share for target trustee
        let encryption_pk = PublicKey::<C>::from_element(&target_channel.channel_pk, &ctx);

        let share_bytes = ctx.encrypt_exp(&share, encryption_pk);

        s.push(share_bytes?)
    }

    let shares = Shares {
        commitments: commitments,
        encrypted_shares: s,
    };
    let m = Message::shares_msg(cfg, &shares, trustee)?;
    Ok(vec![m])
}

/// Computes the public key corresponding to the shares.
///
/// Includes verifying this trustee's shares.
///
/// Returns a Message of type PublicKey signed by
/// this trustee.
pub(super) fn compute_pk<C: Ctx>(
    cfg_h: &ConfigurationHash,
    shares_hs: &SharesHashes,
    channels_hs: &ChannelsHashes,
    self_pos: &TrusteePosition,
    num_t: &TrusteeCount,
    threshold: &TrusteeCount,
    trustee: &Trustee<C>,
) -> Result<Vec<Message>, ProtocolError> {
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
pub(super) fn sign_pk<C: Ctx>(
    cfg_h: &ConfigurationHash,
    pk_h: &PublicKeyHash,
    shares_hs: &SharesHashes,
    channels_hs: &ChannelsHashes,
    self_pos: &TrusteePosition,
    num_t: &TrusteeCount,
    threshold: &TrusteeCount,
    trustee: &Trustee<C>,
) -> Result<Vec<Message>, ProtocolError> {
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
fn compute_pk_<C: Ctx>(
    cfg_h: &ConfigurationHash,
    shares_hs: &SharesHashes,
    channels_hs: &ChannelsHashes,
    self_p: &TrusteePosition,
    num_t: &TrusteeCount,
    threshold: &TrusteeCount,
    trustee: &Trustee<C>,
) -> Result<(C::E, Vec<C::E>), ProtocolError> {
    let ctx = C::default();
    let cfg = trustee.get_configuration(cfg_h)?;
    let mut pk = C::E::mul_identity();
    let mut verification_keys = vec![C::E::mul_identity(); *num_t];

    // Iterate over sender shares
    for (i, _h) in shares_hs.0.iter().filter(|h| **h != NULL_HASH).enumerate() {
        let share_h = shares_hs.0[i];
        let share = trustee.get_shares(&SharesHash(share_h), i)?;

        pk = pk.mul(&share.commitments[0]).modp(&ctx);

        // Iterate over receiver trustees to compute their verification key
        for (j, vk) in verification_keys.iter_mut().enumerate().take(*num_t) {
            let vkf =
                strand::threshold::verification_key_factor(&share.commitments, *threshold, j, &ctx);

            *vk = vk.mul(&vkf).modp(&ctx);

            // Our share is sent from trustee i to j, when j = us
            if j == *self_p {
                // Construct our private key to decrypt our share
                let my_channel_h =
                    channels_hs
                        .0
                        .get(*self_p)
                        .ok_or(ProtocolError::InternalError(
                            "Could not retrieve channel hash for self".to_string(),
                        ))?;

                let my_channel = trustee
                    .get_channel(&ChannelHash(*my_channel_h), *self_p)
                    .add_context("Retrieving channel for self")?;

                let sk = trustee.decrypt_share_sk(&my_channel, &cfg)?;

                // Decrypt the share sent from i to us
                let value = ctx.decrypt_exp(&share.encrypted_shares[*self_p], sk)?;
                // Verify the share
                let ok = strand::threshold::verify_share(&value, &vkf, &ctx);
                if !ok {
                    return Err(ProtocolError::VerificationError(format!(
                        "Trustee {} failed to verify share from {}..",
                        j, i
                    )));
                }
                trace!("Trustee {} verified share received from {}", j, i);
            }
        }
    }

    Ok((pk, verification_keys))
}
